# BUILD-001：保存前 ARXML 差异预览（2026-09-26）

## 范围与依据

- 基线 `cdff6ebe52d5`；实施分支 `feature/BUILD-001-arxml-save-preview`。输入是 CP/FO R24-11 `AUTOSAR_00053.xsd` 有效的两份 ARXML，无选定变体；目标是 Windows 原生 Tauri 工作台与本机文件。预览是工作台行为，不属于某个 BSW 模块的 SWS 服务；本卡不新增 ECUC 配置节点或修改生成 C99 语义。输入保存约束沿用[项目工件决策](../../../.scratch/autosar-platform/issues/09-project-artifacts.md)。
- 核心从 `Workspace` 已保存文本与待写文本构建只读预览，列出全部来源文件。修改项携带完整原文与拟保存文本，未修改项只标识路径和状态。确认保存带上预览修订摘要；后端复核当前配置，仍沿原保存路径再次校验 XSD、引用与全部来源字节，且保留暂存、安装和回滚机制。
- UI 按 XML 相邻标签分行显示差异范围，以便阅读压缩成一行的 ARXML；可展开查看未重排的完整前后文本。重排仅在展示层，不参与写入。界面截图：[双文件差异预览](BUILD-001-save-preview-2026-09-26.png)。截图中的 `802→803` 是另一次只读预览，随后取消；下面的落盘证据来自 `801→802`。

## 可复现输入与独立预期

隔离桌面原生演练从 UI 创建 `PreviewEcu`，加入 Tx `Command`：11-bit CAN ID `801`、DLC `2`、周期 `10 ms`；加入 `SendCount`：起始位 `3`、长度 `8`、初值 `5`，经预览确认保存。另写入一份 XSD 有效的独立 `Unrelated.arxml`，仅包含 `Other` 包下的 `Extra` I-SIGNAL，再由原生文件选择器同时导入两份。随后在帧检查器将 ID 从 `801` 改为 `802`。独立预期：预览列出主文件“将修改”、另一文件“保持不变”；预览期间磁盘字节不变；确认后主文件等于预览的拟保存文本，另一文件字节不变；重开显示 `0x322`。仅凭配置模型可独立算出 `802 = 0x322`，无需由生成结果反推。

核心回归入口：`imported_unknown_content_survives_supported_edit_without_rewriting_other_file` 使用两份文件和保留的未知信号，直接比较预览前后文本、未改文件状态、预览期间磁盘内容、确认后的磁盘内容与重开模型；`save_preview_rejects_edits_and_external_changes_after_preview` 覆盖预览后继续编辑及外部来源变化；`noncanonical_pdu_input_is_not_rewritten_or_generated` 覆盖不支持输入不能出现成功预览。现有三文件拆包测试继续验证跨文件信号与未修改文件原字节。

## 原生窗口与失败路径

用 Windows `CreateDesktopW` 在独立桌面启动实际 `autosar-config-desktop.exe`，WebView2 指向临时用户数据目录并打开本机 CDP 端口；用 CDP 操作真实 WebView DOM、用 Win32 窗口消息操作该独立桌面的原生目录/多文件选择器。未切换使用者前台。Vite 和应用均由本轮启动，演练结束停止。CDP 截图来自该 Tauri WebView；截图证明当时可见布局，不单独证明每一步操作，以下还用磁盘摘要和重开结果交叉核对。

| 场景 | 实际结果 |
| --- | --- |
| 双文件导入，编辑 `801→802` 后预览 | UI 显示 2 份来源、1 帧、1 信号；预览标主文件“将修改”，`Unrelated.arxml`“保持不变”。预览前后两文件 SHA-256 分别保持 `147083ad28e1a29a06a717b9070bab5672c11224467049549125b0744b06f84f` 与 `a667baf5136c93b95d29e4e21f69e27f8c9d653c563ea0a36ea1e0f31aaedc87`。拟保存文本含 `<VALUE>802</VALUE>`，其 UTF-8 SHA-256 是 `e0227165a6289785f35b0391a70f27f13805367d81e8dc661ebd162c49482be9`。 |
| 预览后外部修改未改文件，再确认 | 向 `Unrelated.arxml` 追加 LF 后点击“确认保存”；UI 报来源文件被外部修改并指出完整路径，预览关闭。主文件摘要仍为旧值，没有部分保存或覆盖外部字节。 |
| 恢复来源原字节、重新预览并确认，随后重新导入 | 主文件落盘摘要恰为上述拟保存文本摘要；未改文件仍为 `a667...edc87`。通过原生选择器重新导入两份，UI 显示 2 文件、`Command` ID `0x322`、配置已保存。 |
| 无法安全预览 | 不支持的旧 PDU 输入返回 `PDU_UNSUPPORTED`；核心测试确认未落盘且未生成输出。预览后改变工作区配置则修订摘要不匹配，提示重新预览。 |

原生演练使用临时文件，不随仓库提交；可移植的两文件建立及保留项场景在上述核心测试中固定。截图保存于本证据同目录。

## 构建门与限制

Windows 当前会话先设置 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`。最终源码执行 `python scripts/workflow.py verify --scope all`：工作流 Python 测试 **8/8**、UI TypeScript/Vite 构建、核心单元测试 **2/2**、核心端到端 **40/40**、Tauri 构建均通过；编译器仍报既有 `LNK4098` 默认库冲突警告。另执行 `python scripts/workflow.py check`，状态索引有效。

`HOST-CAN-01` 仍为 `documented_behavior`；本次只增强保存操作的可检查性，不补齐外部真实多文件 CAN 配置、独立离线交接、逐模块规范义务、指定 MCU、第三方互操作或静态质量门。预览与保存之间的并发文件修改仍受现有来源复核和暂存安装逻辑的能力边界约束，不能从顺序拒绝测试推断任意时序下的原子性。

## 独立复核

独立 Agent `build001_review` 于 2026-09-26 以 `cdff6ebe52d5..891c9fcc332c` 为审查范围，核对任务、源码、预览与确认 IPC、UI、测试、上述证据和截图，未发现阻断缺陷。同一新会话按 README 设置三项构建变量后，复跑 `imported_unknown_content_survives_supported_edit_without_rewriting_other_file`、`save_preview_rejects_edits_and_external_changes_after_preview`、`split_package_save_preserves_sources_and_rejects_stale_reference_file`、`noncanonical_pdu_input_is_not_rewritten_or_generated`，四项均通过；`git diff --check cdff6ebe52d5 891c9fcc332c` 通过。审查未重跑原生窗口或完整构建，因此这两项仍以实施证据为准。

审查指出：两文件原生场景中的第二份文件是独立保留文件；跨文件引用的 CAN 配置有既有核心测试，但该测试沿 `save()` 直接保存，未单独通过原生预览确认链。两个入口共用现有保存路径，当前不构成 BUILD-001 阻断；不能把本次证据表述为“外部跨引用多文件配置已在 UI 全链验收”。`HOST-CAN-01` 仍不升级支持声明。
