---
title: '4.6 非抢占、内部资源及资源上限恢复'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'ac1193c3deb6e1d0c20e9ceda7b4f4e845828c2d'
story_key: '4-6-实现非抢占-内部资源及资源上限恢复'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous per-story execution">

## Intent

FULL/NON、内部资源、外部资源 ceiling/LIFO 和显式调度点遵循汽车 OS 契约。实际抢占和恢复位置由唯一固定 FreeRTOS backend 实现，禁止用全局调度锁或 mutex 优先级继承代替逐任务属性和规范顺序。

## Boundaries & Constraints

依据 OSEK2.2.3 §4.6、§8.2–8.7、§13.2.3.4 和 §13.4，维持 E4-AR-6。内部资源在进入 Running 时取得，抢占不释放，Schedule/真正等待/结束时释放并按规范重新取得。源持外部资源时拒绝等待/结束/调度，完整前态保留。分别证明 Task 与模拟 Category2 ISR 资源核心；完整中断/Hook 义务仍归4.18。等待边界必须有实际阻塞/唤醒，4.7 再闭合事件所有权与全部竞态。源原件不改、不引入第二 runnable 选择器、不扩展旧目标或 W3。

## I/O & Edge-Case Matrix

| 场景 | 预期 |
| --- | --- |
| FULL 任务激活更高优先级任务 | 合法立即抢占，恢复原实例/位置 |
| NON 激活更高优先级任务 | 保持执行到显式点；Schedule 释放隐式内部资源，返回前重新取得 |
| 内部资源组、组外高优先级抢占 | 取得时机正确，抢占后保留，显式点释放并重取 |
| 嵌套或同 ceiling 外部资源 | 有效优先级逐层恢复；同级 A/B 与 H 顺序不退回研究反例 |
| 逆序/非所有者/重复资源操作、持资源等待或结束 | 标准 E_OS_NOFUNC/ACCESS/RESOURCE/CALLEVEL，档案与原实例不变 |
| 真实 Category2 ISR 资源与 Task/ISR 共享 | 静态访问、所有权、优先级/屏蔽与释放正确；不执行禁止调用 |
| 外部资源与内部资源组合 | 外部资源必须先释放；不能让 Schedule/Wait 破坏持有关系 |

</frozen-after-approval>

## Code Map

- `runtime/os/src/Os_Backend.c`：复用4.5静态所有权/LIFO、完整请求及实际有效优先级；抽出统一优先级/内部资源转换。仅对内核已选出的当前 Task 获取内部资源。
- `runtime/os/include/Os_Target.h`、`src/Os.c`：FULL/NON、最多2个配置内部资源及静态访问/ceiling关系校验；保留16任务/8外部资源基础。
- `runtime/os/FreeRTOSConfig.h`、复制件策略补丁：在真实 Running 边界执行必要政策；保留既有线程上下文和唯一 selector。Category2 资源的模拟中断抑制与释放必须在真实端口边界验证。
- `runtime/os/include/Os.h`、`src/Os_Backend.h`：Schedule 及必要等待边界；成功的 Terminate/Chain继续使用已验证根帧。
- `runtime/os/tests/resource_preemption.c`、`scripts/epic4_os.py`、`core/tests/end_to_end.rs`：独立 literal 顺序、优先级/实际内核观察，注册 `epic4_resource_and_preemption`。
- `docs/assurance/epic4/sources.json`：记录生产复制件补丁身份，不覆盖历史证据或固定原件。

## Tasks & Acceptance

- [x] 静态调度/内部资源配置与拒绝；在真实进入 Running 时取得内部 ceiling。
- [x] Schedule、实际等待与结束边界释放/重新取得，FULL/NON和资源组分别验证。
- [x] 完善外部资源恢复位置、嵌套/LIFO拒绝及实际 Task/Category2 ISR核心。
- [x] 独立正反向向量、恰选正式入口、前序回归、适用质量门与三视角只读复核。
- [x] 保存真实证据，同步 sprint，本地提交后继续4.7；不推送。

Given 混合抢占和内部资源，When 到达真实调度/等待/结束边界，Then 释放及重新取得时机与独立规范预期一致。

Given A/B同级与H、嵌套外部资源，When 获取及LIFO释放，Then 内核有效优先级与当前实例位置正确恢复。

Given 持有关系或调用层级不合法，When 服务拒绝，Then 标准错误与完整前态可复验，不发生无条件结束或丢失请求。

## Design Notes

内部资源由当前 TCB 的真实 Running 转换取得，不在激活时预先抬高优先级。NON 采用隐式最高汽车任务优先级的内部资源；显式资源组保持自己的静态 ceiling。外部资源与内部资源分别记录，不能把内部持有当 E_OS_RESOURCE 拒绝。

等待实现须原子建立真实阻塞与内部资源释放，唤醒只驱动已有内核就绪性，当前实例帧保持。4.7将独立证明存储事件、唤醒顺序及所有权，不能用默认 notification/mutex行为作等价证明。Category2 ISR核心明确固定 Windows 端口的可验证边界，4.18继续完整中断义务。

## Implementation Notes

4.5已提交，基线和先行验证可复用。首次构建受D盘空间限制，桌面已在独立临时 Cargo target、关闭增量缓存的同一检查下通过；后续按实际空间采用该缓存。自动审批拒绝过缓存删除，不再次尝试或绕过；没有删除源码或构建缓存。

## Spec Change Log

## Review Triage Log

三路独立只读复核已完成：blind 检查配置校验、FULL/NON、Schedule/Wait、LIFO/ceiling、RES_SCHEDULER 和实际端口挂起/屏蔽；edge 返回 complete 且无发现；verification-gap 未发现验证缺口。edge 首次 partial 返回未作为完成依据，续接后取得 complete。没有待分级的具体发现，没有为满足数量要求添加问题。

## Verification

`cargo test --manifest-path core/Cargo.toml epic4_resource_and_preemption -- --exact --nocapture` 必须恰选一项。Finish22、activation20、stack23、lifecycle46及适用增量门以最终代码验证，partial MISRA扫描保留诊断/偏离状态。当前保持in-progress，全部适用出口前不标done，不放行W3。


## Implementation Progress

Implemented per-task FULL/NON and two static internal resource definitions, acquired only on the kernel-selected Running callback. Schedule/actual wait/completion release internal priority; same instance resume reacquires it. Static ceilings and refs reject invalid configurations before threads. External nesting/LIFO/priority restoration remains the actual kernel mapping; reserved eighth slot implements RES_SCHEDULER without a hidden ninth resource.

The minimum real Wait/Clear/wake seam supports the internal release boundary. Native suspended-list membership plus a retained live activation/predicate is an explicit automotive WAITING mapping; ready-order stamps are separate from activation records after wake. Rank rebase includes both with equal old stamps mapped to equal new ranks. Full ownership/race/Wake exit remains4.7, not presumed complete.

The fixed Category2 group has real current ISR identity, independent ISR resource LIFO and a virtual31 pending gate. Source tracing exposed the inherited Windows port's application suspension only when switching: normal Windows priority could allow app execution during externally injected ISR. Re-derived0005 with existing checked suspension/context/resume covering the whole callback interval; no CPU context rewrite or secondary selector. An external producer plus a bounded ISR atomic-progress probe passes, alongside actual mask/pending/release fixtures. Return with leaked ISR resource fails visibly; full lifecycle cleanup remains4.18.

25 native vectors pass, including exact4.2 NON/ceiling priorities, two observable internal ceiling groups, external+internal refusal, reserved scheduler IRQ permissiveness and8 configuration negatives. Exact Rust resource entry selected1 and passed before the last reserved-scheduler vector; final entry/gates still pending. Previous activation20,finish22,stack23,lifecycle46 passed after the main backend/port changes. Scanner covers actual copied tasks/port, backend/wrappers/native stack/event adapter/harness. All changed-source diagnostics are dispositioned; adopted-source/tool model issues remain partial/red, final4.20 approval and full quality open.

Independent review complete; final full increment acceptance remains in progress. Exact Rust resource entry now selects one test and passes all25 vectors. Final source hashes in both resource evidence records match every current product input;64 changed-source diagnostic dispositions contain no unresolved row, while the complete MISRA quality exit remains open. No visible desktop/IPC or full SC1 claim. Separate Cargo cache closes the earlier disk constraint without deleting refused cache.

最终增量门通过：`python -B scripts/verify.py --scope all --base ac1193c3deb6e1d0c20e9ceda7b4f4e845828c2d` 退出0，包含27 Python测试、源码卫生/新改行格式/C99、UI lint/build、3 Rust单元及65集成测试、核心与桌面 clippy、桌面 build。同一命令使用已有 C 盘 Cargo target，dev/test incremental=false；没有删除缓存。先行正式 exact 测试恰选1项通过，前序 OS 原生向量通过全量入口复验。上述完整门替代此前“仍在进行”的阶段状态；独立复核完成，4.6可提交。完整 SC1、MISRA出口及隔离桌面 IPC 仍须后续故事闭合，Epic4保持in-progress，W3未放行。
