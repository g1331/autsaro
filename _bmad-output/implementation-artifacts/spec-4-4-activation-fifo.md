---
title: '4.4 完整激活请求 FIFO 与实际内核状态'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: '0163d1259a47b650a318048abe35a4995caa7037'
story_key: '4-4-保留全部激活请求-fifo-与实际内核状态'
context: []
---

<frozen-after-approval reason="human-owned Epic 4 intent; preserve without human renegotiation">

## Intent

在固定 FreeRTOS backend 内保存每一次合法激活请求，维护同优先级请求 FIFO、抢占后当前实例位置与一致的实际 Ready/Running 状态。用户授权按依赖自主实施、验证、独立复核及本地提交，完成后继续下一 story。

## Boundaries & Constraints

FreeRTOS 仍是唯一 ready 队列、Running 和上下文调度所有者；仅增加有限单核 ready 政策及原子转换补丁，不重写 CPU 上下文或建立另一套 runnable 选择器。不得以只有激活计数、终止后无条件尾插代替请求记录。固定内核原件不可修改；生产补丁应用于复制件。Basic 多激活和 Extended 单激活分别验证；拒绝必须保留原状态。继续保留真实栈和已有生命周期回归。完整 Finish/Chain 出口、资源、事件与等级出口由其负责 story 闭合，不提前宣称完成。

## I/O & Edge-Case Matrix

| 场景 | 判定 |
| --- | --- |
| A1,A2,B1；A1 被高优先级 H 抢占 | H 执行后恢复 A1 原位置，随后 A2、B1；每个请求恰一次 |
| A1,B1,A2；同样抢占 | H 执行后恢复 A1，随后 B1、A2；不能与上一序列混同 |
| AABB/ABAB | 独立 4.2 literal FIFO 预期保持不变 |
| Basic activation limit | 当前及所有 pending 请求计入上限；超限返回 E_OS_LIMIT，无队列或入口状态变化 |
| Extended activation limit1 | 已激活时再次请求 E_OS_LIMIT，无重复实例 |
| 非法 TaskID / 不允许调用层级 | E_OS_ID / E_OS_CALLEVEL，原状态不变 |
| GetTaskState(valid,NULL) | 按 SWS_Os_00566 返回 E_OS_ILLEGAL_ADDRESS；不写输出 |
| 独立观察转换边界 | 请求/当前实例档案与真实内核状态一致，无可见半转换，无宿主轮询排序 |
| 来源/回归 | 固定原件摘要不变，补丁闭包有身份；4.3/4.9 与旧目标仍通过 |

</frozen-after-approval>

## Code Map

- runtime/os/src/Os.c：公开标准包装与静态目标校验。
- runtime/os/src/Os_Backend.c：静态 TCB/线程栈、bootstrap、唯一状态查询及关闭所有者。
- runtime/os/include/Os_Target.h：当前仅 ID/entry/priority/autostart；新增静态任务种类及激活容量。
- third_party/freertos/tasks.c：现有同级 round-robin selector 和尾插 ready 政策必须在复制件补丁中更换。
- runtime/os/patches/0003-activation-ready-policy.patch：新增有限请求序号 ready 排序策略；不修改原件。
- scripts/epic4_os.py、runtime/os/tests/activation.c：独立进程、literal 轨迹、边界 observer 与正式 Rust 入口。
- docs/assurance/epic4/sources.json：维护后续生产补丁清单；4.2 历史证据保留所属提交。

## Implementation Decisions

每个 Task 有静态有界的完整请求记录，每个成功接纳请求保留独立存活记录与 uint64 顺序标记。当前及 pending 记录保留池槽位置和相对顺序；顺序标记不是跨整个生命周期的不可变身份。到达计数上界时，在同一临界事务内压缩全部存活标记并更新内核排序键，保留相对顺序后继续接纳合法请求，不把 bookkeeping 溢出变成永久服务拒绝。内核 TCB 的 ready 排序键对应最早尚未完成请求；任务结束后按下一条已保存顺序号重新定位，不无条件尾插。各优先级的实际 FreeRTOS ready 列表按顺序号稳定排序，同级恢复选择保留头部，优先级选择仍由内核进行。

内核策略接口仅更新排序键/ready 列表，不解释汽车服务或另选 runnable。前置校验、请求记录、内核队列及重新调度请求保持同一临界事务。汽车状态从唯一 backend 实际状态读取；独立 observer 核对记录与实际 TCB/队列，不能用返回值自身作为预期。必要的最小实例完成接入用于验证多激活，4.5 继续完成公开 Finish/Chain 及拒绝原子性，不能用测试器代替运行任务选择。

## Tasks & Acceptance

- [x] 静态 Basic/Extended 激活容量及完整有界请求记录、拒绝路径。
- [x] 有限内核 ready 政策补丁，保留每次请求 FIFO 与被抢占实例位置。
- [x] 标准 ActivateTask/GetTaskState 接入及原子状态转换。
- [x] 无第二调度器的实际入口执行、独立 observer 与正反向 C99 harness。
- [x] 注册并恰选 epic4_activation_fifo；4.3/4.9、旧目标回归、完整增量门及三视角复核。
- [x] 同步 sprint、保存实际证据并本地提交；不推送。

Given 同级 A/B 请求顺序不同且 A1 被 H 抢占，When 实际固定内核运行，Then 每个实例恰一次、恢复当前帧且后续顺序按请求区分。

Given 容量或调用契约不满足，When ActivateTask 拒绝，Then 返回标准状态且完整请求档案、内核队列和原运行实例保持不变。

Given 转换边界独立 observer，When 查询状态和请求记录，Then 与实际内核一致且未观察到半转换；没有宿主轮询或软件 ready 选择器。

## Verification

正式入口 epic4_activation_fifo，独立预期源 core/tests/fixtures/epic4_oracles/os.json。20 个行为向量、独立复核与最终适用增量门已通过；完整等级和交接仍按后续 stories 闭合。

## Review Log

初轮三视角复核已完成，逐项处置见 Review Triage Log；当前进行修复闭合复核。


## Implementation Progress

Implemented static Basic/Extended limits and complete per-request queues, uint64 live-order stamps, standard ActivateTask/GetTaskState, and minimum nonreturning TerminateTask through a private same-native-thread C99 trampoline. The only runnable selector remains FreeRTOS. The copied-kernel 0003 patch orders ready lists by oldest pending request and retains the preempted head; ready membership refines kernel state during completion. ISR services request the existing selector at ISR exit without requiring a private callback to return a scheduling decision.

The formal `activation.c` harness passed18 native vectors: AAB/ABA/AABB/ABAB with H preemption and preserved local frame, Basic and Extended Ready/Running limit refusals, invalid ID, overflow,32-request capacity,48 entries across ring wrap, actual ISR activation, and4 configuration refusals. Its private observer caches native TCB handles during startup, then only reads actual kernel state/keys and complete automotive records at transaction/switch boundaries. Task-level handle lookup from an ISR observer initially stalled; that observer misuse was removed, not masked. Temporary diagnostic watchdog and superseded private policy probe/source evidence were removed. No host polling controls execution order.

Exact formal Rust entry selected one test and passed. Lifecycle46 and actual stack23 vectors still passed. GetTaskState NULL now reports SWS_Os_00566 E_OS_ILLEGAL_ADDRESS; calls outside legal execution context reject E_OS_CALLEVEL instead of the early foundation's target-phase E_OS_STATE. The old host-v1 runtime/API is untouched. Rejection snapshots compare declared fields rather than C padding; independent H ordinal verifies rejected requests consume no identity.

4.5 still owns complete Finish/Chain behavior and error atomicity; resources/events/Hook/ISR/full capacity exits remain their planned stories. Event wakeup in4.7 must account for OSEK ready placement separately from live activation record identity; the current no-Wait observer's equality between activation and ready keys must not be generalized blindly to that later lifecycle.

## MISRA Baseline and Current Check Scope

Applicable repository skill baseline is MISRA C:2012 Third Edition + AMD1-AMD4 + TC1-TC2, language C99. Roles are private host test/observer, native backend, standard OS wrapper and adopted-kernel policy extension. No licensed MISRA original was found under repository docs/official. No full-rule/category conformity claim or agent-approved deviation is made. Additional partial analysis of actual target translation units is retained separately; tool/source/preprocessing/cross-unit issues require explicit triage, not suppression or whole-vendor rewriting. Full applicable static-quality exit remains4.20.

## Verification Progress

- `epic4_activation_fifo`: exactly1 selected test passed;18 native vectors including32/48-entry boundaries and actual ISR.
- Lifecycle46 and stack23 native vectors passed after API and kernel changes.
- Independent verifier mutation tests reject FIFO swap or observer removal;3 OS verifier tests passed.
- Incremental hygiene/format/C99 syntax passed. Full increment and independent three-lens review pending.
- Actual proof: docs/assurance/evidence/epic4/activation-fifo.json. Source identities are preserved by LF attributes except fixed patch bytes; vendor originals remain unchanged. 4.2 historical proof belongs to0163d12 and is not overwritten by current production registry changes.

Additional partial Cppcheck2.21.0 analysis covers actual copied/patched tasks.c, Os.c, Os_Backend.c and activation.c with win64 model and target macros. Full diagnostics retained at docs/assurance/evidence/epic4/activation-static-analysis.json; missing Windows model and adopted-source findings are not marked passed. Tool-reported setjmp/host-I/O identifiers need authoritative rule/role/deviation audit in4.20; no approvals fabricated.


## Review Triage Log

| Finding | Verdict | Evidence and route |
| --- | --- | --- |
| Blind1: hidden counter exhaustion permanently fails valid activation | medium | Demonstrated by original max-boundary vector; non-frozen allocator specification amended and allocator re-derived with bounded live-order rebasing. bad_spec. |
| Blind2: overflow test locks in invalid permanent refusal | medium | Replaced with successful post-boundary acceptance and multi-task rebase FIFO preservation. Same allocator cause; bad_spec. |
| Blind3: Windows ISR critical-section contract not established | medium | Actual port vPortEnterCritical owns recursive interrupt-event mutex; vPortExitCritical checks xInsideInterrupt before waiting. Document/assert the fixed port contract and real ISR path; patch. |
| Blind4: ISR activation refusals not exercised | medium | Added ISR invalid-ID/full-limit snapshots and later accepted B ordinal4, plus private callback returnsfalse while backend schedules at exit; patch. |
| Blind5: failed calls cannot expose global ordinal consumption | false | Already in reviewed diff: task_h checks its accepted ordinal equals worker requests plus two autostarts plus H, including limits/invalid cases. Additional ISR accepted B ordinal now also verifies this. |
| Blind6: same-priority autostart ordering untested | medium | Original claim only monitor autostarts is false (L and M do), but same-priority coverage absent. Added A/B autostart vector and literal ranks1/2; patch. |
| Blind7: ready cursor movement not exercised | false | Fixed config requires generic head selector and no slicing; selector never rotates pxIndex, insertion restores it, no supported caller moves it. A hypothetical alternate selector is explicitly rejected at compile time. |
| Blind8: WAITING state relationship untested | false | Wait/Wake service is expressly assigned4.7; no current supported input reaches eBlocked. Kept integration note to validate wake placement separately there; no premature claim. |
| Blind9: kernel resume success assumed | medium | vTaskResume is void and xTaskResumeFromISR returns yield-needed, not success. Valid zero-count transition must have suspended actual TCB; enforce that invariant before mutation; port failure remains fatal and existing stack-operation refusal vectors pass. patch. |
| Blind10: changed-line static diagnostic disposition missing | medium | Partial scan stays red; add scoped triage, retain unverified rule categories and explicit4.20 gate. No suppression/approval fabricated; patch. |
| Edge1: closing between readiness check and critical entry | medium | Recheck/guard inside transaction and at service boundary; accepted-prefix writes have admission linearization, closing prevents further admission/normal actor return. Real normal-close injection proves zero new A requests; fault closing stays lock-free, never waits for ordinary IRQ mutex. patch. |
| Edge2: observer before deferred completion switch | medium | Removed pre-exit completion observation; kernel switched-in callback observes committed scheduling boundary, while entry traces prove fresh-frame execution. patch. |
| Gap reviewer | none | No verification gaps filed; exact15 native-vector entry independently inspected. Repair vectors now18 and require closure review. |

## Spec Change Log

Iteration1: allocator hidden ordinal exhaustion contradicted valid-service behavior. Non-frozen live-order definition now permits atomic rank normalization; sequence numbers are not eternal identities. KEEP: full per-request slots, Basic/Extended limits, request FIFO, sole kernel selector, native-frame trampoline, no-return checks, independent actual-state observer, real ISR path and lifecycle/native-stack regressions. Re-derived allocator and boundary oracles from that corrected specification; frozen human intent unchanged.

Current repairs passed18 native activation vectors,46 lifecycle and23 stack vectors. Rebase proof accepts after UINT64_MAX and preserves ABAB across normalization. Normal-close race is injected after cached readiness but before critical entry; target actually enters its existing closing state, zero new A requests and no service return/application marker X. The bounded close-delay hook is test-only and absent from production/native-fault harness. Independent repair review and final gates pending.


## Repair State

18 native activation vectors pass after repairs. Windows ISR critical operations use the selected port's recursive interrupt-event mutex and xInsideInterrupt-aware exit behavior; no generic MCU ISR-safety claim is made. Rejected IRQ calls now cover invalid ID and full activation limit, with a later accepted B order4. Same-priority A/B autostart is observed. The internal resume precondition is asserted before mutation; the boolean FromISR result only requests a switch.

Closing rechecks admission inside the transaction, and guarded service boundaries stop automotive actors once closing is observed. Management shutdown publication remains lock-free; no normal interrupt mutex is acquired by fault shutdown. A test-only delayed normal-close controller produces the exact previously reported pre-lock race; no A request is published and no X application-return marker appears. The delay is absent from production and native fault harness.

Order normalization sorts metadata references only, then updates the existing kernel keys; it never selects a runnable task. It preserves every live pool record and all relative ordering, and there is no lifetime-unique-ID promise in the private diagnostic stamps. ABI-independent snapshot comparisons avoid padding. Completion observers now run on the actual kernel switch boundary.

Remaining before4.4 acceptance: regenerate and triage changed-source partial static analysis, independently re-review repaired implementation, run final applicable gates, capture final evidence, synchronize sprint and commit. No applicable static-quality or deviation approval is presumed closed; full artifact-quality exit4.20 remains required.


## Final Evidence and Static Diagnostic Disposition

The current partial scanner's complete output and changed-source dispositions are preserved in activation-static-analysis.json, including tool/addon and source hashes. Easy new precedence/scope/ignored-output issues were repaired. Header/API macro and Windows model findings are separated from real library/deviation candidates; nothing is suppressed or converted into a conformity pass. Native backend trampoline deviation draft E4-HOST-RESTART and host test I/O draft E4-HOST-REPORT remain unapproved, with final source/category/approval closure required at4.20. Existing adopted-source diagnostics are retained, not rewritten outside scope.

External pinned OSEK PDF and kernel archive now have documented, ignored stable docs/official locations rather than research-session temporary defaults. The formal baseline entry passed with these defaults; compiler invocation spelling is separated from pinned vendor/version/target/executable SHA identity. New shared evidence is a labeled path-normalized representation linked to ignored raw structured records by SHA; physical-stack addresses/thread IDs remain for diagnostic correlation. Historical4.1/4.2/4.3/4.9 evidence is preserved.


## Atomic Admission Repair

Repair closure edge review found an actual narrower race after the in-transaction Ready check but before metadata append. Added one shared Interlocked state word: bit0 is an accepted activation transaction and bit1 permanently closes admission. Successful CAS0->1 is the activation linearization point; shutdown atomically ORs bit1 and never waits for in-flight work or the IRQ mutex. A close-first CAS failure writes no metadata; an admission-first operation belongs to the accepted prefix and may finish publishing its record, while the existing closing port guard prevents resuming execution and the service cannot return to application. Clearing bit0 preserves a concurrently set close bit. Real fault closure uses the same gate without allocation/locking, and report reason derives any captured stack fault to preserve fault escalation.

New close-ready vector injects close after the Ready/capacity checks but before CAS and observes zero new pending requests. New close-accepted vector injects after successful CAS and observes exactly one accepted-prefix record; both observe no A entry or X application return. Together with the prior pre-lock close-admission vector, current activation suite passes20 native vectors. Blind closure review found no actionable findings and verified current source hashes/36 diagnostic dispositions; gap review selected the exact formal Rust entry and found no gaps. The narrower edge repair requires scoped closure and final regression.


## Acceptance Closure

Final atomic-admission closure reviewer found no remaining gap. The three original independent lenses and scoped repair closure are complete; no actionable finding remains for this story. The current shared activation and partial-static artifacts match all product source hashes. Current changed-source static dispositions cover41 entries, none silently suppressed or reported as conformity; full quality/deviation closure remains explicitly pending4.20.

`python scripts/verify.py --scope all --base 0163d1259a47b650a318048abe35a4995caa7037` passed: seven capability records unchanged,25 Python tests, source/changed-line/C99 checks, UI lint/build,3 core unit and63 integration tests, core clippy, desktop build/clippy. A subsequent explicit numeric fault predicate (no behavior change) passed activation20, stack23, lifecycle46 and incremental quality again. Exact formal entry was independently selected by the gap reviewer; final all gate included its expanded20-vector suite. No visible desktop/IPC test or full MISRA/SC1 claim is implied.

Sprint4.4 is done at the verified story boundary; Epic4 stays in-progress. This focused increment is committed locally with no push, then4.5 proceeds from that canonical commit. No W3 gate is opened by this acceptance.
