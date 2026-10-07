---
title: '常规开发与可维护性整理'
type: 'refactor'
created: '2026-10-07'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'ed1344ac8424241bec25cfa049e5ee12c8fbfcc1'
context: []
---

<frozen-after-approval reason="用户已批准完整实施计划并要求实施">

## Intent

将仓库的环境、工具、源码、测试和交付整理为团队能够直接接手的工程。用户本轮提供的完整实施计划是实施和验收契约，本规格承载实施结果，不缩减计划范围。

## Boundaries & Constraints

保留产品行为、IPC、C ABI、封存结构、独立预期及受控编译器。开发 Node 24、npm 11、Python 3.12+，离线工具 Python 3.11+；删除精确 Python 3.12.9 限制。保留标准锁文件和 Rust 工具链。不得引入新编排器、管理体系或干扰用户桌面的 GUI 验收，不修改历史包或伪造旧资源身份。

## I/O & Edge-Case Matrix

| 场景 | 输入 | 结果 | 失败处理 |
|---|---|---|---|
| 普通开发 | 正常终端及支持版本 | 安装、开发、测试、构建入口可运行 | 明确缺件及非零状态 |
| 离线交付 | 搬移的封存包及 Python 3.11—3.14 | stdlib-only 构建和独立验证 | 篡改、额外文件、链接及工具链不符仍拒绝 |
| 异步编辑 | 草稿、取消、过期预览或刷新失败 | 保留原子提交、epoch 与 fingerprint 防护 | 已提交与刷新失败分别报告 |
| 资源维护 | 自有内容变化或上游篡改 | 自有摘要统一更新，ABI独立审查 | 固定上游身份不得自动更新 |

</frozen-after-approval>

## Code Map

- `scripts/`：工具、Python包、测试和技能维护混放；迁至 `tools/python/src/`、`tools/automotive/`、`tests/`。Cargo、字节清单和POSIX提取均引用原位置。
- `pyproject.toml`、`ui/package.json`、版本与隐藏配置：保留原生工具，移除重复精确版本门禁；保留 Impeccable 的自动设计检查和相关配置。
- `ui/src/workbench/useWorkbench.ts`：状态、操作、编辑和交付集中；提示文字控制流程需要改为操作类型。
- `src-tauri/src/lib.rs`、`workbench.rs`：命令分组，但Session、提交门及Reply保持。
- `core/src/arxml.rs`、`core/build.rs`、`generator/delivery/tools.rs`：内部职责、交付身份、可信helper；保持输出协议。
- `runtime/contracts/`：来源摘要不是ABI契约；`third_party/`的身份不变。

## Tasks & Acceptance

- [x] 版本文件、Python入口、生成说明：放宽开发及交付解释器；兼容性覆盖真实行为。
- [x] Python包、测试、技能维护目录：完整迁移，更新所有嵌入、导入、清单及换行路径。
- [x] `autosar_tooling`：删除dev/runner；简化verify、诊断及质量检查，集中配置来源。
- [x] 前端状态和测试：业务动作分组，统一操作控制；Vitest替代手写加载器。
- [x] Tauri和核心：按业务组织命令及旧配置编辑，接口和提交边界不变。
- [x] 资源工具：统一摘要维护并去除helper常量重复，不自动接受ABI变化。
- [x] CI和发布：标准命令、明确基准与产物目录，Python矩阵和隔离验收。
- [x] 文档和交接：读者入口、生成源、模板及版本说明同步；无旧入口和临时遗留。

Given 支持版本和正常工具，当开发者沿文档操作，Then 无需代理或隐藏状态即可开发和验证。
Given 原有拒绝用例，当运行迁移后的测试，Then 预期不变且失败仍传播。
Given 离线包和支持解释器，当构建及验证，Then 不依赖仓库或开发环境。

## Implementation Notes

用户已批准整项范围并要求实施，复用本会话完整调查；不重复请求规格批准。主代理负责设计与实现，复核按安装工作流执行。

已移除 dev/runner 和自定义 TypeScript 加载器；开发通过 npm/Cargo/uv/unittest，组合 verify 顺序传递真实输出与失败。Python 源位于 tools/python/src，普通测试、进程与平台测试位于 tests/python，桌面驱动在 tests/desktop，汽车技能维护在 tools/automotive。源码嵌入、资源清单、POSIX helper 和所有调用同步迁移。

前端采用单一 reducer 与内部 Session，工程、草稿、交付和设置动作分组；页面只获得业务动作及必要表单/视图字段。操作种类不再由中文提示决定。批次修改会使旧预览失效；成功提交后刷新失败仍清除已提交草稿。Tauri 命令分组，核心旧配置编辑移到 legacy_editor；没有改 IPC 名称、载荷、C ABI、内核或目标工具链。

验收发现并修复现有 Windows 目录选择器控件不匹配，以及 v1 同源重开丢失显式历史验证模式。历史场景使用当前支持的 v1 导入入口；原预期和独立 oracle 保留。原生减少动态效果验收保留零过渡断言并等待系统偏好与样式更新。

## Spec Change Log

- 2026-10-07：用户要求恢复 Impeccable Hook 及相关改动。恢复 `.codex/hooks.json`、`.impeccable/config.json` 和 `ui/.impeccable/live/config.json` 的原配置；编辑后即时检查及 Stop 完整检查保持原样。此前把 `buildPath: "code"` 当成失效目录是错误判断，它是有效的构建方式选项。Vite 的 `index.html` 实时预览配置同样恢复；其余工程优化不变。

## Review Triage Log

三路上下文隔离复核及修改后的复核已完成。blind-hunter 没有发现可确认的问题；edge-case-hunter 与 verification-gap 的发现逐项处理如下。

| 来源与位置 | 判定 | 处理及依据 |
|---|---|---|
| edge-case-hunter：projectActions.ts，顺序草稿应用 | medium / patch | 基准中已存在：先成功创建或修改，后续诊断编辑拒绝时仍提示整次“被拒绝”，且旧交付状态未失效。本次已授权的草稿结果整理覆盖此路径，因此直接修复：记录已确认提交，保留剩余草稿，提示部分应用并使下游失效；integration 的提交确认在后续视图读取之前清除其草稿。新增两个真实顺序回归，16 个 Vitest、lint 与 UI 构建通过；未修改后端原子提交契约。 |

| verification-gap：execution/unix.rs，长路径 owner 生命周期 | medium / patch | 原桌面聚焦流程已执行真实 Rust owner，但原生 Rust 套件只单独测试了 socket 地址。新增复用真实 supervisor 的 Linux 生命周期回归，覆盖长路径下子进程运行、输出、目录删除及 fd 释放；12 个 execution 用例与 5 个默认库用例通过。没有增加公共入口或修改认证协议。 |
| edge-case-hunter：desktop_builtin_scenario.mjs，逐条 IPC 日志写入 | low / patch | FileHandle.write 返回写入字节数，单次短写可能截断记录；改用现有 FileHandle.writeFile 完成整条内容后继续，不丢弃或裁剪任何证据。没有引入日志体系。 |
## Verification

- npm 安装、开发服务器 HTTP 200、lint、16 个 Vitest 4.1.11 用例及 UI 构建通过；新测试依赖的已知漏洞已在同一 major 修复，npm 审计无已知漏洞。
- 开发 Python 3.12、3.13、3.14 的 31 个 unittest 通过；Windows 21 个进程用例中 10 个执行、11 个平台不适用；Linux 21 个中 20 个执行、1 个 Windows 用例不适用。
- editable 安装、wheel/sdist 构建、独立 wheel 导入、包内 JSON 与实际 doctor 命令通过。uv 锁保持；汽车技能来源未变，24 技能和原 8 项维护行为验证通过。
- Windows aggregate 完整验证有 76 个 native 用例、62 个基础集成、41 个官方对照及 13 个库用例通过。Linux 受控验证 14 个库用例、64 个基础集成、41 个 native 和 41 个官方用例通过，包含平台 OS suites。
- Windows/Linux 的接收者 Python 3.11、3.12、3.13、3.14 都对搬移后的双主机参考包与 ECU handoff 执行真实构建及验证，封存字节保持不变；接收解释器未安装开发工具。
- 增量质量、Ruff、Tauri 构建及核心/桌面 clippy 通过。工作流经 actionlint 校验；仅写入 CI 配置，未触发远端运行。
- 索引导出的新 checkout 中，独立安装的 wheel 执行 assets check 通过，避免依赖原 editable 路径和 Windows 工作区行尾。
- 最终 Windows 私有桌面历史完整场景通过（desktop-windows-15），进程 Job 退出后 remaining=0、errors=[]；37 个 IPC 命令名称、参数及 Reply 类型与基准一致。
- Linux 默认发行 deb 在原 checkout 不可用、接收者 PATH 无开发 SDK 的私有环境通过创建工程、持久化成员、真实 IPC 和减少动画检查；普通发行的验证计数命令不可调用。最终内置完整流程 19 项全部通过、零跳过（offline-results-13），使用真实传输、SDK 不可见的接收环境及不可用的原 checkout。
- 独立接收阶段对 NorthSensor、SouthActuator、GatewayReference、实时用户应用及 v1 参考包真实构建和运行全部通过；实时应用独立高位屏蔽向量成立，搬移包和原包源码字节不变。
- I/O 矩阵覆盖：普通开发由 npm/Vitest、31 个 Python unittest 及默认 Cargo 测试覆盖；离线交付由 Windows/Linux 四版本实际生成包消费、原有篡改/链接/工具链拒绝及 Python 3.10 实际 CLI 拒绝覆盖；异步编辑由批次提交后刷新失败、晚到批次、草稿预览失效、guard 取消测试及 Windows 原生流程覆盖；资源维护由 ABI 拒绝、上游篡改拒绝、自有摘要更新与独立 wheel 新 checkout 检查覆盖。上述覆盖用例均已实际运行通过；完整 aggregate 和最终 19 项桌面流程均已通过。

- 验收暴露并修复 Linux 深临时目录的 AF_UNIX 地址长度限制：仍在原 0700 私有目录内创建 socket，通过持有的目录 fd 的 /proc 路径访问，认证、guardian 与清理协议不变。长路径真实 socket 回归、12 个 execution 原生用例及安装程序的退出 23／完整日志／剪贴板／取消晚到响应聚焦流程通过。普通发行与验证 deb 已重新构建；最终完整 19 场景已通过。

- 开发 wheel 在 Python 3.11 的标准 uv 安装被准确拒绝并说明 >=3.12；离线 CLI 在实际 Python 3.10 被准确拒绝并说明 >=3.11，无新增补丁版本门禁。最终普通发行 deb 在显式 chroot 中通过 dpkg 安装、依赖及图标触发器，状态为 install ok installed；宿主包数据库和图标目录未改写。安装位置的真实程序在独立环境中通过工程创建、保存成员、IPC、减少动画及发行版拒绝验收命令检查。

- 大型工程切换执行状态会反复构建 16,000 个树行，干扰五秒取消窗口：保留树行的 JSX，事件通过当前动作引用处理；新增回归确认执行更新不重建树行、事件使用新动作且过滤仍更新。未延长验证延迟或取消断言。Python 离线 owner 同样使用持有的目录 fd 处理长临时目录，并在启动失败或关闭时释放；独立真实子进程及 fd/目录释放回归进入原进程套件。桌面 CI 上传排除官方 dependencies 档案。

- 最终普通 Linux Cargo 测试未设置外部 Python、受控工具或官方资源变量，5 个库用例和 62 个基础用例通过；包含 native-tests 的 clippy 通过。读者文档 242 个本地链接有效。最终 wheel/sdist 已重建，独立 wheel 对新 Git 导出 checkout 的资源检查通过；可选汽车维护的 24 个技能和 8 项测试再次通过。

- 大型工程的隐藏问题面板原本在执行状态变化时重建 16,000 个诊断行并反复扫描对象。工具窗口现只挂载当前面板并缓存定位索引；原有问题导航和界面保留，16 个 Vitest 与完整安装包流程通过。桌面 IPC 记录逐条完整写入，避免整份拼接超出字符串上限，不裁剪证据。

- 普通发行短检查第一次已通过功能断言，但驱动收尾报告 cleanup_unconfirmed；保留该失败及收尾记录。按相同条件完整复测通过（production-installed-final-smoke-2），服务关闭和 D-Bus socket 删除得到确认。未修改进程清理断言或把未确认状态当成成功。

- 316 个桌面包源码与交付资源输入和最终仓库内容一致，固定原始字节的运行时与离线工具直接比较；普通源码只允许 Git 行尾差异。完整验证后的局部修复已按影响范围复验：16 个 UI 用例、最终两平台 GUI、12 个 Linux 进程用例、四版本离线消费及普通发行安装检查均通过。
