---
title: Package saved host ARXML inputs with generated ECU
type: feature
created: '2026-09-28'
status: done
baseline_commit: 14efd7abdf979354737a1d655d19fd4d795d7683
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
5. 现有已验证的生成 C99、构建和 CAN/有界诊断主机行为仍通过对应集成测试；新交付状态不升级 本 story 的验证记录 中的支持声明。

## Verification

在 `core/tests/end_to_end.rs` 覆盖多文件正向、同名路径、外部改动/脏输入、缺文件或断链及旧输出保护；运行 `cargo test --manifest-path core/Cargo.toml` 和本 story 的增量质量门。无需弹出桌面窗口；原生 GUI 路径仅在隔离桌面会话可用时验证，否则如实标未验证。

## Code Map

- `core/src/arxml.rs`：从已保存且与磁盘一致的 Workspace 提取原始输入及逻辑包根。
- `core/src/generator.rs`：把输入、映射和交接说明并入完整性清单及预览，复用安全暂存与旧输出保护。
- `src-tauri/src/lib.rs`、`ui/src/App.tsx`：提供独立于普通生成的交付入口。
- `core/tests/end_to_end.rs`：证明同名多文件、原字节保留、搬迁与失败保护。

## Tasks & Acceptance

- [x] 在 `core/src/arxml.rs` 提取已保存的来源输入；Given 未保存或外部修改，When 导出交付包，Then 明确失败且不改源文件。
- [x] 在 `core/src/generator.rs` 生成稳定的 `inputs/`、`handoff.json`、说明与完整性记录；Given 同名多文件，When 导出，Then 包内路径无冲突且元数据无原机器绝对路径。
- [x] 在 `src-tauri/src/lib.rs`、`ui/src/App.tsx` 接入预览及确认；Given 旧输出有用户改动，When 确认，Then 拒绝覆盖并保留旧包。
- [x] 在 `core/tests/end_to_end.rs` 验证已保存输入、多文件引用、失败关闭与主机工程回归。

## Review Triage Log

| 来源 | 判定与依据 | 处理 |
| --- | --- | --- |
| Blind 1：映射包根未核对 | medium：原实现可接受与 ARXML 不符的 `packageRoots`；`open_handoff` 现逐项核对实际包根。 | 已修复并加入反例。 |
| Blind 2：千份输入索引 | false：生成与导入均用相同的 `{:03}` 最小宽度格式，索引 1000 会扩展为四位；不存在互不兼容。 | 无需改动。 |
| Blind 3：未做 XSD 校验 | false：`open_handoff` 调用 `checked_profile`，其调用 `validate`，后者执行 `schema::validate_files`。 | 无需改动。 |
| Blind 4：缺少包名被跳过 | false：`handoff_sources` 先运行 `checked_profile` 的 XSD/剖面校验；不合法顶层包无法通过导出。 | 无需改动。 |
| Blind 5：未来嵌套输入目录 | low：当前受支持交付格式只有扁平 `inputs/{index}.arxml`；未来扩展格式须同时变更生成与读取。 | 不为未定义格式加分支。 |
| Blind 6：额外文件被忽略 | medium：旧离线校验只核对清单条目；现拒绝额外文件、目录及重解析点。 | 已修复。 |
| Blind 7：三类用例可被替换 | medium：旧脚本只核对数量；现核对固定名称、ECU、输入和预期结构。 | 已修复。 |
| Blind 8：向量输入和超时不预检 | low：旧脚本会在执行中失败但定位较晚；现先校验输入及超时区间。 | 已修复。 |
| Blind 9：暂存名碰撞 | low：纳秒时间戳与 PID 碰撞时创建会明确失败且不覆盖；重试不改变当前用户结果。 | 保持失败关闭。 |
| Edge 1：已改二进制被重新生成覆盖 | false：二进制放行仅用于只读 `open_handoff`；`generate_prepared` 仍经不放行二进制的校验，拒绝替换。 | 无需改动。 |
| Edge 2：报告路径经链接写回包内 | medium：原字面路径判断不解析链接；现拒绝已有报告和路径祖先重解析点，改将失败报告写到系统临时目录。 | 已修复。 |
| Gap 1：超时分支未验证 | medium：原测试没有触发 `taskkill`；现以大量合法输入和极短上限复验，检查非零退出和失败报告。 | 已补测试。 |
