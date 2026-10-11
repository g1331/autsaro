---
title: 'Story 9.2：CAN 标准视图与主机适配边界'
type: 'refactor'
created: '2026-10-11'
status: 'in-review'
baseline_commit: '6290f1631fa066f48db70b9f789b7e17bcd4fe63'
route: 'dispatch'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-9-context.md'
  - '{project-root}/_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md'
  - '{project-root}/_bmad-output/specs/spec-runtime-codegen-quality/generator-practices.md'
---

<frozen-after-approval reason="maintainer-owned intent — changes require authorized delegated approval or maintainer renegotiation">

## Intent

Can 标准服务、主机注入／输出混在同一公开头，真实 Rx 接缝、DeInit 和 MemMap 尚有软件缺口。让 CAN 开发者沿唯一状态 owner 维护标准视图与显式主机适配，同时保持旧 include-only 用户和真实异步发送／确认行为。目录拆分不是独立交付；覆盖 epics.md Story 9.2 原三 AC。

## Boundaries & Constraints

保持 R24-11、C99、MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2、三族实际 profile、Windows/Linux controlled target。保留 Can_Init/Can_Write 签名、Id32/HOH16、各族 Pdu 宽度、Config 单 sink 和 PDU 布局、全部旧 host 可见性；legacy 零 DLC 拒绝、既有 integration 零 DLC 接受，不用声明视图切换运行策略。

旧 runtime/include/Can.h 为 CAN_COMPAT_H 聚合；新 standard/Can.h 用 CAN_H，仅标准服务及 general/config types；Can_CfgTypes.h 保留原 CanTxSink 与单 sink config，显式依赖 Ecu_Status；Can_Host.h 单向包含 standard/Can.h。标准 consumer 显式使用标准 include 搜索顺序或 standard/Can.h，无 host/test/driver 状态泄漏，不把硬件专用 config 称规范固定布局。Can.c／私有 Can_Internal.h 唯一拥有 controller、pending、payload、lifetime sequence；host 源只有窄适配，无第二状态／队列／锁。保留真实 SchM/Can/CanIf 保护，ECU owns batch/output/receipt，OS owns 调度/Waiting/quiescence。

关闭所选配置下 CAN 与直接依赖的通用软件差距：真实 Rx 调用标准 CanIf_RxIndication，私有适配保留嵌套时间和 EcuStatus；已接纳的实际 overwrite/overrun 报 Det_ReportRuntimeError/CAN_E_DATALOST，不受 DevErrorDetect=false 禁用，可重试未接纳 BUSY 不是假丢帧；合法 STOPPED/SLEEP 直接 DeInit 清理／取消 pending，不以额外通知 guard 推迟后置条件。所有回调不在 driver 持锁区域执行，保持生命周期隔离和可重入；host 未 Init 拒绝业务，GetMode 旧查询保留。Wakeup 模式轮询兼容别名去重，无 wake 配置不伪造 EcuM/IRQ。GetVersionInfo 默认 OFF，unsupported ON 拒绝或按适用契约真正实现。公开版本／外部头一致性基于真实项目依据，vendor 身份不可编造，不能把发布宏当完整符合。

Can_MemMap 检查合法 CODE/VAR_CLEARED marker、配对及禁止嵌套；适用声明／定义 wrapping，合法反复 include，错误拒绝；默认段需实际适用依据、目标符号段／清零及描述对应，不能外推 MCU linker。同步真实生成来源、配置、所有消费者、BSWMD 与完整交付选集；旧包／seal 不重写，live 源保留，历史 profile 用真实结构识别。真正 breaking signature/layout/handle/wire 先暂停受影响变更交 PM。

不新增 profile、调度器、运行时或代理工具；不降规则、扩大排除／suppression、自批偏离、复制 licensed 原件入 Git／附件。不扩大 FD/XL/PN、多 controller、真实硬件／认证。MISRA 原文缺失只限制确需全文的裁决／完整符合性，不泛化停编码。

本卡允许阶段 commit、功能分支 push、PR 至 g1331/autsaro master 和 exact-head required CI；禁止 direct master push、force、保护绕过、部署／发行／支出。远端分支 feat/can-host-boundary（冲突再选真实功能名），本地 native 分支不改；本卡不 merge，下游 Engineer 承担 protected integration。Build 内部隔离实施／独立 review 后同卡 Tester，不能自证独立 QA 或 Story done。

## I/O & Edge-Case Matrix

| 输入／状态 | 预期 |
| --- | --- |
| Init 有效／重复／NULL；未 Init host | 有效 STOPPED；重复／无效不擦旧请求；未 Init 拒绝业务 |
| STOP/SLEEP DeInit 含通知；BUS_OFF | 清 pending／取消，无成功确认；BUS_OFF 对外 STOPPED，显式恢复，不自动启动 |
| 同 HTH 忙、锁错、复制后改原 buffer | BUSY 与 E_NOT_OK 区分且无副作用；输出入队原字节 |
| 嵌套 Rx、已接纳 overrun、未接纳 BUSY | 标准回调一次且时间／返回隔离；真实丢失报 runtime error，BUSY 不报 |
| sync／queued，错／重复／迟到 receipt | 真实 write/flush 后正确 ticket/PduId/token 恰一次确认；STOP/BUS_OFF/DeInit-reInit 后旧 receipt 不完成新帧 |
| output write／flush 物理失败、关闭 | 不回滚／伪成功；停止 admission、排空／取消、join／close、OS quiescence |
| 源 controller/HOH、可选配置非法 | 连续编号与实例数量独立拒绝，旧输入／输出保护；7/9/17 仅机制 fixture |
| legacy／标准-only／混合 include | 旧原 consumer 不改源码可用；标准无 host/test；顺序独立、类型／布局稳定 |
| MemMap 合法反复／nested／错 stop／unknown | 合法编译；非法预处理失败，无 checker 特赦 |

</frozen-after-approval>

## Code Map

- `runtime/src/Can.c:38–69,102–125,243–543`：唯一状态和 copy/token；DeInit 额外通知 guard、host Init 缺口、回调持锁与 Rx 接缝是实际修复点。`Can_HostLock.*` 为进程 once 共享锁，不能每 DeInit 销毁。
- `runtime/include/Can.h`：拆声明视图但保留兼容；新增 `standard/Can.h`、`Can_CfgTypes.h`、`Can_Host.h`，`runtime/src/Can_Internal.h` 与 `Can_Host.c` 为窄适配，不暴露 static 状态。`Can_GeneralTypes.h`／各族 Std/ComStack 维持各自闭包。
- `runtime/src/CanIf.c:165–190`、`runtime/multi/src/CanIf.c:284–305`：multi host Rx 目前绕标准回调；窄私有上下文通道接标准 void 回调，保留诊断返回与 clock。外层递归锁不能冒称已释放。
- `runtime/src/Ecu_Execution.h`、`runtime/host/src/Can_Execution.c`、`runtime/ecu/src/{Ecu_Execution,Ecu_Target}.c`、`runtime/src/{Os,LSduR,ecu_host_main}.c`、`runtime/ecu/templates/Rte.c.in`：直接 consumer 按标准／host 用途选头；Target 确认边界仍核 ticket/PduId/token，batch/probe 各 main 不混链。
- `core/src/integration/{multi_bsw,configuration,plan,catalog,artifacts,ecu,multi_ecu}.rs`：合法 controller=0、共享 HOH={0,1} 已有，反向 HRH/HTH 也需正确生成；补 source-derived RX_HOH、标准声明 owner、host source identity、runtime_sources/allowlist/BSWMD expected Rx/Det 与 memorySection。
- `runtime/multi/{include/Det.h,src/Det.c}`：当前真实 Det 在 multi；legacy runtime error 接缝须按适用依赖接入，不能凭空 stub 或把普通 BUSY 当丢帧。
- `core/tests/support/can_contracts.rs`、`native.rs`、`multi_runtime_contracts.rs` 与 `fixtures/multi-runtime/`：原标准／线程／polling／COM／诊断 token suites。Linux 缺同 HTH flush 握手，compiler 当前硬编码 gcc；用 tooling::execution_settings 的受控 compiler，不以仓库 include fallback 冒充交付闭包。
- `core/tests/support/{targets,semantic_profiles,epic4_batch}.rs`、`fixtures/host_batch_faults.c`、`tests/python/native/test_host_boundaries.py`：真实 production/receipt 和错误 oracle；故障 fixture 当前 Windows-only，sink 拒绝不是物理 write/flush 证明，NvM fclose 也不是 CAN 关闭。
- `runtime/include/Can_MemMap.h`、`core/tests/c_analysis.rs`、`tools/python/src/autosar_tooling/c_check.py`：移除无 marker fallback，真实 TU／合法 section header 验证，不扩大排除。
- `runtime/contracts/{assets-v1,bsw-v1}.json`、`tools/python/src/ecu_tools/workbench-v2-assets.json`、`prepared/resources/target`、离线 `ecu_tools`：嵌套 include 完整物化、ABI/source/seal 身份；遵循 `.gitattributes` 固定字节，不改第三方身份。
- 来源调查复用 `t_82034dad` comments73–75/77、`t_d30278ca` R2、`t_70f519a6` run41、客户 `t_b6a1f960` 三 AC、9.1 continuity。合法 PDF 只读 `/root/workspace/autsaro-reference/R24-11`，实施前核对 CANDriver/BSWGeneral/StandardTypes/CommunicationStackTypes/MemoryMapping 直接适用原文；源码与 ABI 不是唯一 expected。

## Tasks & Acceptance

- [x] `runtime/include/Can*.h`、新 `standard/Can.h`、`runtime/src/{Can,Can_Host}.c/Can_Internal.h`：完整拆视图／窄适配，修 Init/DeInit、唯一 lifetime、callback-outside-lock 与 MemMap；不改变旧 ABI/policy。
- [x] 两族 `CanIf` 私有适配、`runtime/multi/Det`、上述 execution/OS/main consumers：真实标准 Rx、嵌套 clock/status、真实 DATALOST 接缝及 explicit imports，保留 locks 和各 owner。
- [x] 上述 `integration/*.rs`、运行时清单／资源映射：配置／producer／declaration owner／BSWMD 同步，旧结构/live/seal 不漂移，版本/可选 API 以真实依据约束。
- [x] `core/tests/support/can_contracts.rs`、`multi_runtime_contracts.rs`／现有 fixtures：独立标准签名／layout，源合法/非法编号，payload copy，Win event/Linux pthread 同 HTH 握手、sync/queued、重入、STOP/BUS_OFF/DeInit-reInit stale token、runtime reporting及 MemMap 正负向量。
- [x] `targets/semantic_profiles/epic4_batch`／`host_batch_faults`、Python native boundaries：真实 production write vs flush、owner receipt、关闭／quiescence，冻结旧 include-only 原源码与标准-only negative、include 顺序、三族各自 include/source/main/seal/live 回归。
- [x] `runtime/README.md`、必要交付说明与本规格：明确标准视图选择、旧 legacy 限制／默认段和剩余符合条件；审 ABI/第三方及换行后才 assets update/check。真实受影响生成三族／双 target／各 main compile/link/run/c-check，新增／受影响违规整改，原始历史诊断不抹除。
- [ ] 本规格及 `sprint-status.yaml`：按同一 Build snapshot 隔离实施、独立内部 review／修复、真实开发验证和同卡 Tester 移交；记录 draft/ready/in-progress/review 的实际阶段，独立 QA 与下游 integration 未发生不称 done。

AC1：Given 合法／非法 controller/HOH 和状态输入，When 初始化、STOP/BUS_OFF、恢复与发送，Then 独立标准签名/type、连续编号和数量成立，BUSY／拒绝不改既有输出，显式恢复和标准 Rx/DeInit/错误/MemMap 软件差距闭合。

AC2：Given 异步发送及可复制帧，When 真同 HTH 竞争、锁忙／锁错、迟到／错误／重复确认和关闭，Then 唯一 buffer/lifetime、实际物理输出后恰一次 receipt 确认，旧生命周期不污染新帧，真实线程/handle/owner quiescence 按各平台证明。

AC3：Given sink 或测试控制启用，When 正常重新生成受影响三族各目标/main，Then 标准视图无 host/test 控制，适配依赖与来源显式接入，冻结旧 consumer 不改源码 compile/link/run，旧 profile/wire/seal/live 保持且普通生成无需 compiler/官方原件。

## Implementation Notes

2026-10-11 初始 builder 实施记录（历史候选 in-progress；其时未进行独立 review、Tester 或合并，后续复核及复验见下文）：

- 保留本地 `autsaro/t_a00fdf06-can` 和 baseline `6290f1631fa066f48db70b9f789b7e17bcd4fe63`。开始时已有的 context／sprint 修改逐字节保留；本卡原有未跟踪正文保留，冻结区未修改。
- 已按 frontmatter 读取全部 context、仓库开发入口及 MISRA applicable references。只读核对合法 R24-11 CANDriver、CANInterface、BSWGeneral、StandardTypes、CommunicationStackTypes、MemoryMapping PDF；未将原件复制入仓库。独立预期依据包括 SWS_Can_91009/91010/91002/91011/91012（DeInit）、00273/00274（bus-off 取消）、00299/00300/00012/00395（Rx shadow／overrun）、91020（runtime DATALOST）、00413/00415/00416/00429/00496（配置／公开类型）；ECUC_Can_00106/00316/00326；SWS_BSW_00104/00105/00059/00256；SWS_MemMap_00003/00028/00032。适用义务核对与源码内部一致性分开。
- `Can.h` 兼容聚合、`standard/Can.h` 标准服务、`Can_CfgTypes.h` 单 sink 和 `Can_Host.h` 单向显式适配已接入。`Can_Internal.h` 不进入公开 include；`Can_Host.c` 仅转发，不持有 controller／queue／lock。现有签名、Id32、HOH16、PDU／Config 布局和各族 ComStack 宽度保持。
- driver 唯一持有 frame／pending／lifetime。sink、标准 Rx、Tx／mode／bus-off 和 Det 回调在 driver 解锁后执行，在途 reservation 保留；同步 STOP→START、BUS_OFF→START、DeInit→Init 均取消旧确认。Rx 结果属于本次快照，不通过共享全局结果回写。STOPPED／SLEEP 立即 DeInit；未 Init 拒绝 host 业务，旧 GetMode 查询保留。
- 两族 CanIf 均经标准 void Rx 回调；legacy 嵌套时间／返回隔离保留，multi 保持原配置 tick 时钟并使用局部返回上下文。CanIf 的状态锁已与 driver 锁分开，避免外层递归持锁抵消 driver 解锁。真实 Det／host synchronization 作为共享依赖交付，并在实际 bootstrap 初始化。私有已接纳 Rx 接缝在 shadow 忙时报告 CAN_E_DATALOST；可重试 host BUSY 不报告，未虚构硬件事件源。
- 单 ECU 与 multi 的真实 output slot 均保留 driver token，owner 先验证 ticket／PduId，再经 token 完成；output ring、batch、receipt、OS Waiting/quiescence 仍在原 owner。single 集成生成配置显式启用 queued token；legacy 同步策略保持。
- producer 同步 RX_HOH、标准 declaration owner、Det 依赖与 CAN CODE／VAR_CLEARED memorySection、实际 MemMap 头引用。共享资源按正常映射完整物化；修复 Det_Host 私有头在 legacy 交付中的路径。ABI 审阅未发现签名／布局／handle／wire breaking；第三方文件及身份未改变。清单去除与新 source closure 重复的条目，不删除其实际交付能力；按固定换行字节更新 assets。
- MemMap 显式映射受控 GCC PE/ELF `.text`／`.bss`，严格单 marker、配对及禁止嵌套。正常 quality 入口以合法 section TU 检查协议，保留非法向量拒绝，不增加 checker 排除。标准公开 module 80／软件 0.1.0 有项目依据；未编造 vendor 或 R24-11 数字发布宏。GetVersionInfo 默认 OFF，source ON 和直接非零宏均拒绝。剩余身份／版本交叉检查条件：正式分配及适用数字版次依据、公开头一致性和双目标测试后才启用；不得用这些宏宣称完整符合。
- Linux 同 HTH 使用真实 pthread condition 握手，Windows 原 event 路径保留。新增声明／布局／混合 include／标准-only negative／MemMap 正负向量。原 `com_chain_contract.c`、`diagnostic_contract.c` 等 include-only consumer 源码保留。production fault fixture 移植至 Os_Host 正常边界，直接编译封装交付的生产 writer；真实失败流以 unbuffered write／buffered flush 两模式区分，原关闭／watchdog／capacity／sequence oracle 保留。
- 初始 builder 未调用内部 review／Tester 子代理，交由 parent；后续两轮独立内部 review 已执行并记录于 Review Triage Log，但不是 Tester／QA。未修改 sprint 为 done，未创建仓库 agent report、验收管理工具或新 profile。

## Spec Change Log

2026-10-11：delegated approval by dev-pm under maintainer authorization 2026-10-10（t_4aa9b53e；原卡 decision:can-9.2-checkpoint1:v1）。批准 SHA256 219c3b65f872d03377d2929f9bab96be7f87f08b733fb3a69016e1310cef1d4d 的 draft：Keep full spec；Approve and continue。复用 dev-user t_b6a1f960 三 AC，完整 Code Map 和跨层闭包服务同一目标，不缩减为目录改动；3729 tokens 的上下文负担通过依赖有序实施控制。候选 HEAD 6290f1631fa066f48db70b9f789b7e17bcd4fe63、分支 autsaro/t_a00fdf06-can，冻结意图未修改。这不是直接人类回复、独立 QA、客户业务验收或发布批准。

## Review Triage Log

2026-10-11 独立内部源码 review（不是 Tester/QA），snapshot diff SHA256 `f48f94116ee4ba7087aad3aeae49103f446a78ed4517691a34756b4d013b8285`：

- 阻塞：新 CanIf 独立锁与 Ecu_HostBusSM／SchM_Diagnostic_Host 仍共享 driver 锁形成反向锁序；需一起闭合直接依赖并以线程握手复现。
- 高：mode／bus-off、Tx confirmation、Rx snapshot 在 driver 解锁后尚未进入 CanIf 的窗口能穿越 STOP／DeInit-reInit，旧事件可能更新新消费者。必须将 callback admission 与取消／生命周期建立同步，回调后的 lifetime 检查不足以隔离副作用。
- 中：接收 DATALOST 使用发送 service ID 0x06，fixture 固定了该错误；需按真实接收责任入口纠正诊断身份。
- 上述发现待修复与复验；Tasks 前三项实现完成标记不代表这些发现已关闭。新增／受影响 C 诊断尚待裁决整改，不称完整实现或合规。


2026-10-11 本轮 Engineer 按上述 review 修复并完成 Linux 开发复验（未替代独立复核／Tester）：

- 锁序修复：复用现有递归 `SchM_CanIf_CANIF_STATE` 作为 admission／consumer 同步区域。BusSM、ComM／BswM、Dcm／CanTp、实际生成 RTE mode publication 一起迁移；所有直接依赖不再持 driver 锁进入 CanIf。CanIf→driver 顺序保留，driver 在分发 CanIf／Det 和调用 sink 前释放自身锁。
- 生命周期修复：Can Init／DeInit／标准及 host 模式改变与 mode／bus-off／Tx／Rx callback 的快照和分发使用同一 admission 区域；Com／LSduR／PduR／CanTp／Dcm 初始化及已有 DeInit 接入该区域。跨线程取消或新消费者初始化等待已准入回调结束，未准入回调不能携旧快照越过取消；同线程重入取消仍成立。sink 不增加跨调用 driver 持锁或取消等待，原在途槽及 token 策略保留。Can.c 仍是唯一状态 owner，Can_Host.c 未新增状态／queue／lock。
- Det 直接消费者同步：Init 与报告共享 admission 区域，报告保留本次 hook 配置快照；hook 重入 Init 不会将旧报告剩余部分派到新配置。同步与 fatal 行为均保留，独立报告线程及递归报告回归通过。
- DATALOST 身份修复：核对合法 R24-11 CANDriver 原文 SWS_Can_00395／91020／00226（PDF p82，scheduled Read service ID 为 `0x08`；`0x06` 是 Write）。已接纳 ingress 的 overrun 记录在 Can.c，由实际 `Can_MainFunction_Read` 检测／消费并报告；polling 在下一次 Read 处理，即时接收检测后调用 Read。Det 在 Rx callback 前且 driver 解锁后报告，避免 Rx 重入初始化改变报告消费者；host admission 的可重试 BUSY 不产生丢失事件。原 `0x06` 记录保留为被本轮纠正的历史实现。
- 确定性窗口：新增现有 fixture 目录下 `can_callback_lifetime.c`，正常 `multi_runtime_contracts` 注册。通过真实 pthread／Windows critical-section 与条件通知握手，在 driver 解锁、CanIf 分发前暂停；直接观察 native try-lock 的忙结果，再允许分发，不用 sleep／仅“线程已启动”的时序猜测。两种 polling／即时配置，各 11 条 callback／直接依赖路径与两种取消方式，共 44 轮；覆盖 host START→STOP 次序、polled mode／bus-off、Tx／Rx、BusSM、Dcm、CanTp、ComM、BswM、Det Init。独立线程验证 DRIVER 锁空闲；新消费者计数／后续确认验证旧事件不污染新 lifetime；原重入取消及 stale token 回归保留。
- 实际对象证据：`c_analysis` 的 native-tests 分支按正常生成三族 signals／standard-ecu／multi-component，从各自 `target.json` 的实际 flags／includePaths 编译交付 `src/Can.c` 为 Can.o，objdump 核对真实标准函数 `.text` 和 controller／initialized／payload／lifetime `.bss`。另包含同一交付源检查实际全部静态状态在 Init 前清零，link stubs 只关闭链接且不作为行为证据。原 synthetic marker 正负测试保留，不再单独承担实际 driver 段证据。
- 受影响诊断检查：实际读取父任务 `hermes-c-check` 三族原始 summary；修复 SchM lock 初始化的作用域和括号、driver 与 CanIf／Det 内部名字冲突及 bus-off 冗余 initialized 判断。固定换行／公开 ABI／第三方身份审阅后更新 source assets。`AGENTS.md` 增补锁拆分时直接依赖锁序与 callback admission 生命周期握手要求；没有新增报告或运行工具。
- 剩余：独立复核、Tester、Windows 本机执行未发生。父任务原始扫描为 signals 25 TU／292 diagnostics、standard-ecu 46 TU／1874、multi-component 53 TU／2631，均 `error=null, passed=false`；它们不是本轮修复后的源码通过证据。保留 MemMap include、R8.9 scope、POSIX feature/prototype、fatal termination 等未关闭／待逐条核查的原始诊断，不申请 suppression／偏离／排除，不称完整 MISRA 符合。按本轮限定不重复父任务全量 c-check；修复后按同工具重新生成／扫描及逐条裁决仍是源码验收退出条件。本轮未提交、push、PR，也未改初始 context／sprint 或冻结意图。

2026-10-11 最终独立内部 follow-up review（只读，无测试／文件修改，不是 Tester／QA）：原锁序、mode／bus-off、Tx、Rx 生命周期及 DATALOST service ID 五项源码发现均已确认关闭。复核实际 SchM／BusSM／ComM／BswM／Dcm／CanTp／Det／RTE 直接链，确认 admission 在 driver 快照前取得、覆盖解锁后分发，driver 锁外回调及同线程重入保持。44 轮 fixture 对直接依赖分支证明其真实 SchM／CanIf 链，未逐个执行完整模块 Init，初始化接入由源码复核确认。此结论不代替 Windows、Tester 或尚未通过的源码 c-check。

2026-10-11 诊断整改定向 follow-up review（独立只读，reviewer session `c989ad57-0e27-4d08-b8a6-b6b620618e21`；原始记录 can-9-2 scratch `run50-fix-review.json`；无文件修改、无测试、无 Git／网络，不是 Tester／QA）：

- 结论：对 Verification 中“c-check 诊断归因与整改”所列改动未发现回归阻塞项。
- 复核范围：共享状态锁 helper（Linux 独立 once／recursive lock、初始化回收与 ready 发布不变，`abort` 仍三处）；standard ECU `Ecu_SchM.c` owner 断言与 baseline `SchM_Enter_CanIf_CANIF_STATE`→`Can_Lock` 语义一致，`Ecu_Target.c` 无新增非 owner 进入 CANIF_STATE 的路径；三族生成源码闭包无重复定义或缺失，所有编译 `SchM_CanIf_Host.c` 的测试点同时带上 `Can_HostLock.c`；状态锁先于 driver 锁、写路径只用 `Can_TryLock`；bus-off 通知条件编译行为等价；Det／CanIf／BusSM 只是写法变化，行为不变。
- 非阻塞限制，保留为后续验收条件：
  - `host_lock_faults` 只通过 `Can_Lock` 实例覆盖共享 helper，状态锁实例自身的初始化失败（`ready=0`→abort）与 unlock 失败分支没有直接执行。
  - Win32 分支仅按原型做源码核对，未编译或运行。
  - c-check 计数与测试通过数来自作者报告，本次 review 未复现。

## Design Notes

入口 decision:can-9.2-build-contract:v1 已冻结双视图／不变 ABI 和原三 AC；CHECKPOINT 1 委托批准见 Spec Change Log。实现跨度大但只有一个用户目标，不能拆成纯目录整理而减少三 AC。callback 锁拆分须先固定在途／生命周期回写机制，再迁移 consumers；保留有效复制、取消、重入回归。对不存在的实际 overwrite 输入不伪造生产事件；接纳／丢失报告边界需实现和可执行独立向量，不默认新增公共 host API。

## Verification

2026-10-11 接手复验与整合（Windows 实机；接手者不是原 builder，下列本人改动未经第三方复核）：

- 环境：Windows 11，锁定 MSYS2 `gcc.exe (Rev5) 16.1.0`（SHA256 `d38d4dd6…700e2`，与 `runtime/os/toolchain.json` 一致），Cppcheck 2.21.0，合法 R24-11 XSD／ECUC archive，经 `AUTOSAR_CC/OBJDUMP/NM/GIT/PYTHON` 显式声明；`uv run --locked`、Cargo `--locked`。
- PR22 exact-head CI 的 Windows `user-application` c-check 失败：`compiler-defines-13: timeout`。本地复现确认不是源码问题：`c_check.py` 每个 compiler worker 对每一行 `#define` 重新构造 `identifiers - owned_macros`，单个 TU 约 60 秒纯 Python 计算并持有 GIL，可饿死其他 worker 等待 gcc 的 60 秒超时；本卡增加的交付源码扩大了标识符集合。修复为每次分析只构造一次查找集合，行为等价：本地同一样本 46 TU、1872 条诊断逐条相同，发现的 982 个宏定义相同，耗时 7m13s→2m30s；`test_c_analysis.py` 12 项（2 平台 skip）通过。
- Windows 原生首次执行，`--features native-tests`：本卡引入的 3 项失败已修复——`can_callback_lifetime.c` 包含完整 `windows.h` 引入的 `rpcndr.h` `boolean` 与 `Platform_Types.h` 冲突；MemMap／`Can.o` 段检查只识别 ELF objdump 输出（PE/COFF 为 `(sec N)` 索引，改用共享 `symbol_sections` 经 `objdump -h` 解析）；Windows-only `artifact_reader.py` 仍预期 13 模块／29 入口，实际交付新增 Det、`Can_DeInit` 与四个 CanIf 回调（35 入口），替换 API 负向向量改用非入口的 `Can_GetControllerMode`。同时修复 master 原有的 `det_parallel.c`／`canif_parallel.c` 同一 `boolean` 冲突及 Det consumer 的 CRLF 逐字节比较。
- 修复后 Windows 结果：`multi_runtime_contracts` 10／10（含 44 轮生命周期握手的 Windows critical-section 路径）、`c_analysis` 2／2（三族真实 PE `Can.o` `.text`／`.bss` 与初始清零）、`native` 中 `can_declaration_views_and_memmap…`、`epic4_generated_artifact_obligations` 通过；`test_host_boundaries.py` 4／4，`Can_HostLock.c` Win32 分支实际编译运行（关闭前述“仅按原型核对”限制的 Windows 部分）；默认 Cargo 全部通过，Python 47 项、`quality --base 6290f16…` 61 文件及 C syntax、`assets check` 0 changes。
- 已知差距（非本卡引入）：`epic4_independent_behavior_and_legacy_regression` 在 Windows 完整原生套件并行运行时以 `state=Failed reason=7`（`E_OS_STATE`）关停，master 上同样失败；单独运行 70.9 秒通过。属主机 ECU 批次执行在高负载下的时序敏感问题，由 9.3（ECU 批次执行与收据责任）承接；退出条件为 Windows 完整原生套件并行运行通过且不放宽期限。
- CI 覆盖缺口：Windows 必需 CI 不运行 `native-tests`，上述 Windows 原生问题此前均未被门禁发现。由 9.17／9.18 评估把 Windows `multi_runtime_contracts` 等快速原生套件纳入必需检查。
- 未完成：独立 MISRA 221 项人工评估、源码诊断关闭（Windows `user-application` c-check 46 TU／1872 diagnostics，`error=null`、`passed=false`）、完整 AUTOSAR 符合与硬件验证，保持原有范围说明。

2026-10-11 PR22 exact-head CI 发现与修正（已提交候选 `ebbdce3` 之上的工作区修改，同一规格／snapshot，不重启 review；不是 CI 通过记录）：

- CI 发现：Development checks 的 Python ubuntu-24.04／3.12 job 在 `ruff check` 步骤以退出码 1 失败。原始日志 can-9-2 scratch `pr22-python-linux312.log`：`FLY002 Consider f-string instead of string join`，位于 `tools/python/src/autosar_tooling/quality.py:224:22`，即本卡新增的 Can_MemMap 反复 include TU 用 `'\n'.join([...]) + '\n'` 拼接字面量；`Found 1 error`，仅有 unsafe fix。
- 修正：不用 unsafe autofix 或规则抑制，改为隐式相邻字面量拼接，每行显式带 `\n`。用 `uv run --locked python -I triage/fly002_bytes.py` 比较新旧表达式，输出 `identical True`，生成的 TU 字节不变。
- 开发者检查（Linux，`uv run --locked --offline`）：
  - `ruff check tools tests/python scripts`：All checks passed。
  - `--env-file triage/verify.env python -m autosar_tooling quality --base 6290f16…`：55 文件及 C syntax 通过，含该 Can_MemMap TU。
  - `python -B -m unittest tests.python.test_quality tests.python.test_development`：18 项通过。
  - `python -B -m unittest discover -s tests/python`：47 项通过。
- 尚未完成：修正尚未提交，PR22 新 head 的 CI 尚未运行，其余 required jobs 的状态不由本条推断。

2026-10-11 交接状态（冻结候选，供同卡 Tester；本条取代下方各历史记录中“缺少 Git author 身份”的当前状态描述，历史原文保留）：

- 规格状态 `in-review`：冻结当前候选，等待同卡独立 Tester，不是 done。Tasks 4–7 保持未勾选，直到 Windows controlled target 执行、双平台 required CI 与 Tester 完成。
- 证据层次分开：
  - **Linux 开发验证已执行**：native 45／45、multi_runtime_contracts 10／10、c_analysis 2／2、host boundaries 4／4、quality、assets check，以及六样本 Linux c-check（只减少、无新增，均 `error=null, passed=false`）。
  - **未执行**：Windows 编译／运行／c-check、双平台 CI、状态锁实例故障分支的直接执行、独立 Tester／QA。
  - **未达成**：源码诊断与 221 项 MISRA 人工评估关闭、完整 AUTOSAR 符合、vendor 身份、硬件／认证。上述 Linux 证据不构成完整符合。
- 当前身份条件：PM `t_0f7b6fb8` 依据 Boss 授权 `t_cb5d3d98` comment32／run21 提供提交身份；命令级 git identity probe 成功，未写入持久 Git 配置。授权的提交、PR 与 exact-head CI 由 parent 执行，本卡未执行 Git 写入或网络操作。

2026-10-11 c-check 诊断归因与整改（同一候选，HEAD 仍为 baseline；不是独立 Tester／QA，不改冻结意图，保留上方两轮独立 review 及其后既有证据）：

- 归因依据：`hermes-c-check` 已包含本卡初版代码，属中间候选扫描，不是 baseline。因此改用三重证据逐项判定：`hermes-final-c-check` 原始 `diagnostics.xml`、`implementation-20261011.diff` 的新增行文本，以及当前源码人工比对。本会话 Git／GitHub 读取被拒，未另行生成 baseline 扫描。临时归因脚本与全部输出位于 can-9-2 scratch 的 `triage/`，未进入仓库。
- 判为本卡新增并已整改：
  - (1) `SchM_CanIf_Host.c` 整段复制主机锁，新增 R21.8 `abort` ×2（另含 Windows 分支 1 处，Linux 扫描不可见）、R21.1 `_XOPEN_SOURCE`、R17.3 ×4。这超出 `spec-free-c-analysis.md` 中 R21.8 草案“仅限 `Can_HostLock.c` 三个点”的范围；standard ECU 的驱动锁本是 owner 断言，却另交付了一把 pthread 锁，锁模型也不一致。现改回 baseline 的薄包装：CANIF_STATE 调用 `Can_HostLock.h` 新增的私有 `Can_HostStateLock/Unlock`。host／multi 由 `Can_HostLock.c` 用与 `Can_Lock` 共享的 init／enter／leave helper 实现，`abort` 仍只有原三个位置；standard ECU 由 `Ecu_SchM.c` 以 `Ecu_TargetAssertOwner()` 实现。仍是独立的递归资源，先于 driver 锁获取，锁序与 admission 语义不变。
  - (2) `Can.c` 回调移出锁区后，bus-off 轮询配置下 `notify_busoff` 恒为 0（knownConditionTrueFalse，R14.3 类）。现以 `CAN_BUSOFF_POLLING` 条件编译通知分支，bus-off 与 mode 通知仍互斥。
  - (3) `Det.c` R12.1（上一节）及改写报告函数中的 variableScope；指针三元改为显式 if。
  - (4) multi `CanIf_HostRxIndication` 的 variableScope、`Ecu_HostBusSM_ControllerModeIndication` 的未读初值／作用域。
  - (5) 整改中引入的 `state_lock_once` R8.9，已改为函数内静态（该文件无 MemMap 段）。
- 本卡相关但保留的诊断（无 suppression、无排除、无偏离申请）：
  - Advisory，保留理由：R8.9 `Can.c` `host_tx_completed_through`／`mode_notification_processing` 必须位于 `CAN_VAR_CLEARED` 段，块作用域 static 会离开受控 `.bss` 映射；R8.7 是标准与 host 公开 ABI 函数；R2.5 是 `CAN_SW_*_VERSION` 发布身份宏；R20.1／R20.5 来自 SWS_MemMap 要求的声明／定义包裹与 marker 状态协议。
  - 检查器限制：`Can_HostLock.c` 的 R17.3 来自 pthread 原型。Cppcheck 按项目配置不解析系统头，而 GCC 以真实头和 `-Werror` 逐单元编译通过。
  - 既有类别，未改语义：`Det.c` 10.4 `initialized == TRUE`、10.3、8.2，`Ecu_HostBusSM.c`／`CanIf.c` 其余 10.x／15.7，`Can_HostLock.c` 的 R21.1 与三处 R21.8（待责任人批准的既有草案范围）。
- 摘要：审阅后（`assets.py` 确认 BSW ABI 除 `sources` 外未变，换行逐文件核对 CRLF／LF）先后 `assets update` 7 项、1 项，最终 `assets check` 0 changes。
- 正常入口复验，环境文件 `triage/verify.env`：`AUTOSAR_CC/OBJDUMP/GIT/PYTHON`，`AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE` 指向主 checkout 的只读合法档案，`CPPCHECK`／`CPPCHECK_ADDON_DIR` 指向固定 e73bf44 checker。
  - `cargo test --offline --locked --manifest-path core/Cargo.toml --features native-tests --test native`：45／45 通过，含 `windows_and_linux_ecu_targets_execute_production_protocol`、`semantic_profiles_…`、`epic4_host_batch_commit`／`native_boundary` 等首轮未执行的 12 项（`triage/cargo-native-fix1.log`）。
  - `--test multi_runtime_contracts`：10／10，最终源码复跑（`cargo-fix2-multi.log`）。
  - `--test c_analysis`：2／2，生成 `triage/samples-fix2`。
  - `test_host_boundaries.py`：4／4，故障注入现覆盖共享 helper（`host-boundaries-fix2.log`）。
  - `quality --base 6290f16…`：55 文件及 C syntax 通过。
- c-check：每个样本执行 `uv run --locked python -m autosar_tooling c-check --target linux-x64-controlled-v1 --project triage/samples-fix2/<sample> --output-directory triage/c-check-fix2/<sample>`。结果与 `hermes-final-c-check` 相比只减少、无新增：
  - signals 25 TU，286→274；diagnostic 25 TU，292→280；capacity 25 TU，286→274。
  - standard-ecu 46 TU，3702→3680（两个 main 合计）。该结果来自 `c-check-fix1/standard-ecu`，fix1 与 fix2 两套样本仅清单／摘要／`ecu-tool.py` 不同，C 源与头逐字节相同。
  - user-application 46 TU，3702→3680；multi-component 53 TU，5214→5182。均为 fix2 两个 main 合计，无新增。
  - 各样本均为 `error=null`、`passed=false`，c-check 退出 1；源码诊断与 221 项人工评估仍未关闭，不称 MISRA／AUTOSAR 符合。
- 未完成：
  - `Can_HostLock.c` 的 Windows 分支只按 Win32 原型核对，未在 Windows controlled target 编译或运行。Windows c-check／native、双平台 CI 均未执行。
  - 独立 Tester／QA、提交／push／PR 均未执行。
  - 卡保持 in-progress。

2026-10-11 续作复验（同一候选，HEAD 仍为 baseline；不是独立 Tester／QA）：

- 源码修正：人工复审本卡新增 C 时发现 `runtime/multi/src/Det.c` 的 `Det_ReportError` 条件 `config != NULL && (...)` 缺少显式括号（MISRA C:2012 R12.1 advisory，原代码有括号，属本卡引入）；已改为 `((config != NULL) && (...))`，行为不变。固定 LF 换行与 ABI／第三方身份审阅后 `assets update` 仅更新该文件摘要，随后 `assets check` 0 changes。`Det.h` 将 `uint16/uint8` 写作 `uint16_t/uint8_t`，与 multi `Platform_Types.h` 的 typedef 等价，是 Det 交付给不含 Platform_Types 的 legacy Can.c 所需，签名／ABI 不变。
- 正常入口（Linux，`/usr/bin/gcc`、objdump、git 与 uv 环境 Python 经 `AUTOSAR_CC/OBJDUMP/GIT/PYTHON` 显式声明，Cargo `--offline --locked`）：修正后 `--features native-tests --test multi_runtime_contracts` 10／10（含 44 轮生命周期握手）、`--test c_analysis` 2／2（新生成六样本、三族真实 Can.o 段及初始清零）、`--test native can` 过滤 5／5（standard_can 两项、声明视图／MemMap、两项 CanTp）；`test_host_boundaries.py` 4／4；`quality --base 6290f16…` 52 文件及 C syntax 通过；`git diff --cached --check` 通过。
- 首轮完整 `--test native` 注册 45 项：33 通过、12 失败；12 项都在生成前因未提供合法 XSD／ECUC 档案而失败，未进入行为断言。原因是该轮没有设置产品已支持的 `AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE`，并非档案缺失；下一节已按该设置完整复跑。
- 更正：首轮把固定 Cppcheck、工作树外 scratch 证据和 R24-11 原文记为“不可用”，这是会话访问／发现范围受限所致，不是未安装。固定 checker 位于 dev-engineer scratch 的 `cppcheck-e73bf44c3e49686b7495fab352d03a6c6075516b/`，合法档案位于主 checkout 的 `docs/official/R24-11`，原文位于 `autsaro-reference/R24-11`；下一节已实际使用。Git 身份仍未知，提交／push／`gh` 属上游协调，本卡未执行。

2026-10-11 最终 Engineer 直接复验（以下最终结果取代后面的早期 sandbox／缺配置失败作为当前状态；历史失败保留溯源）：

- 正常 `uv run --locked` 可用；显式使用固定 GCC／objdump／Git、既有 Python 环境和只读合法 XSD／ECUC archive，Cargo `--offline --locked`，所有生成／分析产物在指定 scratch。默认 Cargo `--no-fail-fast` 完整命令退出 0。native `--test native` 45／45；multi_runtime_contracts 10／10，含 44 轮生命周期握手；native `--test c_analysis` 2／2，实际三族 Can.o `.text`／`.bss`、初始清零及六样本正常生成均执行。首次全量 native 43／45 暴露两项 CanTp fixture 缺少新真实 SchM 链接依赖；已补入真实 `SchM_CanIf_Host.c` 及 pthread，再完整复跑为上述 45／45，未加同步 stub 或弱化断言。
- Python native discover 实际运行 21 项，退出 0，`OK (skipped=1)`；Windows 专属用例的平台 skip 不算执行通过。c-check 负向用例输出 `passed=False`／编译失败是其明确 oracle，并非本候选源码扫描通过。
- 最终正常 `quality --base 6290f1631fa066f48db70b9f789b7e17bcd4fe63` 检查 52 文件和 C syntax 通过；`assets check` 0 changes，`git diff --check` 通过。独立只读 follow-up review 确认原五项源码发现关闭，未替代 Tester。
- 在当前修复后重新生成的 `can-9-2/hermes-final-samples`，按正式 pinned checker 对六样本分别完整执行 Linux `c-check`，原始证据位于指定 scratch `can-9-2/hermes-final-c-check/<sample>/summary.json`：signals 25 TU／286 diagnostics；diagnostic 25／292；capacity 25／286；standard-ecu 46／1868；user-application 46／1868；multi-component 53／2624。六份均 `error=null, passed=false`，各实际 c-check 退出 1；外层逐样本循环退出 0 不代表源码通过。compiler／checker 已真正运行，但源码诊断与未评估规范条目尚未逐项关闭；不据此称完整分析覆盖、MISRA 或 AUTOSAR 符合，不引入 suppression／偏离／排除。
- 尚未完成：上述 C 诊断裁决整改及人工规范评估、Windows controlled target／原生线程与输出证据、独立 Tester／QA。卡保持 in-progress，不标最终验收完成。`git var GIT_AUTHOR_IDENT` 实际因缺少 author name/email 失败；未伪造身份、提交、push 或创建 PR，HEAD 仍为原 baseline，未合并。初始 context／sprint 修改及冻结意图未主动改写。

2026-10-11 实际开发验证（原待执行清单保留在下方；以下不是 Windows／独立 QA／完整 MISRA 结论）：

本轮 review 修复的追加结果：正常 `uv run --locked cargo test --offline --locked --manifest-path core/Cargo.toml --features native-tests` 下，`--test multi_runtime_contracts` 10／10（含两种配置 44 轮真实窗口）、`--test c_analysis` 2／2（含三族真实 Can.o 与初始清零）、`--test native standard_can` 2／2。Det hook 重入 Init 的追加反例在原 standard consumer 用例通过。因新增同步依赖，补跑受影响的 Python `test_host_boundaries.py` 4／4；父任务默认 Cargo／全量 native Python／c-check 未重复。正常 quality 对 baseline 检查 52 文件及 C syntax 通过；assets check 0 changes、diff-check、固定换行和初始 context／sprint／冻结区字节保护均通过。直接 uv／supervisor 本轮可用，历史 sandbox/startup 失败仅保留历史含义。 全量 `cargo fmt --check` 曾报告本次文件格式及未改动 `core/src/bin/package_host_reference.rs` 的既有差异；本次文件已按配置格式化，未重排无关文件，最终限定质量入口通过，不称全仓 rustfmt 通过。

- 实际 Linux compiler：`/usr/bin/gcc`，`gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`，SHA256 `1b99826121ae6682a634e5efe09bd3e3df58ce58e0b28f849114ab5b89139c26`；objdump SHA256 `325c4205a4c658a9d1e1ebc469ae55975a2b897a3d3c1e79d9b158612d37f745`，与 controlled target 身份一致。测试改用 execution_settings，不替换 toolchain 身份。
- 本环境 `uv run --locked` 的默认 cache 锁不可写；scratch cache 又遇到网络 DNS／lxml 下载限制。因此复用已有 Python 3.12.9 环境 `/root/workspace/autsaro/.venv/bin/python`，设置 `PYTHONPATH=tools/python/src`，执行相同 tooling／unittest 正常入口；Cargo 用 pinned Rust 1.98.1、`--offline --locked`，target/cache 放指定 scratch。临时 PATH 仅对命令生效，不改宿主配置。
- `--features native-tests --test native standard_can`：2 项真实注册并通过，含 Linux 在途同 HTH、重入、同步取消／重新初始化和嵌套 Rx 时间／返回。`can_declaration_views_and_memmap_enforce_the_public_boundary`：1 项通过，内部多族声明／布局／negative／顺序向量和真实 ELF `.text`／`.bss`、清零运行检查。
- `--features native-tests --test multi_runtime_contracts`：9 项注册，8 通过；`canif_different_pdus_and_actual_confirmation_stop_interleave_safely` 在取得 supervisor 时失败，行为断言未执行。通过项含真实 Com lower chain、polling／zero DLC／runtime error、Det 并发、mode、诊断及 queued stale token。最初 isolated fixture 缺 Ecu_DcmCallbackTypes 私有依赖，已补齐并重跑为上述结果，不使用仓库 include fallback。
- 正常 `--test c_analysis`：2 项通过。真实三族 Linux source package 生成至 scratch `can-9-2/samples10`（signals／diagnostic／capacity／standard-ecu／user-application／multi-component），历史格式兼容与新增非法结构拒绝保持。早期 samples／samples2 的 Det 交付路径、samples4 的 legacy PduId 描述宽度失败已修复；原失败目录保留。反向合法 HOH 的 `communication_types_and_polling_periods_are_source_derived` 正常向量通过，直接核生成 RX=1／TX=0、CAN memorySection、Det 和完整 host/private 选集；7/9/17 仍只用于机制 fixture，source 非连续编号独立拒绝。
- 默认 `cargo test --no-fail-fast`：library 33 通过、1 Unix socket 单元测试失败；builtin 72 通过、多组件契约 70 通过、c_analysis 2 通过。总命令退出失败，不能写默认全绿。非 native 构建中 0 项注册的 target 不计验收。原始输出保留在指定 scratch `can-9-2/cargo-default.log`。
- 正常 Python default unittest：47 项通过；`test_c_analysis.py` 单独 11 项通过（失败／缺工具等是其负向 oracle，不是本候选 C 诊断）。`bsw-catalog`：17 个真实 producer 清单通过。`quality --base 6290f1631fa066f48db70b9f789b7e17bcd4fe63` 使用实际 GCC／clang-format 23.1.1，通过格式、头闭包和 C99 syntax；assets check／diff-check 另按最终候选重核。
- 正常 Python native discover：最初缺 pinned checker／样本或合法 archive 的环境失败保留在 python-native*.log。最终明确 CPPCHECK／CPPCHECK_ADDON_DIR、samples10、真实工具路径和合法 archive 环境后，17 项运行、17 failures（含 subtests）、10 errors、1 平台 skip，退出 1；全套未通过，startup 受 supervisor socket bind EPERM 阻塞，锁错／fatal、production build modes／offline／receiver 行为不能计为通过。最终原始日志 `can-9-2/python-native-complete-env.log`。
- 正式 pinned `c-check --target linux-x64-controlled-v1 --project <generated> --output-directory <fresh>` 已尝试 samples5/multi-component，并最终对 samples10/signals、standard-ecu、multi-component 三族分别执行（c-check-final-signals／ecu／multi 的 summary.json 均 passed=false）；在 checker 身份查询阶段 supervisor `listener.bind` 得到 `PermissionError: [Errno 1] Operation not permitted`，没有开始源码分析。未宣称 analysis completeness、源码诊断通过或 MISRA221 人工符合；无 suppression／排除／偏离申请。Windows controlled target、Windows PE／线程／物理 output、全量手工 MISRA、独立 review／Tester／QA 均未执行。
- 补充实际交付编译／运行：从正常生成的 samples9 seal 读取各 target 的 sources／includePaths／flags，在 scratch 复制实际 kernel 并应用交付补丁；隔离外层 Git，核对选定补丁文件的前后 hash 确认确实改变，不改原包。实际 GCC 身份符合上述锁定目标。signals ecu_host_main、standard-ecu 和 multi-component 各自 ecu_host_batch／ecu_probe 共 5 个产物均 compile／link／run 成功，实际 ELF Can_Write 位于 .text、tx_payload 位于 .bss。legacy T 10 输出实际 X 帧；两个 probe 完成 epoch 20，两个 HostBatch 运行 BEGIN 10／COMMIT，产生真实 OUT ticket/PduId 和 COMMIT_OK。原始 compiler、symbols、stdout／stderr 在 `can-9-2/link-verified/`。该补充证据不替代受管正常套件或独立 QA。早期 Det_Host.h 交付遗漏和未补丁 kernel 的诊断保留；缺失私有头已修复。
- 真实 production fault composed TU 按交付 source closure／实际 private patched kernel 编译 probe 和 test 两模式后运行 6 组断言：write 与 flush 经生产 writer 的真实 /dev/full 分别失败，退出 7、无 receipt／COMMIT_OK；sink fail 退出 7；blocked sink 在 5.009 秒后退出 7，满足原关闭期限断言；sequence 成功保留 UINT64_MAX 边界；overflow 实际 accepted=256、next=257、output_close queued=256 reason=7。原始 stdout／stderr 保留在 link-verified/standard-ecu。这是 Linux 开发者补充运行证据，未证明 Windows／独立 Tester 或完整受管 quiescence 验收。
- 最终 builtin 72＋多组件契约 70 项正常测试再次全部通过，日志 `can-9-2/cargo-contracts-final.log`。声明／MemMap 测试增加 include/standard 优先搜索正负向量后通过（44 个声明编译向量＋7 个 section 向量，日志 can-views-configured.log）；随后只修 rustfmt 差异，quality-formatted-final.log 记录正常质量检查通过（40 个文件、C syntax 实际执行）；最终 assets check 一致、0 changes，git diff --check 通过，所有改动的固定换行字节检查无不符。冻结区及初始 context／sprint 逐字节比较均相同。
- 完整 native 默认并发命令注册 45 项；6 项约 30 分钟仍无完成结果后中止，退出 130，保留 cargo-native.log。单独 source protection 用例在 5.43 秒内到达 supervisor startup failure，未执行保护断言；随后完整 native 按 test-threads=1 正常结束：45 项注册、9 通过、36 失败、退出 101、224.78 秒；全部 36 个失败均在 supervisor startup，原始 cargo-native-serial.log 保留，不能把中止运行或 startup 失败当产品通过。
- 最终原生 batch 边界 fixture 使用 Os_Host 可移植调用；补充按相同 sealed 标准 ECU／实际 patched kernel 编译并运行，退出 0，实际输出 `native_batch capacity=256 refusal=257 same_epoch=0 future_requires_tick=pass confirmations=pass` 及 `lifecycle=Closed ... reason=0`。日志 link-verified/standard-ecu/native-boundary.*；这没有将受管 native 套件或 Windows 验收变为通过。
- 实际暂存 49 个本卡文件失败：Git 无法建立 `/root/workspace/autsaro/.git/worktrees/t_a00fdf06/index.lock`，Read-only file system。Git author identity 也未配置，未自动修改全局身份。当前无暂存／提交／push／PR／CI／merge；本地 native 分支和 baseline HEAD 保持。完整可审查 diff 保留 scratch `can-9-2/implementation-20261011.diff`，包含初始 context／sprint 差异但两文件原始内容未被本次改写。parent 的独立 review／Tester 待执行，不能将本卡标为 done。

以下为待执行，不是已通过记录。两个 controlled runner 必须同精确 HEAD、非零 test count；Windows 不借 Wine/交叉编译结果。

- `uv run --locked cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test native standard_can`；复核实际注册数量。
- 同入口 `--test multi_runtime_contracts`：真实 canif_parallel 两 policy、can_polling_contract、com_chain、diagnostic_queued，及受影响接缝；过滤以真实名字核对，不接受零测试。
- 同 `--test native`：`windows_and_linux_ecu_targets_execute_production_protocol`、`semantic_profiles_preserve_host_and_integrated_protocol`、生产故障/关闭及兼容新增向量。
- `uv run --locked python -B -m unittest discover -s tests/python/native -p test_host_boundaries.py`；保留锁错/fatal oracle；按实际改动再跑 build modes/offline/source 保护。
- 默认 Cargo 和正常 Python 相关回归、`quality --base 6290f1631fa066f48db70b9f789b7e17bcd4fe63`、`git diff --check`、`autosar_tooling assets check`。
- 真实生成受影响三族，`autosar_tooling c-check --target <actual> --project <generated> --output-directory <fresh>`；各 main 独立、固定 Cppcheck2.21.0/addons及真实 controlled compiler。分别记录完整性、源码诊断／失败和 MISRA 人工未审，不把 PR 链路绿色当源码通过；新增／受影响真实违反不得延期。
- 授权 feature push/PR 后使用现有 `acceptance.yml` 双平台 native 及 required CI，读 exact-head raw 日志/实际 test 数；内部 review 完成后同卡独立 Tester，整候选 hash／diff／spec／sprint／风险与 published_pr 交接。完整 vendor/MISRA221人工/硬件/认证/Epic9出口保持对应范围未验证，不冒称整体符合。
