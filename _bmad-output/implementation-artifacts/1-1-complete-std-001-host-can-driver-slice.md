---
title: Complete STD-001 host Can Driver slice
type: feature
created: '2026-09-27'
status: done
route: dispatch
baseline_commit: fb776bfaef4016b3f4903136701b80b108ccc55e
review_loop_iteration: 0
context:
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
- [x] 核对官方 R24-11 Can Driver/BSW General 与 ECUC MOD 的适用条款，修复可证实的缺口。
- [x] 运行生成工程正反向主机验证、增量质量门和独立复核，记录未通过的能力门。

**Acceptance Criteria:**
- 有效双 ECU 配置发送保持正确且经过标准入口；无效输入与状态拒绝不发旧报文。
- 证据包含输入、独立预期、实际输出、基线提交、检查结果和剩余限制。

### Review Findings

- [x] [Review][Patch] 补齐诊断发送的 STOPPED 和 bus-off 拒绝向量 [core/tests/end_to_end.rs:396] — C99 harness 已覆盖拒绝、恢复及底层调用次数；定向测试和完整增量门通过。

Rejected:
- 历史段落中的 `in-progress` 记录带有当时基线与日期；后续段落给出最终切片结论，不是当前状态的第二来源。
- Story frontmatter 的 `done` 与 sprint 的 `review` 属于审查前过渡状态；项目规则以 sprint 文件作为当前进度来源，审查流程将按结果同步两处。
- 执行清单要求核对标准义务并记录未通过的门，未要求把完整 Can/CanIf 标准交付闭合；剩余缺口已记录。
- 最终验收结论明确限定固定主机切片，且明确保持 `HOST-CAN-01` 为 `documented_behavior`，没有宣称完整标准支持。
- Verification 段的占位命令属于 story 文档改写建议；本轮代码审查不将规格文件编辑列为代码缺陷。
- MemMap 头文件与运行时说明均限定为 Windows 默认链接段和宿主标记，没有宣称 MCU 内存分区。
- 历史 STD-001 任务卡约束的是完整标准义务；本 Story 的固定主机切片边界及未通过门已明确，能力声明未升级。

## Implementation Notes

迁移时已存在 `STD-001` 分支和实现。本文件仅恢复进行中状态，不宣称 story 已完成。旧任务基线提交：`c4a01b5cd41c`；迁移后新改动应另记 story 起始提交。

2026-09-27 实际演练起始提交 `35d0274`。独立只读复核发现运行中无效或重复 `Can_Init` 会重置控制器、丢失已排队报文；主 Agent 将初始化限制为首次有效配置，新增待发帧期间三种重入初始化的回归向量。补丁复核未发现这三个文件中的可执行缺陷。仍待落实历史任务卡列出的适用 Can/CanIf 标准义务及完整配置工件，尤其重复初始化的 DET 处理、控制器模式与 bus-off 通知、BSWMD/MemMap；不能仅因主机测试通过将 story 或 `HOST-CAN-01` 标为完成。

本次从 `fb776bfaef4016b3f4903136701b80b108ccc55e` 继续：按本地 R24-11 Can Driver PDF 页 33–36 的 `SWS_Can_00373`、`SWS_Can_00020`、`SWS_Can_00272`，以及 CanIf PDF 页 58、128、133 的回调签名和上层通知义务，补了固定控制器 0 的模式与 bus-off 回调。主机同步状态切换现通知 CanIf，CanIf 以受锁保护的状态拒绝未启动控制器的信号和诊断发送；无效控制器回调不改变该状态。现有主机链仍同步调用模式回调，没有 `Can_MainFunction_Wakeup` 异步派发或 CanSM 上层通知，因此标准义务尚未闭合，第二项执行任务和 story 继续保持进行中。

## Spec Change Log

## Review Triage Log

- 2026-09-27：本轮代码审查保留 1 项诊断发送门控的验证缺口，已在现有 C99 CanIf harness 补充 STOPPED、bus-off、恢复与错误控制器通知向量；定向测试及以 `fb776bfaef4016b3f4903136701b80b108ccc55e` 为基线的 `python scripts/verify.py --scope all --base ...` 通过。边界条件审查层返回空结果，故该层未提供有效意见；其他 7 条意见未形成代码缺陷。

- 2026-09-27：最终独立只读复核发现模式通知只在 `Ecu_Init` 轮询，运行期间 `Can_SetControllerMode` 后缺少常规调度入口。核对 `Os_Advance` 和主机命令 `T` 路径后确认；在有效虚拟时间推进时调用 `Can_MainFunction_Wakeup`，并更新运行时契约。
- 2026-09-27：blind review 未发现可复现缺陷。edge-case review 指出主机 `Can_SetMode` 接受不支持的枚举会改变控制器状态且取消待通知；核对源码确认（medium），增加无副作用拒绝并在待通知状态回归。verification-gap review 指出原测试直接调用 `Can_MainFunction_Wakeup`，删除 `Os_Advance` 的轮询连接不会使测试失败；核对后确认（medium），用 C99 harness 经 `Os_Advance` 触发并断言通知。

- 2026-09-27：独立只读审查发现 `Can_Init(NULL)` 或重复初始化可破坏运行状态。确认并修复；新增 C99 待发帧向量在旧实现会因控制器回到 STOPPED 失败。
- 2026-09-27：另一只读审查对 `Can.c`、`Can.h`、`end_to_end.rs` 补丁未发现待修复问题；该结论只覆盖补丁，不代替整个 Story 1.1 的标准义务复核。
- 2026-09-27：本轮独立只读审查指出模式回调仍在 `Can_SetControllerMode` 返回前同步发生。核对 Can Driver PDF 页 33 的 `SWS_Can_00373` 后确认是剩余标准时序缺口，当前主机同步行为已在运行时说明中标明，未标为闭合。
- 2026-09-27：同一审查指出 CanIf 回调只更新本地发送门控，没有向 CanSM/CDD 派发。核对本地 CanIf PDF 页 58 的 `SWS_CANIF_00724`、`SWS_CANIF_00711` 后确认上层通知仍缺失；当前工程没有该上层模块，本项留在本 story 的标准义务清单中。

## Verification


本轮以 `35d0274` 为基线运行增量门：12 个 Python 测试、3 个核心单元测试、53 个端到端测试、UI lint/构建、核心与桌面 Clippy/构建均通过。定向 `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 1/1 通过；双 ECU 金向量也在完整测试中通过。以新生成的独立代表工程运行全量基线：全文件格式、Python/UI lint、全告警 Clippy 与 Doxygen 通过；BSW/RTE/生成 C 的部分 MISRA 扫描分别报告 25/11/7 项，七个BMad 实现记录的 `spec_obligations` 仍为 `not_run`，故全量基线退出 1。原生桌面 GUI 未在隔离会话重新执行。Story 保持 `in-progress`。


2026-09-27 最终增量以 `f92fd6ade6c8d8582df750bc46a0cdd0d2d067de` 为起点。`python scripts/verify.py --scope all --base ...` 的 12 个脚本测试、3 个核心单元测试、53 个端到端测试、UI lint/构建、核心及桌面 Clippy/构建全部通过。`cargo test --manifest-path core/Cargo.toml generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults -- --nocapture` 在调度接入后再次通过。隔离 Windows Sandbox 原生桌面新建 `Story1Sandbox.arxml`，加入 `StatusTx`（11 位 CAN ID 321，DLC 8，100 ms Tx）及 `Counter`（bit 0，8 bit，初值 17），保存预览确认 1 处 ARXML 变化，运行诊断 0 错误，界面确认生成 48 项工程文件。宿主以生成的 `build.ps1` GCC 编译成功；向 `ecu_host.exe` 输入 `T 100`，实际输出 `X 321 8 1100000000000000`，与独立配置预期一致。隔离 Sandbox 未安装 GCC，构建与运行在宿主后台完成。全量基线对该 GUI 工程报告格式、lint、Clippy、Doxygen 均通过；部分 MISRA 扫描 BSW/RTE/生成 C 为 42/11/7 项，七个 `spec_obligations` 仍 `not_run`，因此基线退出 1。新增 MemMap 标记约定触发 BSW 部分扫描中的 20.1/20.5；它仅映射宿主默认代码与清零数据段。BSWMD、CanSM/CDD 上层通知、完整 MISRA 和官方一致性仍未闭合，`HOST-CAN-01` 维持 `documented_behavior`；此 Story 的结论只针对固定主机切片。

最终审查补丁后，以 story 起始提交 `fb776bfaef4016b3f4903136701b80b108ccc55e` 重跑完整增量门，12 个脚本测试、3 个核心单元测试、53 个端到端测试和全部构建、lint、Clippy 通过；定向 C99 harness 也独立通过。使用最终源码在隔离桌面再次预览并生成 `gui-generated-final`，宿主 GCC 构建成功，`T 100` 再次输出 `X 321 8 1100000000000000`。以该最终工程重跑全量基线，格式、lint、Clippy、Doxygen 通过，仍由上述 42/11/7 项部分 MISRA 与七个规范证据门退出 1。Story 的固定主机切片验收完成；不改变完整 AUTOSAR 能力声明。
