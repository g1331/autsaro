---
title: Reopen delivered inputs and reproduce host C99 project
type: feature
created: '2026-09-28'
status: ready-for-dev
context:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
---

## Intent

**使用者成果：**接收工程的人在新目录中使用同版工作台与另行合法取得的 R24-11 XSD，按交接说明重新导入 Story 3.1 随包的 ARXML，看到当前固定主机剖面的校验结果，并生成与原交付等价的 C99 源工程。这证明交接包是可重建的，而不仅是可运行的二进制来源。

**依据与范围：**PRD R1–R4、R6、R5，Epic 3 的受限 Windows 主机交接。只要求当前受支持的 11 位 Classical CAN、一个虚拟控制器和已声明的有界物理 DoCAN 输入；完整 ECU Extract、SWC/RTE、AUTOSAR OS 与 MCU 不在本 story。

## Boundaries & Constraints

- 复用现有多文件导入、引用校验和确定性生成；新目录不能依赖原机器 ARXML 路径或未列出的文件；当前工作台从源码目录运行及所需官方 XSD 必须作为明示外部依赖，而不是假称已随交付包提供。
- 比较内容以与输出目录无关的生成源码、配置、脚本和清单为准；不可用机器绝对路径或运行时间戳制造虚假差异。
- 交接说明明确工具版本、主机目标、GCC/PowerShell 依赖，以及“重新生成等价”与“已独立验证行为”是不同状态。
- 不自动修复缺失、篡改或未支持的 ARXML；失败不得重写原输入或覆盖已有可用工程。

## Acceptance Criteria

1. 将 Story 3.1 的交付目录移动到不同路径后，另一执行者在准备好明示的同版工作台和合法 XSD 后按随包说明导入所有输入文件，经当前主机剖面校验并在新空目录生成；与原交付的声明文件集合及相关 C99 内容一致，且能独立链接。
2. 交接说明给出输入文件映射、必要版本和具体导入/生成/构建步骤；步骤不需要猜测仓库内未交付文件。
3. 缺少或篡改任一受支持输入、改变跨文件引用、工具不支持的版本或影响生成的变体时，导入/生成被定位拒绝，原输入及旧产物保持原状。
4. 结果只显示“输入可重建”“工程已生成/构建”等确已达到的状态；若未运行 Story 3.3 的独立向量，不显示主机行为已验证，更不显示实机或标准符合性已通过。

## Verification

在独立临时目录运行完整导入→再生成→文件集合及内容比较→GCC 构建；加入缺文件、篡改和跨文件断链反例。运行 `cargo test --manifest-path core/Cargo.toml` 和增量质量门；没有隔离桌面会话时只验证无头路径，界面声明另标未验证。
