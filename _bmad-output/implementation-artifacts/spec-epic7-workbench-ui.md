---
title: 'Epic 7 真实统一壳层、定义编辑、草稿保护与主题身份'
type: feature
created: '2026-10-03'
status: done
route: dispatch
baseline_commit: '52367974cab5f73ce5a336936d5c0f352290dae1'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/spec-epic-7-configurator.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
  - '{project-root}/_bmad-output/planning-artifacts/epics.md'
  - '{project-root}/_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md'
  - '{project-root}/DESIGN.md'
  - '{project-root}/_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md'
---

<frozen-after-approval reason="derived from approved Epic 7 intent">
## Intent
把全部既有真实 CAN/诊断/DTC/标准输入/交接/源码预览/预检/构建/主机运行/取消迁入用户已定稿的新统一工作台，同时为 7.1–7.6 的真实通用源投影、定义/引用/批次/结构/模板工程接入可用表面，落实 7.7–7.9 全验收。
## Boundaries & Constraints
同一 useWorkbench reducer/call epoch/fingerprint，不建第二 store。后台业务 IDs/RuleSetIdentity/definitionFingerprint 原样回显，值 kind/lexeme 不转 JS number。无官方资源总门禁，无编译器仍配置和预览源码。不能删旧能力、做假引用/按钮/原型数据/成功计时器，不用浏览器 mock invoke。不得提交或推送、运行用户桌面窗口。
</frozen-after-approval>

## Code Map
- `ui/src/App.tsx`、`ui/src/workbench/useWorkbench.ts`、既有 pages/components/types：唯一状态及真实所有命令。
- `core/src/project_model.rs`：协调者已经实现统一 wire 类型；前端定义与其逐字段一致。
- 根 `DESIGN.md`、当前 `EXPERIENCE.md`：中性明暗 tokens、13px 系统字体、树表检查器/菜单/工具窗口、草稿/焦点/快捷键；冻结 HTML 只参考布局，不复制其 setter 场景。
- 定稿 logo 原始资产位于规划 mockups，使用实际 PNG；迁入 ui/public/src-tauri/icons 并移除旧正式图标引用。

## Tasks & Acceptance
- [ ] 独占整个 `ui/` 和 `src-tauri/icons/`；允许修改图标相关 Tauri 配置字段，但不改 command/backend Rust/平台工具契约。读取适用 frontend-design、interaction-design、vercel-react-best-practices、composition 技能并沿既定设计实现，不另起视觉方向。
- [ ] 菜单/上下文操作条/工具轨、一棵对象或文件树、复用文档标签、对象表、属性/真实引用检查器、底部问题/生成/构建/主机验证/日志/状态栏；1480×920、1080×720可用，窄屏折面板/表局部滚动，不造未来模块。
- [ ] 保留所有实际 legacy CAN/diagnostic/DTC/standard/generated/build/host/cancel 操作，公共动作统一能力/草稿保护；当前真实只读标准类型/端口/映射边界保持。起始页新建/ARXML导入/成员工程打开/两类包进入同一壳层。
- [ ] 通用树/字段/引用/源原文只消费 projection/descriptor/真实候选；多选择、搜索名称/path/definition/value不丢选择/草稿；prepareChange 展示逐项真实影响、明确 apply 确认，草稿变动失效 changeRevision。结构创建/改名/移除表面据后台定义/capability，不做未接线功能。
- [ ] IPC 统一新命令：`project_projection`、`read_project_source`、`prepare_configuration_change`、`apply_configuration_change`、`import_definition_catalog`、`remove_definition_catalog`、`open_member_project`、`preview_project_creation`/`create_project_previewed`、`preview_save_as_project`/`save_as_project_previewed`、`configure_appearance`。均返回既有 Reply<T>；当前 token 从 capabilities 获取。命令参数/序列化字段按共享模型和协调者确认，不自行伪造后台。
- [ ] 草稿覆盖对象/诊断/DTC/标准参数/批次；全部上下文替换“应用并继续/放弃/留在原处”，所有工程替换/重开/交接/退出“返回并保存/明确放弃/取消”；保存需真实 preview+confirm 后继续，失败不导航。普通面板/标签/主题/设置不丢草稿。
- [ ] 问题依真实 source/object/field 接力选树/文档/行/字段，无法定位保留复制详情。Ctrl/Cmd+K/S/O、Alt+1/4/9、Escape共用命令；树键盘、label/aria-invalid/describedby、非颜色状态、dialog焦点圈定归还、温和 live/reduced motion；全局快捷键不吞文本。
- [ ] 设置外观/规则与模块定义/执行工具独立草稿/类别提交。只读内置版次/hash/覆盖，扩展原生选择+显式接纳/移除+版本/摘要/消费者/工程dirty提示；旧官方资源仅明确兼容/开发分区。工具环境优先；theme light/dark/system持久，系统事件不重建工程。
- [ ] 透明珊瑚 A 的浅/深灯位同步，固定通用桌面 ICO/ICNS，正式 favicon/窗口/安装资产统一；旧图标引用移除。
- [ ] 所有解析/验证/日志来自后台；重任务期间工具窗口/已知路径复制可用，owned 长日志摘要有界及完整复制；不复制第二业务状态。

## Design Notes
协调者独占 Tauri Session/IPC/core integration；命令正在同批实现。接口疑问先发 Orca question；保持真实已存在命令可用，其他按后台能力接入，不 mock 或添加假 fallback。可拆 hook 内部模块但每文件只有本 worker 修改。不启动子代理、不自行运行 build/lint/tests/formatter/浏览器/可见 UI/安装；协调者统一运行真实隔离表面核查。不维护额外设计报告。

本仓库安装的 Build 技能实际目录为 `D:/Codebase/Autosar/.agents/skills/bmad-build`。新 Dispatch 的 renderer 必须使用此绝对路径，不能猜测不存在的用户全局目录；前次因错误路径 HALT，未修改代码，新次从已批准规格重新激活。父级技能已成功渲染，本任务仍遵守自身新次激活的 render-once 规则。

## Implementation Notes

### 已确认 IPC 数据包

所有下列命令返回真实 `Reply<T>{value,capabilities,inputFingerprint}`。输入 `fingerprint` 取当前 capabilities；返回后以新 capabilities 为权威，不能用请求 echo。身份/值按 `core/src/project_model.rs` 逐字段一致序列化，不经 JS number。

| Command | Payload | T |
|---|---|---|
| `project_projection` | `{fingerprint}` | `ProjectProjection` |
| `read_project_source` | `{fingerprint,sourceId}` | `string` |
| `prepare_configuration_change` | `{fingerprint,changeSet}` | `ChangePreview` |
| `apply_configuration_change` | `{fingerprint,changeSet,changeRevision}` | `ChangeOutcome` |
| `import_definition_catalog` | `{fingerprint,catalogPath}` | `ProjectProjection` |
| `remove_definition_catalog` | `{fingerprint,catalogId}` | `ProjectProjection` |
| `open_member_project` | `{fingerprint,path}` | `WorkspaceView` |
| `preview_project_creation` | `{fingerprint,directory,name,templateId}` | `ProjectCreationPreview` |
| `create_project_previewed` | `{fingerprint,preview}` | `WorkspaceView` |
| `preview_save_as_project` | `{fingerprint,directory,name}` | `ProjectCreationPreview` |
| `save_as_project_previewed` | `{fingerprint,preview}` | `WorkspaceView` |
| `configure_appearance` | `{fingerprint,appearance}` | `null` |

`ProjectCreationPreview={revision,templateId,directory,name,files:[{path,contents}],acceptedExtensionDefinitions}`。确认以完整预览回显；后台重新核对真实模板/源/规则/定义/目的地，不信任复制的 preview 对象。

Capabilities 新增 `ruleSetIdentity:RuleSetIdentity|null`、`ruleError:string|null`、`ruleCoverage:RuleCoverage[]`、`definitionFingerprint:string|null`、`appearance:"light"|"dark"|"system"`、`actions:ActionCapability[]`。`resourceError` 仅为可选 legacy/开发档案错误；`toolError` 仅限制执行。实际作用域结果以 projection.validation 为准。

Projection 新增后台真实 `referenceCandidates:ReferenceCandidate[]`、`instanceDefinitions:InstanceDefinitionOption[]`、`extensionDefinitions:ExtensionDefinitionView[]`。候选按 fieldId/真实目标及允许定义给出；创建选项同时给 parentId、定义、prospective fields/children；扩展给精确身份、实际 source/消费者/可用性与原因。不能由前端路径筛选发明合法性。

### 已注册的交付 follow-on

- `open_handoff_project` 实际参数 `{fingerprint,directory,newWorkspaceDirectory:null|string}`，返回 `Reply<WorkspaceView>`。v2 格式精确为 `autosar-workbench-handoff-v2`，必须显式选择新的空 live 工作目录；重建／规则／seal／拥有权核验全部成功才替换。原 host/ecu v1 分派和显式固定档案保持，不自动升级。接入真实文件选择与同一替换守卫；不复制 source snapshot 状态。
- 当前 `GenerationPreviewFile` 有 nullable `owner/producerId/snapshotOf`，展示实际提供者拥有权；`Capabilities.configuredExecutionTools` 是 saved 配置，`executionTools` 是 effective 环境覆盖，编辑草稿只能取 saved。
- `Capabilities.verificationMode` 为实际 `native-webdriver` 编译模式；公开命令面板中的“验证：受管长日志失败”“验证：受管取消”只在该模式出现，走真实 `verification_owned_failure` 的 `value.log`、非零 exitCode、owned 详情与取消路径。不能调用私有 React state/setter 验收。
- 基础统一 UI 已三次实际通过授权 TypeScript/Vite build，但未原生验收；先前 dispatch 因依赖尚未发布正确报告失败／不完整，代码保留。此次由已结算终端复用继续真实 v2 和 source-owner 将发布的 application create-only preview/confirm；后者未落地前不发明 DTO 或按钮，协调者及时提供实际接口。
- 本次 dispatch `ctx_cf7433b61d37` 已接入实际 `preview_application_initialization {fingerprint}` → `Reply<ApplicationInitializationPreview>` 与 `initialize_application_previewed {fingerprint,preview}` → `Reply<ApplicationInitializationOutcome>`。canonical DTO 来自 `core/src/arxml/application.rs`：预览为 revision／slot／files{path,contents}／manifestBefore／manifestAfter，结果为 projection／warnings／retainedRecoveryFiles；slot 的 producerSlot／componentPath／sourcePaths／generatedHeaders／entrySymbols 原样回显，seed 只读，未硬编码符号或伪造接纳。
- 应用初始化复用 edit 能力、草稿守卫及 current-token call；确认后消费真实 projection／警告／恢复文件并失效旧生成、构建、运行，草稿／输入／定义／工程及取消变化清除旧确认。共享 v2 导入对话框实际选择封存包与新空 live 目录；legacy 明确传 null。后台重建期间退出模态以保留工具窗口与取消入口，失败回到原输入，不前端判断 seal 或重写源。

## Verification
报告实际文件、真实命令接入、未执行的检查/截图与任何风险。最终 UI 由协调者在隔离 Windows/Linux原生入口和无头实际页面验证，不能声称已验收。

- 协调者通过本 dispatch 的 `msg_bb3ff1164195` 回报中央 `bg_68`：`npm run build --prefix ui` exit 0，TypeScript `tsc -b`＋Vite 6.4.3，1924 modules，JS 425.51 kB、CSS 17.50 kB。此为协调者运行的当前 UI 类型／构建证据，worker 没有重跑。
- 协调者 `msg_a87a1cddb8e9` 回报中央 `bg_66`：`cargo check --locked --manifest-path src-tauri/Cargo.toml --tests --features native-webdriver` exit 0，无警告，实际 application IPC 类型编译通过。`bg_67` 是运行中的真实 builtin 模板／create-only 应用／preview-confirm／v2 重建 API smoke；未收到完成结果，不计通过。
- 后续协调者 `msg_e66ee2b0b99b` 回报中央最终 `bg_78` API smoke exit 0：原创 CAN 的 1 个源及标准 ECU 的 7 个源、显式 create-only live 初始化、preview-confirm 封存生成、独立 v2 重建及 ARXML／application 原字节一致；未配置外部档案或编译器，warnings 为空。此为真实 API／源文件 smoke，不替代原生 UI 操作。
- 同一消息回报中央 `bg_75` UI ESLint exit 0、0 warnings；协调者仅修正 ProjectTree 的类型注解，未改运行值依赖或抑制规则。中央仅对全新 UI/helper 文件应用 Prettier，旧文件未整体重排；UI 继续冻结，后续原生发现仍须在实际页面复验。
- 协调者 `msg_10b3ea9812ff` 回报最终 UI 自 `bg_85` 后未变，TypeScript/Vite build exit 0，1924 modules、JS 425.52 kB／CSS 17.50 kB；中央 `bg_114` Windows `scope all` exit 0，12 unit＋174 未过滤 integration、UI lint/build、Ruff quality、core/desktop Clippy 与 desktop build 均通过。Linux 13 unit＋141 未过滤 integration 及最终 identity `epic7_59` 通过，以上均为中央运行证据。
- 同一消息确认最终 MSI／deb／AppImage 实际构建及提取 `bd20` snapshot；Windows MSI 第一次验收 exit 1，原因是 `native_elevate` 读取高权限创建的 controller status 出现 `PermissionError 13`，产品启动／清理尚未观察，不能作为 UI 缺陷或产品验收通过。Linux 原生普通入口验收仍在推进，全部 AC 继续未勾选，worker 不额外启动运行槽位。
- 按协调者定向请求，只读核对恢复后的 `EditorPage.tsx` communication wrapper 50–217：requestSave 的守卫、帧／信号实际路径选择及表格、创建动作与 scoped empty state 均保留，未发现具体保留差异；这是源码保留核对，不是实际 UI 操作证据。UI 保持冻结，等待具体原生发现或中央最终结算。
- 本 worker 未执行 lint／formatter／浏览器／原生窗口或截图；未修改后端、提交或推送。Windows/Linux 隔离原生 UI、完整 guard/focus/clipboard/cancel 与发行入口实际验收仍由中央完成，本记录不标 Tasks & Acceptance 已验收，也不将 UI build 当作运行或产品能力通过。

## 2026-10-06 开发收尾

用户明确要求本轮只确保开发与单元／集成测试完整，不执行真机或原生发行场景，不扩建隔离／验收设施。本规格的开发部分已随[中央实施规格](spec-epic-7-configurator.md)完成开发检查与独立复核；这里的 done 限定为该开发范围。原未完成的发行／GUI验收事实保留，后续交给CI，不写成已通过，也不自动恢复旧场景。
