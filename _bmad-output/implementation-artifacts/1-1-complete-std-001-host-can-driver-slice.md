---
title: Complete STD-001 host Can Driver slice
type: feature
created: '2026-09-27'
status: in-progress
route: dispatch
baseline_commit: fb776bfaef4016b3f4903136701b80b108ccc55e
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
- [x] 运行生成工程正反向主机验证、增量质量门和独立复核，记录未通过的能力门。

**Acceptance Criteria:**
- 有效双 ECU 配置发送保持正确且经过标准入口；无效输入与状态拒绝不发旧报文。
- 证据包含输入、独立预期、实际输出、基线提交、检查结果和剩余限制。

## Implementation Notes

迁移时已存在 `STD-001` 分支和实现。本文件仅恢复进行中状态，不宣称 story 已完成。旧任务基线提交：`c4a01b5cd41c`；迁移后新改动应另记 story 起始提交。

2026-09-27 实际演练起始提交 `35d0274`。独立只读复核发现运行中无效或重复 `Can_Init` 会重置控制器、丢失已排队报文；主 Agent 将初始化限制为首次有效配置，新增待发帧期间三种重入初始化的回归向量。补丁复核未发现这三个文件中的可执行缺陷。仍待落实历史任务卡列出的适用 Can/CanIf 标准义务及完整配置工件，尤其重复初始化的 DET 处理、控制器模式与 bus-off 通知、BSWMD/MemMap；不能仅因主机测试通过将 story 或 `HOST-CAN-01` 标为完成。

本次从 `fb776bfaef4016b3f4903136701b80b108ccc55e` 继续：按本地 R24-11 Can Driver PDF 页 33–36 的 `SWS_Can_00373`、`SWS_Can_00020`、`SWS_Can_00272`，以及 CanIf PDF 页 58、128、133 的回调签名和上层通知义务，补了固定控制器 0 的模式与 bus-off 回调。主机同步状态切换现通知 CanIf，CanIf 以受锁保护的状态拒绝未启动控制器的信号和诊断发送；无效控制器回调不改变该状态。现有主机链仍同步调用模式回调，没有 `Can_MainFunction_Wakeup` 异步派发或 CanSM 上层通知，因此标准义务尚未闭合，第二项执行任务和 story 继续保持进行中。

## Spec Change Log

## Review Triage Log

- 2026-09-27：独立只读审查发现 `Can_Init(NULL)` 或重复初始化可破坏运行状态。确认并修复；新增 C99 待发帧向量在旧实现会因控制器回到 STOPPED 失败。
- 2026-09-27：另一只读审查对 `Can.c`、`Can.h`、`end_to_end.rs` 补丁未发现待修复问题；该结论只覆盖补丁，不代替整个 Story 1.1 的标准义务复核。
- 2026-09-27：本轮独立只读审查指出模式回调仍在 `Can_SetControllerMode` 返回前同步发生。核对 Can Driver PDF 页 33 的 `SWS_Can_00373` 后确认是剩余标准时序缺口，当前主机同步行为已在运行时说明中标明，未标为闭合。
- 2026-09-27：同一审查指出 CanIf 回调只更新本地发送门控，没有向 CanSM/CDD 派发。核对本地 CanIf PDF 页 58 的 `SWS_CANIF_00724`、`SWS_CANIF_00711` 后确认上层通知仍缺失；当前工程没有该上层模块，本项留在本 story 的标准义务清单中。

## Verification

`python scripts/verify.py --scope all --base <story-baseline>`；`python scripts/verify.py --scope baseline --generated-dir <工程目录>`。按证据逐一执行隔离主机行为验证；原生 GUI 无隔离环境时标记未验证。

本轮以 `35d0274` 为基线运行增量门：12 个 Python 测试、3 个核心单元测试、53 个端到端测试、UI lint/构建、核心与桌面 Clippy/构建均通过。定向 `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 1/1 通过；双 ECU 金向量也在完整测试中通过。以新生成的独立代表工程运行全量基线：全文件格式、Python/UI lint、全告警 Clippy 与 Doxygen 通过；BSW/RTE/生成 C 的部分 MISRA 扫描分别报告 25/11/7 项，七个能力档案的 `spec_obligations` 仍为 `not_run`，故全量基线退出 1。原生桌面 GUI 未在隔离会话重新执行。Story 保持 `in-progress`。

本次以 `fb776bfaef4016b3f4903136701b80b108ccc55e` 对最终源码运行 `python scripts/verify.py --scope all --base ...`：12 个脚本测试、3 个核心单元测试、53 个端到端测试、UI lint/构建、核心及桌面 Clippy/构建通过，含双 ECU 金向量和新增回调/门控定向 harness。单独的 `cargo test --manifest-path core/Cargo.toml standard_can -- --nocapture` 两项通过。使用最终源码的 `quality_sample` 新生成代表工程，`files.list` 含 `Can.c`、`CanIf.c`、`CanIf.h`；运行 `python scripts/verify.py --scope baseline --generated-dir ...`：格式、Python/UI lint、全告警 Clippy、Doxygen 通过；BSW/RTE/生成 C 部分 MISRA 分别为 25/11/7 项，七个规范证据门仍 `not_run`，基线退出 1。当前未在隔离桌面会话复验原生 GUI；能力声明保持 `documented_behavior`。
