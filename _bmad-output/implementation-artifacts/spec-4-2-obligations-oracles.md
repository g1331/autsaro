---
title: '4.2 完整义务、依赖来源与独立预期'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'b01a4082e2389449c0662d3b6df13bad669705a1'
story_key: '4-2-固定标准义务-依赖来源及独立验收预期'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

把完整适用 SC1/参考集成义务、来源及权利边界和独立 CAN/UDS/OS 预期固定为后续实现及最终复核的共同基线。每项绑定版次/条款、配置、生产者、负责 story 和未来证据；未来功能没有实际证据时保持 not_run。用户已授权逐 story 自主推进、本地提交，无逐条确认。

## Boundaries & Constraints

固定 FreeRTOS 11.3.1 commit、Windows x64/GCC 16.1.0，不比较 OS。单核 SC1/Extended Status、全部 BCC1/BCC2/ECC1/ECC2、至少八软件 Counter/两个 ScheduleTable、真实栈、ISR/Hook/错误和 ARTI 必须逐项追踪。参考实例不配置某项不等于其等级能力 N/A。官方原件仍为外部资料，不再分发；研究补丁与生产补丁分列。原创 4.1 输入是样例配置权威；不修改已声明 CAN/DID/周期值来适配旧 runtime，不扩展 Epic 2/5/6，不推送，不冒称标准认证或 MCU 能力。

## I/O & Edge-Case Matrix

| 项目 | 独立判定 | 证据状态 |
| --- | --- | --- |
| R24-11 OS 与 OSEK | API、配置/错误、Hook、等级容量及 ARTI 全部适用义务逐项可定位 | 未运行行 not_run |
| 条件 N/A | 明确规范条件与目标配置事实；保留等级能力测试 | 独立复核 N/A 理由 |
| 内核/工具/许可 | 固定归档及关键原件摘要，原始许可与来源；生产补丁单列 | 不接受浮动源码或未确认再分发项 |
| CAN/DID | 精确输入、epoch、SF/UDS/CAN 输出字节与初值/30 ms 边界 | 预期来源为契约与规范，不由生成器计算 |
| 诊断计时/失败恢复 | 纳入选定 P2/P2*、S3、CanTp N 定时，区分各计时意义及同刻输入规则 | 未闭合的响应/计时不得凭空定向量 |
| OS FIFO/服务错误/栈 | 两种请求顺序、原子拒绝、真正 E_OS_STACKFAULT 关闭 | 不共享被测调度算法 |
| 缺资料或摘要失配 | 明确 fail/not_run，不跳过计为成功 | 阻断对应基线与交付门 |

</frozen-after-approval>

## Code Map

- planning architecture/epic-4/R24-11-CONTRACT.md：已有义务族，明确不是完整符合性清单。
- core/tests/fixtures/epic4/：4.1 原创配置/摘要/语义与拒绝基线。
- third_party/freertos/source-manifest.json 与 runtime/os/patches/：当前生产闭包；planning feasibility/source-manifest.json 为研究身份记录。
- docs/official/R24-11/CP/SystemServices/AUTOSAR_CP_SWS_OS.pdf 及对应 Dcm/CanTp/RTE/模板资料：固定本机外部权威资料。
- docs/assurance/epic4/：新增义务矩阵与依赖/权利档案。
- core/tests/fixtures/epic4_oracles/：新增独立向量，保持 4.1 原始文件集合不变。
- scripts/epic4_obligations.py、core/tests/end_to_end.rs：完整性/来源/独立预期入口 epic4_obligation_and_oracle_baseline。

## Tasks & Acceptance

- [x] 固定外部规范摘要/页码索引，完整盘点 OS/OSEK 及所选集成义务，逐项判定适用性与条件 N/A。
- [x] 每条义务绑定配置、源码/未来生产者、负责 story、精确未来测试及 not_run 状态；保留所有等级容量和一致性类维度。
- [x] 核对固定依赖归档、原件、许可和 ABI，研究/生产补丁分列，声明外部资料及未确认权利的交付边界。
- [x] 固定独立 CAN/UDS/OS 正反向量及诊断计时条件，不由实现推导预期；按实际配置核对全部字节与定时边界。
- [x] 注册并执行恰一个正式入口；全量增量检查、三视角独立复核、同步 sprint、本地提交。

Given 已核官方资料和原创配置，When 建立义务矩阵，Then 每项有版次/条款/配置/生产者/story/test/status，未运行保持 not_run，并保留完整 SC1 能力门。

Given 固定内核和工具，When 复验来源闭包，Then 原件/许可/归档/补丁真实摘要一致，构建不取浮动源码，交付权利缺口明确阻断对应打包项。

Given 独立协议和 OS 输入，When 核对向量来源，Then 精确字节、FIFO、错误、epoch/计时边界来自已确认契约和规范，生成器不能重写预期。

## Implementation Notes

4.1 已本地提交 b01a408，输入基线保持独立；4.3/4.9 有实际基础证据，不能据此把完整 OS 义务标 pass。已完成两项只读资料盘点；主代理仍须核验关键条款、完整 §8/API/ARTI 和诊断 NRC/计时条件。

已按实际正文位置生成 595 条当前 OS 要求/约束的来源索引；只出现在 change log 中的删除条目不计作当前义务。矩阵加入 28 条具体等级容量、31 个 OSEK 服务、10 个语义族及 27 个选定集成条款，共 691 行；已纠正按整个 service-protection 章节 N/A 导致遗漏的 core cleanup/disabled-interrupt/missing-end 义务。适用性仍待完整独立复核。

sources.json 固定并实测核对九份 AUTOSAR PDF、OSEK PDF、FreeRTOS 归档/27 个原件/MIT、两项生产补丁与研究材料和 GCC 身份。未来生成二进制的 runtime imports/notice 审计保留 not_run，并明确未关闭前阻断对应打包。

独立预期初稿包含 11 个协议字节向量、6 个计时边界和 8 个 OS 向量。按实际 MOD/SWS 补齐 ErrorHook 信息开关、1 ms 主函数周期、零填充、单请求最多两个 DID，以及 0x10/0x3E 的 DSD 子服务。重复同一 DID 形成 13-byte 响应；未增加应用 C/S 操作。更新输入后恰一个 epic4_reference_input_baseline 回归通过，七文件边界保持，原始摘要已刷新；4.2 正式入口已恰选一项通过；未来目标行为仍未执行，保持 not_run。

## Review Log

首轮三视角独立复核已收齐；修复后正在复核闭合，不得提前标 done。


## Review Triage Log

| Finding | Verdict | Evidence and route |
| --- | --- | --- |
| Blind 1: repeated-DID FF/CF bytes inconsistent | false | Actual FF is `10 0D 62 12 34 12 34 56`, not the truncated value quoted by the reviewer. Six FF payload bytes plus seven CF payload bytes reconstruct exactly `62 12 34 12 34 56 78 12 34 12 34 56 78`; 13-byte UDS payload is correct. |
| Blind 2: timing cases unchecked | medium | Previously only counted; lock each complete source-backed timing case and inventory; patch. |
| Blind 3: other OS expected statuses unchecked | medium | Previously only three anchors; seal all complete OS cases and mutation-test E_OS_LIMIT; patch. |
| Blind 4: missing positive event/resource/Schedule oracles | medium | Added independent Extended wait/wake and already-set event, nonpreemptive Schedule and ceiling/release traces with OSEK sources; patch. |
| Blind 5: non-OS row omission undetected | medium | Existing capacities were guarded but services/selected clauses were not; pin non-OS inventory and reviewed trace; patch. |
| Edge 1: SWS_Os_00763 incorrectly excluded | medium | Primary PDF page88 explicitly requires Standard Status to be possible in SC1 despite its location; marked applicable and mandatory. Reference remains Extended; patch. |
| Gap 1: other ARTI/API applicability waiver passes | medium | Pin the whole reviewed applicable-key set and trace; mutation-test SWS_Os_00838 and SWS_Os_00763; patch. |
| Gap 2: selected integration omission passes | medium | Pin reviewed non-OS inventory; mutation-test removing SWS_Rte_08104/OSEK WaitEvent; patch. |
| Gap 3: other bytes and timing change passes | medium | Complete case fingerprints include input, expected result and provenance; mutation tests cover another NRC, timing and OS status; patch. |

## Verification

- `python scripts/verify.py --scope all --base b01a4082e2389449c0662d3b6df13bad669705a1`: first complete run passed (20 Python tests, 3 core unit tests, 62 integration tests, zero failures/ignored; UI lint/build; core and desktop clippy; desktop build; incremental quality; 7 capability gates unchanged).
- Repair mutation tests: 7 passed. Reviewed-baseline seal is a reviewed artifact, never regenerated by a build, parser or target implementation. A later change requires a source-backed independent baseline review.
- Exact registered entry and final repair checks pending. Future OS/protocol behavior and binary import/rights audit remain not_run; baseline acceptance does not imply target compliance.


| Repair review finding | Verdict | Evidence and route |
| --- | --- | --- |
| Blind 6: Alarm service oracles absent | medium | Added source-backed relative wrap, absolute cyclic alarm/cancel, base/remaining/error and preserved-deadline traces; patch. |
| Blind 7: ordinary ScheduleTable/capacity oracle absent | medium | Added two NONE-synchronized nonrepeating tables on independent counters, expiry/stop/state observations and six unchanged counters; patch. |
| Blind 8: interrupt-control/ISR oracles absent | medium | Added nested All/OS restore and separate nonnested Disable/Enable, category gating, ISR call-level rejection and exit scheduling observations; patch. |
| Blind 9: Hook oracles absent | medium | Added initialization/Task-state Hook ordering, ErrorHook metadata/no-recursion, successful-service exclusion and normative no-return shutdown trace; patch. |
| Blind 10: stack triggers insufficiently distinct | medium | Added separate actual native reserve-boundary and saved-Rsp corruption oracles, independent shutdown/control conditions; patch. |
| Blind 11: pending review metadata/evidence do not imply accepted behavior | false | Metadata intentionally remains pending during review and evidence states future behavior not_run. It is closed only after final independent acceptance; no target behavior is claimed. |
| Gap repair: NON resource precondition ambiguous | medium | OSEK section4.6.2/page22 and section8.7/page34 specify special internal resource; made setup/release/reacquire explicit, patch. |
| Edge repair 1: SWS_Os_00107 incorrectly excluded | medium | Primary page83 requires no-ProtectionHook fallback ShutdownOS on a protection error. Actual SC1 stack errors trigger that condition; marked applicable and mandatory; patch. |
| Edge repair 2: CanTp main period not bound | medium | Bind configured CanTpMainFunctionPeriod to logical tick; mutation test demonstrates changed value fails; patch. |
| Edge repair 3: App alarm cycle not bound | medium | Select unique Alarm_App and bind its cycle in logical ticks to application_period_ms; mutation test demonstrates divergence fails; patch. |
| Edge repair 4: Com TX period not bound | medium | Bind ComTxModeTimePeriod to tx_period_ms and mutation-test divergence; patch. |

All accepted repairs remain within frozen intent. No implementation generator or tested scheduler derives these literal expectations. OS oracle count is now20; future execution remains not_run. Repair checks:8 tests passed. Three original gaps were independently confirmed closed; final added-case/source repair review pending.


| Final repair finding | Verdict | Evidence and route |
| --- | --- | --- |
| Blind final: SWS_Os_00396 hook branch incorrectly applicable | medium | Primary page56 conditions that branch on a configured ProtectionHook. Target has none; record conditional N/A and remove mandatory assertion. Mandatory actual stack monitoring and ShutdownOS remain under 00067/00068/00107; patch. |
| Gap final: stack oracle source conditions inconsistent | medium | Explicitly declare no ProtectionHook in generic case, remove 00396 source from both no-hook vectors; preserve real native boundary/Rsp checks and mandatory shutdown; patch. |
| Edge final: SystemCounter duration not bound | medium | Bind unique SystemCounter OsSecondsPerTick to logical tick, mutation-test divergence; also seal all original fixture source hashes and manifest so changed reviewed configuration cannot silently validate; patch. |


## Completion

Three independent review lenses confirmed repaired closure; no unresolved findings deferred. Final full incremental gate passed:24 Python tests,3 core unit tests,62 integration tests,0 failed/ignored; UI lint/build,core/desktop clippy,desktop build,quality and7 unchanged assurance gates. Registered `epic4_obligation_and_oracle_baseline` selected exactly one test and passed after accepted review metadata. Evidence is `docs/assurance/evidence/epic4/obligation-oracle-baseline.json`.

Raw JSON source/evidence input hashes are preserved across fresh checkouts by LF attributes and normalization; canonical case/trace seal is unchanged by whitespace. The baseline is accepted, but every future target obligation remains not_run and binary runtime-import/rights audit remains a packaging gate. Epic4 remains in-progress; 4.2 completion does not imply SC1 or final engineering handoff acceptance.
