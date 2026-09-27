---
title: Complete STD-001 host Can Driver slice
type: feature
created: '2026-09-27'
status: in-progress
route: dispatch
review_loop_iteration: 0
context:
  - docs/workflow/tasks/STD-001-host-can-driver-api.md
  - docs/workflow/evidence/STD-001-host-can-driver-api-2026-09-27.md
---

<frozen-after-approval reason="既有已授权任务迁移；范围由历史任务卡限定">

## Intent

**Problem:** 现有主机 CAN 代码已有标准命名入口，但接口、配置闭包和独立行为证据尚未完成复核。

**Approach:** 恢复 `STD-001`，核对现有实现与 R24-11 Can Driver 有界要求，只修复真实缺口，交付主机正反向证据。

## Boundaries & Constraints

**Always:** Windows/GCC、一个虚拟控制器和一个 Tx 句柄、11 位 Classical CAN、DLC 1–8；保留原信号闭环；逐条判断适用 SWS/MOD。

**Never:** 把主机接口等同真实硬件驱动、第三方互操作、完整 MISRA 或官方一致性。

## I/O & Edge-Case Matrix

| 场景 | 输入 / 状态 | 预期 | 失败处理 |
| --- | --- | --- | --- |
| 发送 | 有效配置、已启动控制器、有效 PDU | CanIf→Can 发送并保留可观察报文 | 输出失败如实传播 |
| 拒绝 | 错误句柄、ID、DLC、空指针、非法状态或变体 | 不发报文 | 返回拒绝或阻止生成 |

</frozen-after-approval>

## Code Map

- `runtime/src/Can.c`：标准入口与主机适配。
- `runtime/src/CanIf.c`：发送、确认与接收路由。
- `core/tests/end_to_end.rs`：生成工程和主机行为集成验收。

## Tasks & Acceptance

**Execution:**
- [x] 对照历史证据与当前源码，找出尚未完成的 STD-001 条目。
- [ ] 核对官方 R24-11 Can Driver/BSW General 与 ECUC MOD 的适用条款，修复可证实的缺口。
- [ ] 运行生成工程正反向主机验证、增量质量门和独立复核，记录未通过的能力门。

**Acceptance Criteria:**
- 有效双 ECU 配置发送保持正确且经过标准入口；无效输入与状态拒绝不发旧报文。
- 证据包含输入、独立预期、实际输出、基线提交、检查结果和剩余限制。

## Implementation Notes

迁移时已存在 `STD-001` 分支和实现。本文件仅恢复进行中状态，不宣称 story 已完成。旧任务基线提交：`c4a01b5cd41c`；迁移后新改动应另记 story 起始提交。

2026-09-27 实际演练起始提交 `35d0274`。独立只读复核发现运行中无效或重复 `Can_Init` 会重置控制器、丢失已排队报文；主 Agent 将初始化限制为首次有效配置，新增待发帧期间三种重入初始化的回归向量。补丁复核未发现这三个文件中的可执行缺陷。仍待落实历史任务卡列出的适用 Can/CanIf 标准义务及完整配置工件，尤其重复初始化的 DET 处理、控制器模式与 bus-off 通知、BSWMD/MemMap；不能仅因主机测试通过将 story 或 `HOST-CAN-01` 标为完成。

## Spec Change Log

## Review Triage Log

- 2026-09-27：独立只读审查发现 `Can_Init(NULL)` 或重复初始化可破坏运行状态。确认并修复；新增 C99 待发帧向量在旧实现会因控制器回到 STOPPED 失败。
- 2026-09-27：另一只读审查对 `Can.c`、`Can.h`、`end_to_end.rs` 补丁未发现待修复问题；该结论只覆盖补丁，不代替整个 Story 1.1 的标准义务复核。

## Verification

`python scripts/verify.py --scope all --base <story-baseline>`；`python scripts/verify.py --scope baseline --generated-dir <工程目录>`。按证据逐一执行隔离主机行为验证；原生 GUI 无隔离环境时标记未验证。

本轮以 `35d0274` 为基线运行增量门：12 个 Python 测试、3 个核心单元测试、53 个端到端测试、UI lint/构建、核心与桌面 Clippy/构建均通过。定向 `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 1/1 通过；双 ECU 金向量也在完整测试中通过。以新生成的独立代表工程运行全量基线：全文件格式、Python/UI lint、全告警 Clippy 与 Doxygen 通过；BSW/RTE/生成 C 的部分 MISRA 扫描分别报告 25/11/7 项，七个能力档案的 `spec_obligations` 仍为 `not_run`，故全量基线退出 1。原生桌面 GUI 未在隔离会话重新执行。Story 保持 `in-progress`。
