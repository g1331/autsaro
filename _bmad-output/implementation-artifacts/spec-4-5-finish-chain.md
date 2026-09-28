---
title: '4.5 原子终止、链式移交与新入口执行'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '8d66389db6c42b6f0779068528ed55032df5bf95'
story_key: '4-5-原子终止-链式移交与新入口执行'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous per-story execution">

## Intent

合法 TerminateTask/ChainTask 原子完成源实例并建立目标请求，成功不返回旧应用帧。拒绝必须保留源及目标完整前态。实际入口重新初始化局部变量，调度及上下文仍由唯一固定 FreeRTOS backend 拥有。

## Boundaries & Constraints

遵循 OSEK2.2.3 §13.2.3.2/.3（原文页51–52）和既定 E4-AR-6。自链不增加激活数量，但新入口按同级新就绪位置重新排列。Basic pending 请求保留 FIFO；Extended 从 Suspended 激活时清空事件。公开应用只能调用标准服务，不依赖 setjmp、私有 TCB 或第二调度器。4.18 负责完整 missing-end/Hook 清理；本条保留可观察的最低关闭处理。不扩大旧目标、OS选型或 W3。

## I/O & Edge-Case Matrix

| 场景 | 预期 |
| --- | --- |
| Terminate、有 pending、自链满容量 | 当前实例结束，下一入口局部初始化；自链不额外占槽且不返回旧帧 |
| 向高/同/低优先级 Chain | 目标最早在源完成后执行；现存同级请求和新增请求顺序正确 |
| 无效目标、目标满、持资源、ISR/Hook 调用 | E_OS_ID/LIMIT/RESOURCE/CALLEVEL；完整档案、事件、资源和内核位置不变 |
| observer/实际ISR在转换边界交错 | 只见完整前/后态，无请求丢失/重复，无旧帧哨兵 |
| Extended 目标重新激活/自链 | 事件按新实例初始化；拒绝不清事件 |
| 应用入口直接返回 | 最低失败关闭可见，无后续应用执行；不宣称完整4.18出口 |

</frozen-after-approval>

## Code Map

- `runtime/os/src/Os_Backend.c`：复用完整请求环、原子接纳门及同原生线程根帧。抽出共享完成事务，先校验再同时改变源/目标；仅驱动已有内核队列。
- `runtime/os/include/Os.h`、`Os_Target.h`、`src/Os.c`、`Os_Backend.h`：标准 ChainTask 及 E_OS_RESOURCE；静态资源/事件核心支持真实持资源拒绝和新实例事件重置。
- `runtime/os/tests/finish_chain.c`：标准任务入口、literal轨迹与独立 TCB/档案 observer，不用测试器选择任务。
- `scripts/epic4_os.py`、`core/tests/end_to_end.rs`：注册恰选 `epic4_finish_chain_atomicity`；保留前序回归。
- 固定内核原件不改。0003 的排序政策及唯一 selector 继续复用，不需要新的 CPU 上下文机制。

## Tasks & Acceptance

- [x] 实现共享完成事务与 ChainTask；自链解除当前请求、按新请求位置接续，不突破激活上限。
- [x] 最小实际静态资源所有权/ceiling、事件存储接入，供持资源拒绝/事件重置验证。完整非抢占、内部资源、ISR资源和无丢失等待仍由4.6/4.7负责。
- [x] 增加独立成功、拒绝、重入/边界向量与 missing-end 最低关闭；保存真实证据。
- [x] 恰选正式入口、前序回归、完整增量门、三视角只读复核；同步 sprint、本地提交、不推送。

Given 合法完成/移交，When 实际 backend 转换，Then 任务从正确入口执行、局部值重新初始化且无旧帧哨兵。

Given 目标或源不满足契约，When 服务拒绝，Then 原实例、目标请求、事件、资源和 ready 位置全部保留。

Given 独立观察者与真实ISR，When 转换边界交错，Then 只见完整前/后态且每次合法请求恰执行一次。

## Design Notes

关闭与接纳沿用4.4共享原子门。Chain 自目标容量核验扣除本次结束的当前实例，不先执行可能失败的 ActivateTask。源完整前置校验后，在同一临界事务中完成源出队和目标入队；源挂起或按 pending 键重排，目标只在零→一时恢复。成功通过原生 backend 根帧重启，公共 API 不返回旧帧。

资源核心使用静态配置/有界 LIFO 所有权与实际内核有效优先级，不能以测试注入 held 布尔值冒充真实持有。事件初始化在实际新实例转换中执行；当前仅接入标准存储/查询，4.7 再闭合 Wait/Clear 所有权和竞态。无新的费用、外部服务或不可逆操作；用户已授权逐条自主实施和本地提交。

## Implementation Notes

## Spec Change Log

## Review Triage Log

## Verification

`cargo test --manifest-path core/Cargo.toml epic4_finish_chain_atomicity -- --exact --nocapture` 必须恰选一项。激活20、生命周期46、真实栈23回归和 `python scripts/verify.py --scope all --base 8d66389db6c42b6f0779068528ed55032df5bf95` 均须通过；检查保留实际源码身份、失败事实及待批准偏离。未完成前保持 in-progress，Epic4 不标done。


## Implementation and Verification Progress

Shared complete_activation validates call level/source resource depth/target identity and capacity before mutation. Same-task Chain pops current and appends replacement without increasing total activation count; all paths use the existing atomic admission gate. Source and target queues/ready keys update within one native critical transaction, and only the kernel chooses runnable/context. The private native root frame remains unchanged across non-local completion. Captured missing-end is still the explicit minimum E_OS_STATE closure, with4.18 full obligations open.

Necessary real resource/event core supports the story's held-resource/error-preservation and Extended initialization contracts: static access/ceiling/LIFO, saved kernel priorities, stored event masks and independent snapshots. No test-only held boolean or simulated resource is used. Full NON/internal/ISR resource and Wait/Wake paths remain4.6/4.7. Actual scanner array-bound diagnostics prompted explicit depth guards; narrow operand and scope issues were repaired. SetEvent namespace collision was reproduced by strict compile and isolated through a separate native adapter and reviewed fourth copied-kernel patch. No CPU/context/selector replacement.

20 native vectors pass; exact Rust entry selected1 and passed. Previous activation20, stack23 and lifecycle46 regressions passed after namespace/ABI changes. Python OS verifier4 tests pass; positive replay now precedes each output mutation, avoiding a false-positive mutation test caused by replaying a single fixture for all scenarios. Finish checker catches wrong entry order, old-frame return marker and missing observer with a full accepted replay control.

Current partial Cppcheck scan includes actual copied tasks.c plus wrappers/backend/native-stack/native-event and harness units; Windows declarations/model remain incomplete. Changed-source diagnostic dispositions and source hashes are retained without suppression or conformity claim.4.4 historical evidence is not overwritten; current4.5 evidence is finish-chain.json/finish-static-analysis.json. Final three-lens review and full increment gate remain pending; do not mark4.5done or openW3.


## Independent Review Triage

| Finding | Verdict | Evidence and disposition |
| --- | --- | --- |
| Blind: fourth patch calls Os_HostSetEvent without a declaration, so strict build fails | false | Production copied port already includes Os_Backend.h via0001; that header includes Os_Windows.h, which declares Os_HostSetEvent. Fixed original vendor is not the compiled closure. Actual C99 -Wall -Wextra -Werror finish20 and prior regression builds linked the separate adapter successfully. No missing declaration occurs. |
| Edge reviewer | none | No edge findings returned. |
| Verification-gap reviewer | none | No verification gaps returned. |

All independent review results collected before triage. No actionable story issue remains. Current partial scan changed-source entries are all dispositioned; Windows import macro departure is explicit, not suppressed or declared complete MISRA. Final full increment gate follows on the reviewed tree.


## Final Static Ceiling Repair and Environment

Primary OSEK §8.5 (page31) requires ceiling at least the highest accessing task priority and below higher-priority non-accessors. Focused source review found the initial private configuration only checked numeric range. Added source-based constructor relation checks and genuine negative vectors for too-low and too-high ceilings; valid fixture now declares A/H accessors, ceiling5 below non-accessing launcher6. Current finish suite passes22 vectors; frozen Finish/Chain intent unchanged. This is the necessary valid resource core, not a full4.6/SC1 claim.

Full increment gate passed26 Python, UI lint/build,3 core unit/64 integration and core clippy, then desktop archive failed with Windows error112 (D disk full). Automatic approval review rejected recursive deletion of the identified build cache with reason blocked by policy; no cache was deleted by the agent. Artifacts/source preserved; desktop build is retrying in an explicit separate local temporary Cargo target with profile.dev.incremental=false. Same pinned dependencies/product source/checks remain, no acceptance rule disabled. Final applicable desktop build/clippy and the new constructor predicates need successful evidence before done.


## Acceptance Closure

Scoped static-ceiling repair review returned no findings. Current finish22, activation20, actual stack23 and lifecycle46 vectors pass, and exact formal Rust finish entry selected1 and passed on final constructor code.26 Python tests and final incremental quality pass. Both current shared artifacts match all product source hashes;34 changed-source static entries have explicit dispositions with no unresolved entry, suppression or false conformity claim. Required final4.20 quality/deviation obligations remain open.

Full increment's accepted checks are reused where unaffected: UI lint/build,3 core unit/64 integration and core clippy passed. Desktop build's initial D-disk error112 remains recorded; the same desktop build and full-target clippy subsequently passed with an explicit separate Cargo target and profile.dev.incremental=false. This changes cache placement, not rules/product interfaces. The final constructor-only change was revalidated by its exact22-vector entry, all previous native suites,26 Python checks and incremental quality. No visible UI/IPC execution is implied.

Story4.5 is accepted and sprint synchronized; Epic4 remains in-progress. Commit locally, no push, continue4.6. Full NON/internal/ISR resource, Wait/Wake, comprehensive Hook/missing-end and full SC1/handoff exits retain their planned dependencies.
