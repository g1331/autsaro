---
title: 'Autsaro 软件多语言支持'
type: 'feature'
created: '2026-10-07'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
context: []
baseline_commit: '96095c9a6ade6550a800818aa7c08ac60dd1ed2c'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 软件需要支持多语言。现有界面文本主要硬编码为中文，后端也直接返回部分中文消息，尚无统一语言选择与翻译机制。

**Approach:** 支持简体中文和英文，覆盖完整产品文案，包括界面、操作反馈、软件自身的后端错误、校验说明与修复建议。采用统一语言资源、呈现设置与结构化产品消息；外部工具日志保持原文。

## Boundaries & Constraints

**Always:** 保持工程编辑、保存和生成语义不变；语言偏好属于应用呈现设置，不进入 ARXML、工程指纹或生成内容。保留 AUTOSAR 名称、参数名、错误码、用户对象名、文件路径及外部工具原始日志。沿用现有设置保存失败保留旧值的契约。中文 README 保留，新增英文 README 并提供双向入口。

**用户确认：** 2026-10-07 确认中英文、完整产品文案；随后批准完整实现、独立功能分支、创建并合并 PR，且要求英文 README。生成包说明不随界面语言变化。

**Never:** 不以 DOM 文本替换、运行时机器翻译或中文句子匹配模拟国际化；不修改生成代码、规则身份、第三方内容或本机全局语言设置。未经批准不发布或推送。

## I/O & Edge-Case Matrix

| 场景 | 输入／状态 | 预期行为 | 错误处理 |
| --- | --- | --- | --- |
| 切换语言 | 工程打开且存在字段草稿 | 呈现更新；工程、草稿和已归属结果保留 | 不使生成或校验结果失效 |
| 保存失败 | 设置文件被外部修改或写入失败 | 已保存语言及原文件保持 | 明确显示失败，不冒充已保存 |
| 数据与原文 | 对象名、源代码、工具日志、路径 | 保持输入原文 | 不把未知原文误判为翻译键 |

</frozen-after-approval>

## Code Map

- `ui/src/{main,App}.tsx`、`pages/`、`workbench/*.tsx`、`Integration{Panel,Delivery}.tsx`：界面、帮助、无障碍标签、弹窗及产品提供的文件选择标题。
- `ui/src/workbench/{state,session,useWorkbench,forms,useDelivery,operation,projectActions,deliveryActions,settingsActions,batchEditing,legacyEditing}.ts`：消息生产与状态；提示、阶段说明、错误与操作标签不能存储预翻译字符串。
- `ui/src/{types.ts,workbench/projectTypes.ts}`：诊断、能力原因、定义说明及 IPC 类型。
- `src-tauri/src/`：全部返回通道，包括字符串错误、诊断数组、`SaveOutcome.error`；语言保存复用 `configure_appearance` 的无工程修订模式，不走会使结果失效的通用 `configure`。
- `core/src/{project_model,model,arxml,rules,definitions}.rs`、`definitions/`、`arxml/`、`integration/`、`generator/`、`host/`、`{target,resources,schema,prepared,host}.rs`、`execution/`：产品错误、诊断、约束与执行外围状态。原始工具输出与系统错误证据独立保留。
- `src-tauri/tauri{,.windows}.conf.json`：原生标题；MSI 提供中英文安装界面，保留既有产品身份。

## Tasks & Acceptance

**Execution:**
- [x] `ui/package{,-lock}.json`、`ui/src/i18n/`、`ui/src/main.tsx` -- 接入 `i18next/react-i18next`，完整资源、稳定键、插值、复数及缺项检查。
- [x] `core/src/`、`src-tauri/src/`、`ui/src/{types.ts,workbench/}` -- 迁移上述全部产品消息为消息标识＋参数，更新全部调用方；保留分类、定位与证据，不留原句匹配兼容层。
- [x] `src-tauri/src/{workbench,configuration,lib}.rs`、`ui/src/pages/SettingsPage.tsx`、`ui/src/workbench/{state,useWorkbench,settingsActions}.ts` -- 增加 `system | zh-CN | en` 偏好，中文系统选中文，否则英文；预览／保存／取消与主题一致，旧设置默认跟随系统，失败保留旧值；浏览器也能切换。
- [x] `ui/src/`、`src-tauri/tauri{,.windows}.conf.json` -- 迁移全部界面与原生标题；MSI 中英文安装资源独立于应用偏好。系统提供的对话框控件由系统语言决定。
- [x] `ui/tests/`、`core/tests/`、`src-tauri/src/`、`tests/desktop/` -- 覆盖状态保持、旧设置、失败、插值，完成中英文真实工作流验收。
- [x] `README.md`、`README.en.md` -- 增加完整英文版本、双向语言入口及语言使用说明。

**Acceptance Criteria:**
- Given 中／英文，when 操作打开、编辑、校验、保存、生成、构建与运行及失败路径，then 全部产品文案与无障碍名称使用当前语言。
- Given 旧诊断和草稿，when 切换／取消语言预览，then 消息更新而草稿、指纹、预览及结果归属不变。
- Given 已保存或无语言字段的旧设置，when 重启，then 恢复选择或跟随系统；失败不覆盖旧文件。
- Given 英文窄窗口与深浅主题，when 使用菜单／弹窗，then 入口、焦点与滚动正常。
- Given 原始证据或生成／复制机器数据，when 切换语言，then 字节与机器标识不变；缺翻译不静默回退掩盖。

## Design Notes

桌面使用应用设置，浏览器使用本地偏好。消息渲染时翻译，反例与外部错误只插值；资源离线内置。

## Implementation Notes

### 实施接口与文件归属

- 后端切片负责 `core/`、`src-tauri/`、`ui/src/i18n/backend.ts`；前端切片负责其余 `ui/`；集成负责人负责规格、README、最终检查与 PR。切片中途不运行 build/lint/tests/formatters，最终统一验证。
- IPC 产品消息统一采用 `{ key: string, params: Record<string, string | number | boolean | null> }`；原始外部证据仍为字符串，聚合消息为递归数组。前端 `LocalizedText` 保持同一契约；不能将产品文案作为原始字符串绕过迁移。诊断 `message/remedy`、能力／定义／引用 `reason`、产品约束说明、保存错误及普通 IPC 失败均采用此消息契约；机器标识不变。
- 后端切片维护 `backend.ts`，导出 `backendResources = { 'zh-CN': Record<string,string>, en: Record<string,string> }`，使用 `backend.*` 稳定键，不写其他前端资源。消息参数与 i18next 插值一致；迁移显式生产点，不运行时识别原句。库内纯机器／原始工具错误维持证据，不局部吞错。
- 桌面能力增加 `language: 'system' | 'zh-CN' | 'en'`；提交命令 `configure_language` 接受 `{ fingerprint, language }`，返回既有 `WorkbenchReply<void>`，持久化而不增加工程修订、不终止操作。前端同步类型及控制器，语言预览不重建工程状态。
- 后端导出的消息值统一让前端渲染器按当前语言解析；前端自有提示同样存消息键／参数，不提前翻译后保存。未知外部文本保持原文；缺少产品键不能静默转原句。协作时通过消息确认具体类型变更。
- 英文 README 保持与中文功能、命令、限制和链接一致；不翻译标识符、不伪称英文版关联技术文档也已翻译。保留品牌、图标和项目范围，不顺手改品牌。
- 具体消息契约补充：聚合失败／恢复说明可为递归 `LocalizedText[]`，各产品叶节点仍为消息键与标量参数；前端嵌套提示的 `values` 组合只在前端使用，不进入 IPC。
- 后端语言资源采用 `core/src/messages.json` 单一来源，Rust 与 `ui/src/i18n/backend.ts` 共用；Rust 使用正常 Cargo 构建时生成的静态模板查找，不在运行时解析整份资源，也不保留会话专属生成器。


## Spec Change Log

- 2026-10-07：按用户批准实施中英文完整产品文案、独立功能分支与英文 README。未改变工程格式、生成字节、ABI 或资源身份。

## Review Triage Log

| 来源／发现 | 分级 | 处理 | 验证 |
| --- | --- | --- | --- |
| Blind review：递归消息数组被当作集成诊断，导致不存在的分类渲染失败 | 高 | 以结构校验区分诊断数组与消息聚合，保留全部叶节点和原始证据 | 实际工作台消费者回归通过 |
| Blind review：语言保存等待期间关闭设置，预览回滚与成功提交竞争 | 中 | 独立保存状态；等待期间禁用切换、保存和关闭，不占用工程操作状态 | 延迟提交／Escape／重复提交回归通过 |
| Edge-case review：同一语言保存关闭竞态 | 中 | 合并到同一根因修复；未另加回退路径 | 同一延迟提交回归通过 |
| Edge-case review：通用配置验证期间克隆的旧设置覆盖后提交的语言／主题 | 中 | 在现有持久化锁内读取最新呈现设置，验证仍在锁外 | 确定性屏障回归修改前失败、修改后通过；桌面后端 5 项通过 |
| Verification review：原文断言移除后，已提交与部分失败反馈的行为区别缺少保护 | 中 | 按结果与原始失败证据验证工作台反馈，不固定句子措辞 | 中英文消费者回归通过 |
| Verification review：实际 cardinality 诊断的上下界／数量插值缺少消费者证据 | 中 | 保留实际 Rust 生产器与问题面板消费者的边界回归 | 核心 86 项、前端 34 项通过 |

- 验证中修正局部格式化产生的导入移动及重复片段；按暂存源码核对受影响 Rust 标识符／字面值，恢复丢失导入及唯一重复片段。默认核心／桌面测试、CLI 构建和变更行格式检查通过。
- 原生／oracle 特性编译发现遗留中文句子断言；移除措辞依赖，保留拒绝操作、文件不变、协议原始字节与旧预览不得提交的行为断言。

## Verification

- `npm run lint --prefix ui`、`npm run test --prefix ui`、`npm run build --prefix ui`。
- `cargo test --locked --manifest-path core/Cargo.toml`、`cargo test --locked --manifest-path src-tauri/Cargo.toml`。
- 正常 Vite 入口验证预览；Windows Sandbox 中使用 MSI 提取的成品验证原生语言预览、取消、保存及重启恢复，沙箱禁用网络，不占用用户工程或应用设置。
- `uv run --locked python -m autosar_tooling assets check`；不以摘要更新吸收 ABI／第三方身份变化。
- 已执行：前端 lint、34 项测试和生产构建；核心默认 86 项、桌面后端 5 项测试及核心 CLI 构建；核心扩展特性执行 25 项库测试、71 项内置案例、42 项端到端案例与 77 项原生案例。扩展全量运行最后一项因措辞断言失败，移除该实现细节断言后，实际 DLC 拒绝案例单独重跑通过；合计 215 项核心行为用例已成功执行，并非声称单次全量命令全部通过。
- 正常桌面构建与核心／桌面 Clippy 通过；资源摘要一致且无变化，未改变 C ABI、生成资产或第三方资产身份；129 个文件的变更行格式检查通过。
- 已执行：真实浏览器中英文预览、取消、保存及刷新持久化；1080×720 英文深色设置截图。README 双向入口、命令与本地链接核对通过。
- 已执行：Windows 中英文 MSI 均成功构建；英文安装镜像提取至源码树外，原构建源码目录移走后，成品在禁网 Sandbox 中正常启动。实际鼠标／键盘操作验证英文预览、取消恢复中文、保存英文与重启恢复英文，原生标题同步切换；已查看原生中英文设置及重启画面。未将沙箱临时验收脚本或工具改动加入仓库。
- 平台边界：本轮未运行 macOS／Linux 原生 GUI；Windows Sandbox 验收不冒充现有独立 WindowStation/WFP 隔离验收全套通过。
- 构建仍有 Vite 单包超过 500 KB 的提示；双语资源离线内置，未关闭告警或改变分包策略。
