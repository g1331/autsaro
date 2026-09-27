---
title: Package saved host ARXML inputs with generated ECU
type: feature
created: '2026-09-28'
status: ready-for-dev
context:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
---

## Intent

**使用者成果：**使用者经工作台明确的“导出可重建主机交付包”操作取得受限 Windows 主机 ECU 工程，同时携带本次生成所依据的已保存 R24-11 ARXML 输入，接收者能够识别来源，并用同版工作台及合法取得的 R24-11 XSD 在新位置重新导入。当前 `files.list`、`files.sha256`、`build.ps1` 已覆盖生成源码和单 ECU 构建；`runtime/generated-README.md` 明确说明 ARXML 源文件尚未包含。本 story 补的是输入交接，不改变 CAN/诊断运行语义。

**依据与范围：**PRD R1、R2、R4、R6、R5 的交接表述；R24-11 `AUTOSAR_CP_TR_Methodology.pdf` 的 `TR_METH_01114` 区分标准 ECU 配置输入来源。本 story 仅打包当前工具确实解析并允许生成的主机配置，不能称为已支持完整 ECU Extract、BSW Delivered Bundle、SWC 描述或标准 AUTOSAR OS。

## Boundaries & Constraints

- 复用 Workspace 的当前文件集合、校验、保存和生成预览机制；现有“生成主机工程”操作保持可用但不得突然改称完整可重建交付；新的导出操作经 Tauri 使用核心交付函数。
- 交付模式只接受全部源文件已保存、仍与磁盘一致且当前固定剖面可生成的状态。跨文件引用须在包内或由当前明确的外部契约覆盖；无法证明闭包时拒绝。
- 输入快照保留 ARXML 文本及跨文件引用所需的逻辑身份，以包内相对路径和映射表达，生成的路径映射不写原机器绝对路径；不额外复制 `ecu.key`、NvM/安全状态或其他运行秘密，原始 ARXML 内容由交付者审阅。
- 输入快照及映射进入现有 `files.list`/`files.sha256` 和生成预览；已有输出受用户修改时继续拒绝覆盖，失败后旧输入和旧产物保持可用。

## Acceptance Criteria

1. 从一组含跨文件引用的已保存合法 R24-11 主机配置交付后，仅用交付目录即可列出所有实际参与生成的 ARXML、各文件的逻辑身份、R24-11/工具/Windows 主机目标信息及其完整性记录；原 ARXML 字节内容不被静默规范化。
2. 单文件输入、同名但不同目录的多文件输入都能得到无冲突的包内路径；有歧义、越界、链接或不安全路径时明确拒绝，不写入包外。
3. 未保存编辑、源文件外部改动、缺文件、影响生成的断开引用、未支持变体或已修改的旧输出，均在交付前拒绝并指出原因；旧输出字节与用户源文件不变。
4. 工具生成的元数据中不出现源机器绝对路径，交付目录不额外包含密钥或运行状态；源 ARXML 原文由交付者审阅；`files.list`、`files.sha256` 和预览列出的文件集合一致，重复交付同一输入产生稳定内容。
5. 现有已验证的生成 C99、构建和 CAN/有界诊断主机行为仍通过对应集成测试；新交付状态不升级 `docs/assurance/capabilities.json` 中的支持声明。

## Verification

在 `core/tests/end_to_end.rs` 覆盖多文件正向、同名路径、外部改动/脏输入、缺文件或断链及旧输出保护；运行 `cargo test --manifest-path core/Cargo.toml` 和本 story 的增量质量门。无需弹出桌面窗口；原生 GUI 路径仅在隔离桌面会话可用时验证，否则如实标未验证。
