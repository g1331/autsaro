---
title: '经真实 OS 调度运行多组件通信'
type: 'feature'
created: '2026-10-09'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 319d39f9c688c59b6a19ff8c2b7c399874534cba
context:
  - /root/.t3/worktrees/autsaro/t3code-b540700d/AGENTS.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/implementation-artifacts/epic-8-context.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/application-contract.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/acceptance.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/compliance-references.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/.agents/skills/misra-c2012/SKILL.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

8.1 只有声明，完整 multi 工程仍拒绝生成。实现候选 R6 的实际 RTE／SchM／OS／通信闭环，先通过 AC-1–AC-8，再进入源码编辑及交接。

## Boundaries & Constraints

**Always:** 用户已选择最小真实 Rx I-PDU group、标准 DM、COM→RTE 通知，保留正数网络 aliveTimeout。固定 R24-11／C99；同一可信计划生成全部配置、接口和任务调用。按所选配置关闭 COM→PduR→CanIf 及 Dcm→PduR→CanTp 直接依赖的标准接口、状态与回调差距，独立核验官方契约。正常 sealed 工程运行真实用户测试应用和实际 OS；各组件 caller-provided 源码的最小可信准备／快照／归属接纳是本故事必要切片，8.3 承接 live 初始化与再生成流程。旧 profile 精确回归。

**Never:** 第二调度器、主机计时模拟新 DM、函数桩、生成业务算法、替换 sealed 源后重封、空壳标准包装、默认放行不支持配置。R11、Epic 7 收尾及 merge 不在范围。

## I/O & Edge-Case Matrix

| 场景 | 输入 | 预期 |
| --- | --- | --- |
| 通信／任务 | acceptance AC-1–AC-3 | 固定 CAN／DID／local／CS 值及 owner 顺序；重复 epoch 无重放 |
| 初值／隔离 | AC-4、AC-8 | 各 R 独立 init／fanout；CAN 停止不等于 COM_STOPPED；空参数拒绝且输出保持 |
| Rx DM | acceptance 末段独立向量 | 真实 group／周期 DM／接收与超时回调；NEVER_RECEIVED／MAX_AGE／恢复／启停／NONE 保值 |
| 非法配置 | AC-5–AC-7 及 group／callback／timebase | 来源明确，生成前拒绝，无部分产物 |
| 公开接口 | 所选 R24-11 COM、PduR、CanIf、CanTp、Dcm 类型化消费者 | 独立签名编译链接及真实成功／拒绝／buffer 生命周期，不以 inventory 生成唯一预期 |
| 来源／回归 | 全槽真实字节；旧 profiles | 精确来源闭包和 sealed 校验；已有行为不变 |

</frozen-after-approval>

## Code Map

- `core/src/integration/{multi,plan,configuration,schedule,ecu,artifacts,handoff}.rs`：8.1 私有模型已有完整身份；legacy guards 不可直接删除复用单组件模板。
- `core/src/{prepared.rs,generator/delivery.rs,generator/delivery/ownership.rs}`：NativeInputs 已有 application 数组，准备仍 first()／单槽；复用来源 guard、资源身份与正常 source-only prepare。
- `runtime/{include,src,ecu}`：旧 COM 主机签名／计时及单入口保留；新 profile 选择真实标准实现，Ecu_Target 的 mailbox／owner／tick 是唯一调度源。
- `core/tests/{multi_component_contracts.rs,support/epic4_ecu.rs,support/tooling.rs}`：复用正常生成／离线构建／probe／HostBatch；实际 C 行为测试置 native-tests。
- `runtime/contracts`、`tools/python/src/ecu_tools`：资产身份、包内工具与归属同步；审阅 ABI 后处理固定 CRLF 摘要。
- `/tmp/autsaro-r6-official/*.txt`：二十一份官方 PDF 已匹配官方 SHA；COM／RTE handle-bearing 通知属于 R24-11 draft 条款，不能套旧零参回调。

## Tasks & Acceptance

- [x] 模型／fixture／定义：真实 Rx group、timeout、通知 handle、主函数 timebase 和完整调度共同核定，所有非法关系有拒绝测试。
- [x] 模板／运行时／调用方：生成多组件 RTE、标准选定链路配置、回调及真实任务调用；最小可信应用准备入口接纳所有计划槽。
- [x] 正常工程入口：用户应用真实执行矩阵及 AC-1–AC-8；变名、fanout、DM 和下层拒绝验证全部运行，不跳过。
- [x] 交付资源／BMad：审阅 ABI／第三方／换行；实际 multi 与受影响 single c-check，记录完整性与诊断，必要回归通过。

验收：真实生成与 OS 运行关口全部通过，才推进 8.3–8.5；无授权 MISRA 主文不声明完整符合。

## Implementation Notes

最小准备入口按计划接受实例身份及真实磁盘文件，复用 read_source／refuse_links／NativeGuard 冻结来源；精确匹配全部槽，拒绝缺失／重复／额外成员及源文件复用。输出路径／owner／entry 由计划决定，无任意 file-map 替换；guard revision 必须覆盖全部输入。复用既有 v2 policy／seal／builder，多资产选择与 source owner 一致，OS build profile 仍 ecu。此入口不初始化 live／manifest，不承诺 handoff；8.3／8.5 状态保持。

2026-10-09 阶段 1（配置闭包，尚未完成运行关口）：新增 `integration/multi_com.rs`，可信计划的 `comRuntime` 记录唯一最小 Rx group 及其真实 PDU 成员、Rx／Tx 主函数实例名与 timebase、ComUserSignal 的系统映射和 uint16 通知 handle、first／regular timeout 及固定标准回调。原创 multi fixture 明确 group 0、Rx 1ms／Tx 10ms、handle 17、first 0／regular 30ms。只在 multi profile 接纳新增配置语法；旧 profile 不输出 `comRuntime`。R24-11 `ComUserModuleCnf` 的实际定义位于 `Rte/RteComUser` 下；通知声明头按 RTE SWS_Rte_91123／91127 核定为 `Rte_Com.h`。新增 builtin 定义的类型、范围、multiplicity 和枚举已通过固定官方 MOD oracle，不能把这项结果替代 C 行为证据。

本阶段保留全部 multi ECU 生成拒绝 guard，不借用旧单组件模板。当前 BSW 源描述仍以 `Com_AdvanceTime`／`Com_TriggerTransmit` 标记逻辑周期角色；`comRuntime` 仅核对其周期并保存真正的 `Com_MainFunctionRx_<shortName>`／`Com_MainFunctionTx_<shortName>` 名称，后续须共同迁移 runtime catalog、BSWMD、schedule 和生成调用到真实标准实例。尚未实现 ComM／BswM／CDD 配置闭包、标准 C 通信链、多槽可信准备、multi RTE／任务生成和真实 OS 运行；全部 Tasks 保持 unchecked，8.3–8.5 不推进。

2026-10-09 阶段 2（真实 DET 模块切片，尚未完成生成／运行关口）：`runtime/multi` 新增隔离的 R24-11 `Platform_Types.h`／`Std_Types.h`／`Det.h` 及真实 DET 实现。新 profile 的 boolean 使用 C99 `_Bool`、TRUE／FALSE 使用 `true`／`false`，`NULL_PTR` 是 void pointer to zero；旧 runtime 头文件及 `bsw-v1.json` ABI／来源摘要均未改变。`Det_ReportRuntimeError` 初始化前无动作且始终返回 E_OK，初始化后按配置顺序调用全部 callouts，不以 hook 的返回值取消后续通知。`Det_ReportError` 未初始化时无 hooks 并停止执行，初始化后调用 development hooks 再停止执行；明确的 controlled-host adapter 使用 abort 停止 ECU process，以本机 GCC TLS 限制递归 development reporting，允许不同线程独立报告。`Det_Start` 的空动作仅对应官方 SWS_Det_00025 明确允许的“没有启动依赖存储”配置，不是用空壳包装已有实现。

DET 的 cleared state 和 code 使用可核对、成对的 module MemMap 标记，分别实际映射到 controlled x64 的 `.bss`／`.text`；host TLS 使用正常 native TLS 段，不声称 MCU 内存放置。7 个新增 Apache-2.0 资源以独立 `ecu-multi` 选择标签纳入正常 embedded asset inventory，显式 LF checkout 字节，摘要通过正常 `assets update` 建立后检查。已审阅现有 `bsw_catalog.materialize`／`bsw-v1.json`：它们仍是旧 profile 的显式 ABI，不把新标准模块塞进旧声明、改名追认或重新计算旧 ABI。新 DET ABI 由独立 C 消费者按官方签名编译链接；完整新 profile 的 BSW catalog／BSWMD／producer 选择闭包仍须在后续生成阶段共同完成，目前没有移除 multi guards 或把这 7 个资产交付到旧工程。

COM 计数相位仍是后续高风险机制：当前正常 owner 的 epoch0 输入分支不调用周期 COM，首次周期在 epoch1；“新接收跳过下一次 main”的孤立设计会使 epoch0 reception 滑到 31ms，不能采用。必须根据真实启动、copied input、owner tick 的先后共同设计标准周期 DM，并由正常 HostBatch 验证 epoch0→30ms、epoch30 先接收免假超时、重复 epoch 不重放。无 update-bit 的 DM 必须按 PDU 取全部适用信号的最小非零 first／regular timeout（SWS_Com_00290／00291），不能按 signal 分别计时。阶段 2 尚未实现 COM 或其余标准链；所有 Tasks 保持 unchecked，baseline 与 8.3–8.5 状态保持。


2026-10-09 主代理继续实现中的 COM 切片（尚未提交）：`runtime/multi/{include/Com.h,include/ComStack_Types.h,include/Com_MemMap.h,src/Com.c,src/Com_Internal.h}` 接纳标准 COM 生命周期／Send／Receive／group／DM／Rx／pull-copy／confirmation 形状，first timeout 仅零、NONE、每 PDU 一个四字节 UINT32，最多 32 PDUs 对应既有工程 frame 上限。Rx DM 仅由私有 main 实现逐周期递减，owner phase adapter 只提供 reception 是否先于本周期 main，不接纳 timestamp／deadline；epoch0 的 reception phase=false，先于待处理周期的 reception phase=true。该相位尚未接入实际 owner，不能认定真实 OS 30ms 边界通过。独立 Rx-only 消费者的三个 PDU 分别使用 handle17／43／59、timeout30／50／0，真实核验 group／DM／NONE／handle隔离／初值／非法参数／启停与恢复。`ComStack_Cfg.h` 目前只是独立消费者的预期宽度 fixture，正式生成必须由真实 EcuC 闭包核定。配置的标准 SchM 实例 wrappers、标准 Tx 周期／下层拒绝／实际确认语义、新 BSW catalog／assets 和全部生成路径仍 pending；不能将这一源码切片直接放行 full multi。尤其 Tx PERIODIC 不能因未收到上一次确认就跳过后续 configured 周期，须按 COM 所选义务和真实 lower BUSY 拒绝完善并验证。

Windows DET 测试的明确后续缺口：MinGW CRT 的 abort 可产生 stderr／WER dialog，现有广泛非零退出码和 stderr-empty 断言不构成可靠 Windows oracle。按 [Microsoft CRT abort](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=msvc-170) 与 [MinGW 原维护讨论](https://sourceforge.net/p/mingw-w64/mailman/message/59347344/) 核定进程内测试 CRT 行为、准确终止结果及断言失败区别；不修改全局 host 设置、不宣称 Windows 已执行。

2026-10-09 阶段 3：COM IF 源码切片及真实下层消费者（尚未完整接入生成 profile）。完善此前主代理 COM 切片，PERIODIC Tx 每次 configured main 都发出真实请求，不由上次 confirmation 决定是否跳过；实际 CanIf／Can_Write 的接受／BUSY 决定下层结果。`multi_com::inspect` 同时拒绝 Tx PDU period 与 configured Tx main 不同的首批配置；独立有效向量把 COM Tx main／BSW period及现有 alarm/event 一起改成1ms而保留 application／PDU10ms，在 definition 与 plan 两入口获得 source-located COM_TIMEBASE，不用失效引用掩盖拒绝。

新增 `runtime/multi` 的标准 PduR／LSduR／CanIf IF 接缝、不可变 route 配置和实际模块 MemMap；公共 `Can.h`／`Can_GeneralTypes.h` 复用原有受信资源及其原字节，独立消费者按正常flat include布局物化它们，未保留重复契约来源；legacy交付头／Can.c均未改变。PduR／LSduR 在所选一对一、无 metadata、同 partition 路由中翻译各自真实 handle，原样转发结果、不缓冲／复制 LSduR 载荷、不代替 CanIf 长度 admission。CanIf 实际调用既有 Can_Write／Can_GetControllerMode／Can_SetControllerMode，跟踪每个 PDU 已接纳但未向 upper 报告的请求数量，实际 STOPPED 给每个准确 E_NOT_OK；OFFLINE／TX_OFFLINE普通positiveconfirmation被抑制，后续STOPPED仍能准确取消。STOPPED／SLEEP成功请求立即关闭PDU通道，SetPduMode以实际driver状态拒绝旧cache重新开放；STARTED不自动ONLINE。BusOff按真实driver已进入STOPPED的查询结果收敛，随后通知配置的真实provider，重复事件和旧confirmation不重复失败。真实Det runtime hooks消费OFFLINE、Rx minimum-length、Classical最大载荷及globalPDU长度错误。HostRx adapter只转换Mailbox／PduInfo，不将timestamp用于COM DM。

独立 `com_chain_contract.c` 链接实际 COM／PduR／LSduR／CanIf／DET、未改动的 Can.c／Can_HostLock.c 和正常 host 输出 adapter；fixture 只提供不可变模块配置及输出／mode／runtime-error／callback观察者，confirmation观察者实际调用Com_TxConfirmation，无假标准模块函数。正常 host 配套 ProfileLimits header用于独立driver消费者，这不是新 sealed ECU 的容量／生成证据。CanIf对多个未确认接纳请求的计数由真实queued driver验证；仍 pending 的 CanTp／Dcm TP callbacks／ComM／CDD／BswM 不由已声明的可配置 IF transmit入口替代。

阶段边界：尚未注册／交付新COM等资产，未解开multi ECU generation guards。真实 EcuC→PB config、所有 configured standard main符号的BSWMD／catalog／signature／scheduler同源身份、RTE callback freshness与NeverReceived自systemstart的状态、实际OSowner／HostBatch30ms、完整诊断／mode链、不可变多slot caller source准备、normal multi c-analysis sample及CI membership仍待后续阶段。新profile公共types／macros也须进入正常pre-generation namespace collision拒绝，不能等编译才发现。旧Can driver zero-lengthWrite边界及Windows DET精确CRT终止oracle仍明确pending，详见linked compliance／上述记录。所有Tasks保持unchecked，Storybaseline/status及8.3–8.5状态不变。

阶段3审阅修正：已删除未提交的相同Can.h／Can_GeneralTypes.h副本；原受信asset原始字节与checkout一致，链消费者在正常Scratch flat include目录放置这些原头、实际新类型头及独立Cfg／SchM声明。CanIf所有public共享状态路径（Init／DeInit、mode请求／查询、Transmit、Rx／Tx callbacks、STOP cancellation及BusOff收敛）使用同一真实SchM critical area。controlled-host adapter直接进入／退出既有Can_HostLock的同一递归资源，没有第二mutex或反向锁序。Can_Write的接纳至outstanding发布保持同一area，driver flush／confirmation不能在接受发布前丢通知；真实输出callback再调用CanIf getter也不会自锁。SWS_CANIF_00005仅要求不同PduIds重入、同一PduId非重入；新消费者遵守此输入约束，仍检验真实异步确认与stop交错，不以single-owner说明代替所选同步机制。

当前consumer的SchM_CanIf.h只含声明，实际函数体是交付源码候选中的真实host adapter，非测试标准函数桩。最终profile仍必须由真实BSWMD exclusive-area身份生成SchM声明／wrapper及source provenance，不能将当前fixture的area名字当作已完成生成闭包。同步机制不引入scheduler／timer，不改变COM DM周期来源或legacydriver bytes；新host adapter资产注册仍待fullgen阶段。

2026-10-09 阶段4：新增真实ComM／主机CDD BusSM／BswM源码切片。ComM单channel、静态users、NM NONE，无PNC／inhibition／NvM；普通FULL请求与诊断活动独立合并，Allowed默认false且只在REQUEST_PENDING评估（00884／00896），进入NETWORK_REQUESTED按configured周期计数最小FULL duration（00886／00889），NONE的READY_SLEEP不虚构自动关网。Init及re-init真实请求lower NO（00073），不通知默认NO（00313）；公开getters查询真实provider而非把desired当current。生成的configured-channel void main及完整EcuC user／channel／period来源仍pending，私有ComM_RunChannel一次只消费一个周期，没有新增调度器或主机时钟。

Ecu_HostBusSM通过实际CanIf／Can调用完成mode，FULL要求真实STARTED indication及ONLINE；pending启动时getter保持NO，不能因driver已改变但CanIf尚未确认就放行Rx。真实bus-off持续阻止普通FULL请求恢复，需要既有明确host CAN mode命令；normal owner STOP须先释放持久user，再请求实际provider NO，不能用Allowed(false)强停FULL。BswM选一个ComM IMMEDIATE input和真实lower方向action，不停止COM group；重入input按00069／00281延后到当前action完成再仲裁，失败不伪成功，未选择可选action-failure runtime reporting。标准mode/user类型由Rte_ComM_Type.h提供，最终RTE生产归属仍待生成闭包。异步mode路径的ComM／BswM状态与CanIf一样进入既有同一递归CAN资源，不增加mutex或scheduler；真实BSWMD exclusive-area声明／wrapper/provenance仍pending。阶段4源码尚未注册交付assets，完整Dcm mode notification尚未绑定，consumer观察者只用于验证实际mode，不声称诊断链或sealed ECU已完成。全部Tasks及8.3–8.5状态保持。

后续实现先关闭实际Dcm／CanTp buffer及mode依赖，再共同接入生成／source准备／实际OS，保持可独立审查阶段并在稳定切片返回交接，不在实施handoff期间并行修改共享接口或执行remote操作。CanIf_Transmit接受有效route的SduLength=0会真实到达旧Can_Write的最小1byte拒绝；COM信号4byte／诊断非空PCI不足以排除这个公开可达路径。新profile须在保留legacy行为的前提下关闭此标准差距，以真实driver／sink／confirmation独立验证零DLC，不将它反复交给后续阶段。原fixture诊断Rx长度检查8byte且CanTp padding开启，成功SF必须按原配置提交padded DLC8，不私下把实际生成的CanIf minimum length改为1。normal c-analysis生成测试和CI membership须同时加入实际multi样本，保留原5个样本及双平台完整分析失败条件。

2026-10-09 阶段5：新增真实 CanTp／Dcm TP buffer 与模式源码切片。PduR 的不可变 transport routes 翻译各层非同值域 Rx／Tx handles，并原样转发 StartOfReception／CopyRxData／RxIndication／CopyTxData／TxConfirmation；没有 router message buffer。独立消费者使用真实 CanTp／Dcm／PduR／LSduR／CanIf／Can／ComM／CDD／BswM／Det，源码配置和 DID observer 均为 fixture，不提供标准模块函数体。Dcm 拥有两份有界256-byte request／response storage，完整 admission reservation 后由 CopyRxData 提供字节，MainFunction 才调真实 DID provider；最终成功／失败 TP confirmation 前不接纳复用。RetryInfo 按 confirmed／pending／retry cursor 处理并拒绝越过已确认边界，零长度查询不需 payload，拒绝／BUSY 不改写输出或游标。Dcm_Init 选择 ActiveDiagnostic=true、初始NoCom；三个真实标准 mode callback 位于 Dcm_ComM.h，未知channel无动作。Full通知前不发、Silent只禁发、NoCom禁收发，false setter 不绕过 Full gating／inactive；实际最终confirmation释放default活动，non-default保持至S3的主函数周期到期。Dcm re-init释放旧诊断需求并不继承旧 Full 状态。

CanTp 新切片限定单physical、标准寻址、padded Classical、half-duplex、完整上层buffer reservation；SF／FF／CF／FC均经真实lower完成，未复制wholemessage，只有一个8-byte待发送frame。私有状态消费configured main周期，包含N_As／N_Ar／N_Bs／N_Cr／N_Cs、peerBS／STmin、SN／padding／overflow及admission与accepted failure区分。N_As到期只结束upper session，未确认lower N-PDU持续占用至真实lateconfirmation，避免旧confirmation完成新请求；Shutdown／Init丢连接且不伪造upper通知。selected DevErrorDetect=false，不通过development DET触发进程halt；错误SN／copy失败／FC协议失败使用真实runtime COM报告，padded短SF／末CF／FC按00345／00346／00349报告0x70。STmin分离期不消耗N_Cs，按配置tick向上取整，未引入host timestamp、timer或第二scheduler。

阶段5仍是源码候选与独立模块验收。正式profile必须从原始 EcuC 核定 ComStack_Cfg.h 的uint16 PduIdType／PduLengthType、64-byte SDU／实际DcmDslBuffer边界，以及标准timer／SchM／BSWMD／catalog／callback生产者身份；当前独立Dcm256-byte配置不能替代64-byte来源，需加入实际configured capacity的admission／response边界并由fullgen派生。生成所选 single CDD ComM／BswM EcuC闭包、multi RTE／标准task调用、多slot immutable preparation、assets登记、实际sealed OS／HostBatch／c-check与CI样本仍pending。当前新诊断资产尚未登记，assets check仅核对已登记资源；全部Tasks、8.3–8.5及Story status保持原状态。

fullgen新增已核实CAN直接义务：原始multi source选择CanBusoffProcessing=POLLING，旧Can_SetMode(CAN_BUS_OFF)直接回调与缺少Can_MainFunction_BusOff不能作为所选polling证据。按SWS_Can_00227（void(void)、service0x09、SchM_Can.h）、00109及ECUC_Can_00314，必须实际injection先记录事件，再由唯一owner的真实configuredpolling main发布BusOff；00183的empty define仅对没有选择polling适用。新profile仍需关闭CanIf_Transmit合法route、SduLength=0公开可达路径至真实Can_Write的旧最小1-byte拒绝，独立验证zeroDLCsink／confirmation并精确保留legacy行为。修复必须共同reach生成源码、catalog、BSWMD、schedule、asset选择与typedconsumer，精确保留legacy行为，不用summary追认ABI；共享driver实现的阶段6设计调整见下。

2026-10-09 阶段6：复用实际 Can.c 的配置化轮询实现，避免复制第二份长期driver。此前“legacydriver原字节不变”是实现阶段边界而非frozen条款；本阶段调整为共享源码、保留旧公开Can_ConfigType／ABI及旧profile行为。四个编译配置 CAN_RX_POLLING／CAN_TX_POLLING／CAN_BUSOFF_POLLING／CAN_ZERO_LENGTH_SUPPORTED 缺省均0；当前只有独立selected消费者显式开启，正式生成仍须从原EcuC派生而非复制fixture。Rx接纳保存mailbox，真实Read消费；Tx输出后由真实Write通知；BusOff立即使实际controller STOPPED但保留事件，真实BusOff main仅发布一次callback，未消费前拒绝恢复和DeInit。零DLC在selected配置真实到达sink及confirmation，legacy仍拒绝；两个输出局部数组全元素初始化，避免零长度路径带未初始化数据。

新增 SchM_Can.h 的 Can_MainFunction_BusOff(void) 按SWS_Can_00227／00109、ECUC_Can_00314核定；Can_MainFunction_Mode(void)按SWS_Can_00368（service0x0c／SchM_Can.h）核定。固定官方00369／00370／00373文字仍称Wakeup；两个公开入口复用同一个实际mode polling体，不增加计时源或scheduler。正常fullgen须共同核定原source选择的Rx／Tx／BusOff／Wakeup processing、真实main周期、catalog、BSWMD、schedule与唯一owner调用身份。现有Ecu_Target.c逐项Can_Inject与一槽driver意味着开启Rx polling后，同epoch第二帧可能BUSY；必须沿正常input／output路径验证多Rx、COM＋诊断同epoch和confirmation drainage，不能仅把所有Read延至下一周期，也不能移动frozen application positions4／5／6或增加第二scheduler规避。

Dcm_ConfigType新增实际buffer_length，bounded storage仍最多256而admission／CopyRx查询／response生成使用configured容量；合法首批范围7..256。真实chain的64-byte配置验证Start65返回OVFL且输出不改写、Start63保留64、copy剩余边界与过大DID响应的NRC0x14。正式PB配置仍须从原64-byte DcmDslBuffer／SDU及refs核定生成，不把独立256配置冒充来源。Windows DET fixture使用进程内SIGABRT handler精确_Exit(86)，失败注册88、其他signal87，配合固定stdout／stderr区分assert失败；_set_error_mode仅在fixture进程内转stderr，不改变实际Det halt或宿主设置。当前Linux执行，Windows编译／运行未验证。

已审阅共享driver／SchM新增公开entry及仍不变的旧config结构、第三方身份与原profile选择；Can.c／SchM_Can.h保留.gitattributes要求的固定CRLF，正常assets update仅刷新这两个已登记来源摘要，未用摘要更新接受ABI变化。新multi源码资产注册、source-derivedPB配置／通信uint16类型、BSWMD／exclusive-area、RTE／OS／多slot准备、实际multi c-check及CI样本仍pending；全部Story Tasks及8.3–8.5状态保持。

2026-10-09 阶段7（通信配置闭包，未放行生成）：新增 multi_bsw 的不可变communicationRuntime，正常validation与plan共用一次检查，保留真实EcuC PduIdTypeEnum／PduLengthTypeEnum、Can controller／CanIf controller ID、Rx／Tx hardware完整path及不同HOH、实际POLLING选择、RW／BusOff／Mode周期。每个配置PDUhandle及PduLength必须能由所选公开类型表达；独立输入验证UINT8 IDs／UINT32长度与真实ID／HOH／period实例变名，不以默认UINT16或名字猜关系。旧profile不序列化新字段。

原multi source补齐一个真实CanMainFunctionRWPeriods、两个HOH的CanMainFunctionRWPeriodRef和CanMainFunctionBusoffPeriod，均1ms并由同源OS tick核定；未改变应用position4／5／6。builtin按ECUC_Can_00437／00438／00484／00355／00486补齐真实container／field／reference及TriggerTransmit缺省false；固定官方MOD oracle全量核对通过。首批单RWperiod用标准Read／Write未加suffix（SWS_Can_00441／00442仅多period时要求suffix）；不能把新增EcuC period当作BSWMD／schedule迁移已完成。fullgen仍须真实调这些main，并关闭normalowner多RX／confirmation drainage。

新的配置身份揭示必要后续driver接线：原source RxHOH0／TxHOH1，而此前consumer和共享driver只用0；正式CanIf不得把真实HTH1默改0。配置变名向量还核验controller7／CanIf9与TxHOH17保留，fullgen必须相应派生真实driver／CanIf ID及HOH映射（legacy缺省0），同步公开说明／回调翻译／catalog身份并真实运行，当前guards仍保留。ComM／BswM sourcecontainers、Dcm mandatory ComMchannel ref、配置规则／静态UserCallout、SchM／BSWMD生产者闭包及其余fullgen仍pending，本阶段没有消费caller源码或生成可运行multi。

## Spec Change Log

## Review Triage Log

## Verification

正常 Cargo default／native-tests／official-compat、Python、相关质量与 assets check；使用固定 GCC 13 的实际交付工程编译链接与运行。按 docs/development/testing.md 跑真实生成 profile 的 c-check，分析未完成须修正原因；诊断如实评估。矩阵逐行对应已执行测试。开发结果只写本规格及相关既有 BMad 工件。

2026-10-09 阶段 1 已执行：`cargo test --manifest-path core/Cargo.toml` 117 tests passed；`cargo test --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts` 29 tests passed，包括独立 scalar／void typed header 消费者的真实编译、链接与运行（GCC 13，正常工具链环境）；`--features official-oracles --test builtin fixed_r24_11_oracle_agrees_on_integer_precision_enum_and_default` passed。新增拒绝向量在 normal definition validation 与 plan construction 两入口验证 group 数量／handle／成员／重复引用／Tx 成员、主函数 timebase／PDU 绑定、callback 类型／符号／header／缺 handle／重复引用，并回归零 first timeout 的合法数值拼写及生成符号碰撞。正常项目 Clippy（`-A clippy::all -D clippy::correctness -D clippy::suspicious`）、`quality --scope core --base 319d39f9c688c59b6a19ff8c2b7c399874534cba`、`assets check`（0 changes）及 `git diff --check` passed。

历史失败保留：首次 native typed 检查因未设置 `AUTOSAR_CC` 拒绝；补齐明确 GCC 13／objdump／git／Python 的正常环境后上述 29 tests passed。额外 `clippy --all-targets -- -D warnings` 被既有 `core/build.rs:24` 的 `too_many_arguments` 阻断；未扩大范围修复旧 lint，正常项目 Clippy 门通过。阶段 1 未修改 C／模板，未生成可运行 multi ECU，故本阶段没有 multi／受影响 single 的 c-check 运行证据，不声明 DM 行为、标准通信链、真实 OS AC-1–AC-8 或完整 MISRA 符合。

2026-10-09 阶段 2 已执行：`cargo test --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts` 2 tests passed，使用已验证 embedded asset 的真实 DET 两个源码及独立消费者，由固定 GCC 13 以 `-std=c99 -Wall -Wextra -Werror -pedantic` 编译、链接和运行。主消费者真实覆盖未初始化 runtime report、原参数和 hook 顺序、首 hook 返回 E_NOT_OK 后继续调用、嵌套 runtime report、re-init 与 NULL 配置；独立子进程覆盖未初始化 development report、完整 development hooks、递归 development hook、非法 hook list，Linux 实际结束信号均为 SIGABRT（6），精确 stdout 及空 stderr 排除普通拒绝／断言失败假通过。第二消费者使用真实 pthread 双线程，各 1000 次 runtime reports，调用方 TLS 观测各自恰好 1000 次且报告仍 E_OK。预期依据为固定官方 DET SWS_00008／00009／00010／01001／00014／00018／00024／00026／00208／00501／00503 和 Platform／StandardTypes 条款，未从 ABI inventory 生成预期。

阶段 2 正常 `cargo test --manifest-path core/Cargo.toml` 117 tests passed；正常项目 Clippy passed；`quality --scope core --base 319d39f9c688c59b6a19ff8c2b7c399874534cba` passed（12 files formatting，C syntax checked）；`assets check` 0 changes 和 `git diff --check` passed。这里的 C syntax／独立模块行为证据不等于 sealed multi ECU 行为、实际 OS 调度或生成 profile 的 `c-check`；后者和完整链路验收仍 pending。Windows consumer 已保留正常编译入口但本次未执行。已按 MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2 的相关指导核对声明、初始化、循环边界、指针与函数指针、host 库终止及跨 TU 类型；没有授权 MISRA 主文及完整新 profile 分析／人工审核证据，不声明完整 MISRA 符合。

主代理当前 COM 切片已执行：固定 GCC13 环境下 `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts com_standard_consumer` 1 passed／2 filtered，严格 C99 编译链接和真实 Rx-only 模块运行；这不是完整 native suites、Tx 链或 sealed ECU 证据。尚未更新新 COM 资产清单或运行实际 profile c-check。


2026-10-09 阶段 3 已执行：固定GCC13正常工具链环境下 `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts --test multi_component_contracts` 分别4／30 passed；新增链消费者严格C99（`-Wall -Wextra -Werror -pedantic`）真实编译、链接、运行。Rx-only3PDU消费者覆盖first0、regular30／50／0、handle17／43／59隔离、重复enable不复位、NONE／group stop-start／初值／非法参数；链消费者固定独立标准函数签名，真实Driver→CanIf→LSduR→PduR→COM Rx及callback，真实CAN_BUSY保留已复制payload、queued confirmation之前仍按周期接纳下一发送、非同值域handle翻译、pull短buffer拒绝不改写、runtime错误61／62／70／90、多个未确认发送STOPPED逐个失败、TX_OFFLINE delayedpositive抑制、pre-indication重新ONLINE拒绝、真实modepolling、重复busOff／lateconfirmation不重复失败及SLEEP即刻OFFLINE。其DM逐周期运行仍只是模块消费者，不是实际OSowner时间验收。

阶段3 `cargo test --locked --manifest-path core/Cargo.toml` 全部passed（unit17／builtin71／multi_component29）；正常项目Clippy（`-A clippy::all -D clippy::correctness -D clippy::suspicious`）passed。`quality --scope core --base 319d39f9c688c59b6a19ff8c2b7c399874534cba` passed（15 formatting files与正常Csyntax）；runtime扩展scope第一次发现此前DET提交的6处formatting差异，按配置formatter仅调整Det.h／Det.c，API／行为不变，正常assets update只更新这两项固定LF摘要。新COM等资产未注册，assets check的一致性证据仅覆盖当前已登记资产，不能声称新profile已交付。实际新源码同时由上述严格native编译检查；最终 `quality --scope runtime --base 319d39f9c688c59b6a19ff8c2b7c399874534cba` passed（29 formatting files与正常Csyntax）；摘要刷新后4个native模块测试再次passed，`assets check` 0 changes及`git diff --check` passed。

历史失败保留：新链第一次编译缺少正常generated ProfileLimits include；正常driver消费者加入实际host配套header目录后通过。第一次Tx20ms向量额外加入第四owner event，先被现有TASK_PROFILE（三个event）边界拒绝；替换为引用有效、正常已有1ms alarm/event的Txperiod不一致向量后COM_TIMEBASE两入口通过。BusOff后额外请求STOPPED的早期consumer预期被真实旧driver的已STOPPED拒绝暴露；核对实际driver模式与官方CanIf义务后改由真实state-query／transition收敛，保留legacydriver不变。没有把这些失败记为验收通过，Windows、sealedmulti ECU、完整标准/MISRA符合与actual-profile c-check均未执行／未宣称。

主代理阶段审阅追加核定：R24-11 SWS_CANIF_00005 的 CanIf_Transmit Service ID 为0x49，而旧值0x05不能沿用。阶段消费者与实现曾共同使用旧值，绿色测试没有识别这项契约差距；已按官方独立服务表同时修正真实DET报告及消费者固定预期。相关native重跑结果随后记录，不以先前通过代替此修正的验证。


2026-10-09 阶段3审阅修正已执行：固定GCC13及正常native工具链环境下 `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts` 5 passed（0 failed／ignored）；重新执行包含已修正的CanIf_Transmit DET service0x49、复用受信原始CAN头的真实链及新增不同PDU并发消费者。新增消费者使用两个真实producer各1000次发送，四种barrier轮次交错真实driver flush／positive confirmation和实际STOP／mode poll，逐轮逐PDU核对accepted等于positive加negative、真实frame等于positive；固定延后flush／stop轮次各自证明一接纳一BUSY且复制载荷保持。实际输出callback在driver资源内重入CanIf getter，正常ProcessOwner以30秒期限运行并核对成功退出与空stdout／stderr；没有测试标准函数体、第二production mutex或额外scheduler。Windows分支未执行，不将此结果扩大为Windows、sealed ECU或完整重入配置符合证据。

审阅修正正常Clippy（native-tests、multi_runtime_contracts目标，`-A clippy::all -D clippy::correctness -D clippy::suspicious`）passed；正常 `quality --scope core --base 319d39f9c688c59b6a19ff8c2b7c399874534cba` passed（17 formatting files与Csyntax），runtime对应scope passed（28 formatting files与Csyntax）；`assets check` 0 changes、`git diff --check` passed。新增host SchM adapter尚未登记asset，正式configured BSWMD exclusive-area wrapper／provenance以及actual-profile c-check仍待fullgen阶段。历史失败保留：新增并发消费者第一次正常ProcessOwner运行因缺少required private execution-log目录而被拒绝（directories_missing）；按既有ProcessOwner约束在Scratch创建正常0700目录后重新运行通过，未放宽隔离或退出判定。全部Tasks及完整Story运行关口保持pending。

2026-10-09 阶段4恢复后独立复核：固定GCC13／Python／objdump／git环境运行`cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts`，6 passed、0 failed／ignored／filtered；严格C99编译链接实际mode模块与既有真实CAN／新COM链。新consumer独立核验mode/user宽度与数值、标准typed签名、未知user／channel及空输出拒绝、初始Allowed、两个user与诊断需求、3周期最小FULL／重复请求不重置、NONE READY_SLEEP保持、真实bus-off持续10 main且无假FULL、显式恢复、释放user后真实STOP持续10 main、SILENT允许Rx拒绝Tx、COM缓冲保持、BswM即时方向效果／下层失败／重入延后／Deinit、ComM re-init关闭实际lower及STOP→START待确认期间拒绝Rx。此前5个模块／并发消费者同时重新通过。

阶段4正常Clippy（native-tests／multi_runtime_contracts目标，`-A clippy::all -D clippy::correctness -D clippy::suspicious`）passed；正常quality core／runtime以原story baseline检查，分别20／43 formatting files及Csyntax passed；assets check 0 changes。历史失败保留：恢复后的首次core quality因当前PATH没有rustfmt而退出1；给该正常命令添加本地Cargo工具目录后重跑通过，没有修改全局环境或关闭规则。新mode源码未登记assets，摘要检查不构成其交付证据；Windows、实际generated multi／OS调度闭环、完整Dcm／CanTp及actual-profile c-check仍pending，不能将模块切片结果写为完整Story或完整标准／MISRA符合。

2026-10-09 阶段5已执行：固定GCC13／正常Python／objdump／git环境下`cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts` 7 passed、0failed／ignored／filtered；新增diagnostic消费者使用严格C99 flags真实编译、链接、运行。按官方固定signature消费Dcm／CanTp公开接口并通过真实不同handle链验证SF DID wire 0762123412345678、完整buffer admission／zero query／拒绝输出保持、未Full与未知channel、活动保持到真实confirmation、multiple DID25-byte segmentedresponse／peerBS1／SN顺序、segmentedrequest成功、wrongSN、N_As／N_Bs／N_Cr timeout释放及lateconfirmation隔离、FF overflow、shortpadded末CF的真实DET、Shutdown／reinit、RetryInfo pending／retry／confirmed边界与BUSY输出保持、non-default活动及S3周期释放、STmin10ms大于N_Cs5ms仍按真实周期发首CF。最后扩充padding／N_Bs／STmin后，targeteddiagnostic再次1passed／6filtered。COM／DET／CanIf并发及mode的此前六个模块tests同轮通过。

阶段5正常Clippy（native-tests／multi_runtime_contracts，`-A clippy::all -D clippy::correctness -D clippy::suspicious`）passed；quality core／runtime以原storybaseline分别23／51 formattingfiles及Csyntax passed，assets check0changes及gitdiffcheckpassed。历史失败保留：首次新consumer严格编译指出误用了不存在的Can_Receive／Can_FlushTx／Can_MainFunction_Mode及缺少ComM_BswM声明；改用实际Can_Inject／Can_HostFlush／Can_MainFunction_Wakeup与正常声明，并从真实Can_MainFunction_Read消费。首次运行queuedpolicy下flush不合成confirmation，使buffer正确保持但consumer误期望复用；核对真实hostadapter后使用selectedsynchronousconfirmation的正常Can_HostFlush→Can_MainFunction_Write，不提供fakecallback。首次quality发现新source格式差异，按现有clang-format仅处理本阶段changedfiles后通过。Windows执行、正常sealedmulti／OS AC-1–AC-8、actual-profilec-check、完整标准及MISRA符合均未宣称；N_Cs的upperBUSY重试分支尚无独立实际可BUSYprovider的行为证据，正式configuredprofile仍须完成相关适用义务与完整timer拒绝矩阵。

阶段5独立审阅修正：先前实现／consumer共同使用Dcm_SetActiveDiagnostic未Init返回E_NOT_OK的错误预期，绿色测试不能代替SWS_Dcm_01068的alwaysE_OK。已修正为未Init无状态动作但仍E_OK，并更新固定独立预期。CanTp原incomingWT计数误用了receiver发出WAIT的WFTmax；按00315每次incomingWT重启N_Bs，移除incomingcap，并由4次WT（超过fixture sentWFTmax2）后真实CTS／STmin／CF成功验证。CanTp原lowerE_NOT_OK重试也违反00343，已改SF／FF／CF真实最终negative并释放buffer；保持upperCopyTxData的BUFREQ_E_BUSY与N_Cs重试机制。新增真实Can_Write占用driver→DcmSF遇CAN_BUSY／LSduR E_NOT_OK→立即可复用Dcm、flush只输出原已占用frame、后续5tick无TP重试、下次新请求成功；没有假lower函数体。审阅修正targeteddiagnostic真实GCC13编译链接运行1passed／6filtered。正常defaultCargo全部117passed（17unit／71builtin／29multi），未将default下0native执行写为native证据。上述pairedoracle错误已修复，历史结果保留其当时范围；最终fullgen尚未完成。

阶段5最终审阅修正后同轮复验：multi_runtime_contracts全部7passed，corequality23／runtimequality52formattingfiles及Csyntax均passed，gitdiffcheckpassed。至此阶段源码冻结交协调者审阅；所有Story Tasks保持unchecked，尚未执行fullgen／actualOS／c-check。

阶段6验收：正常multi_runtime_contracts 8tests全部passed（无ignored／filtered）；其中新selectedCAN向量真实观察零DLC、Rxmailbox复制／BUSY、Read／Write延后通知、BusOff一次通知／negative取消、未轮询DeInit保留事件、Mode明确恢复。最终新增DeInit向量单独复验passed。正常native的standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame 1passed，证明legacy原边界仍保持；其余7module消费者继续通过。Dcm64容量拒绝／NRC0x14由同一实际diagnosticchain核验。正常Clippy native目标passed；quality core25／runtime54formattingfiles及Csyntaxpassed，assets check0changes与gitdiffcheckpassed。

阶段6actualsingle c-check：正常c_analysis sample生成测试1passed，仍生成原5个sealed样本；在standard-ecu上使用固定Cppcheck2.21.0／官方固定addons和GCC13运行linux-x64-controlled-v1的正常c-check，42translationunits、host-batch及legacy-probe两program均完成，summary.error=null且无工具定义的incomplete/parsingdiagnostic。最终1808源码诊断、两programexit1、passed=false；Can.c保留8.7公共接口作用域及20.1MemMap include诊断，不能写成源码合规通过。首轮1810中两处本阶段零数组部分初始化9.3已改显式8元素初始化，重新正常生成sealedsource并完整重跑后该两诊断消失；历史首轮失败未抹去。既有generatedsingle的广泛源码诊断需按真实规则／调用路径继续评估，不申请虚构偏离；没有授权主文／人工221项评估，不声称完整MISRA符合。实际multi仍被generationguard拒绝，无法把上述single分析或module编译替代multi c-check，正常multi样本及CI membership将在fullgen闭环加入。

阶段6源码冻结交协调者审阅，尚未提交本阶段；全部Story Tasks维持unchecked，完整8.2生成／真实OS验收和Windows执行仍pending，不推进8.3–8.5。

阶段7验证：正常multi_component_contracts 31tests通过，含新增communication source derivation及两入口range／period／reference拒绝，最后新增UINT8长度溢出和重复HOH向量聚焦复验通过。正常defaultCargo的17unit／71builtin／31multi均通过；default下没有执行native module行为，不把0native写成通过。固定official-oracles MOD全量definition对照1passed，正常Clippy及quality core26formattingfiles／Csyntaxpassed，assets check0changes与gitdiffcheckpassed。本阶段仅Rust配置计划、builtin与输入fixture，不改C／生成模板或已登记来源，因此复用阶段6实际single C分析范围，不声称实际multi C分析／OS验收或完整标准／MISRA符合。全部Tasks、8.3–8.5状态保持。

2026-10-09 阶段8：模式配置来源闭包。固定 R24-11 MOD 原件核对后，builtin 新增所选 ComM／BswM 参数、容器、choice／reference 的真实路径、类型、基数、枚举、边界与 default metadata，并保持 ECUC_Dcm_00952 的 DcmDslProtocolComMChannelRef 下限1。multi 原始 fixture 明确写入一个 CDD channel／静态 user 关系／NM NONE／1ms main／5ms 最小FULL、一个 IMMEDIATE ComM input、显式初始NO、三条独立 EQUALS rules与 CONDITION action lists。BswMUserIncludeFiles 实际位于 BswMGeneral，其字段是 BswMUserIncludeFile；每条rule明确 BswMNestedExecutionOnly=false，每项明确 BswMAbortOnFail=false。三个 UserCalloutFunction 是包括全部固定参数的完整调用字符串，分别请求 NO／SILENT／FULL；不以 runtime currentMode 参数替代配置来源。固定MOD oracle先后揭示错误include位置／名称及漏写Nested default；均已按原件修正，不把失败覆盖成通过。

同一正常 definition validation 与 trustedplan 消费新的 ModeRuntimeContract；保留真实 channel/user/config/input/rule/condition/expression/actionlist/action/DcmConnection 身份、channel handle、派生 ComM_MainFunction_<真实短名>、周期、最小FULL及静态callout。配置审查拒绝未支持的 bus provider／NM／feature／DEFERRED输入／include路径／nested rule／trigger list／错误引用及动态callout，允许真实channel变名、handle7与user19并同步固定调用参数。所选规则仅一个真且无false list，每次即时仲裁执行对应静态动作；生成端仍须实际include配置头并忽略UserCallout返回（00843），当前Rust模型不构成runtime闭包证据。

新增mandatory引用的历史兼容只覆盖已识别的旧source profile，绝不降低官方下限。旧 epic4-win64-sr-cs-v1 要求完整旧单组件图与九模块依赖、无ComM/BswM，复用既有Graph／component／reference识别；profile身份与当前参数是否可在native target生成分开，SC2等合法通用编辑保留身份，但原target generation诊断继续生效。旧 host-can-v1 复用原完整host source parser、diagnostic shape与host CAN配置核定，实际XML与根据真实解析值render的XML做完整元素／namespace／attribute／text比较；忽略comment／格式空白和独立entry顺序，保留新增业务元素参与比较。解析器提取为共用source逻辑，不构造递归Workspace或进入definition validation。原source caller顺序用于恢复已有handle分配，不由界面排序重写配置。历史standard模板补全明确保留缺少ComM来源的该单点，normal generation仍可用；Dcm-only、multi删除ref及显式新增ComM／BswM都严格报MULTIPLICITY。已识别旧工程的definition coverage对该条为supported=false，状态由原Passed纠正为Unsupported；UI将如实显示此标准缺项，不把功能回归或旧profile兼容写成标准符合。AGENTS补充这条历史身份／target能力分离及编辑、格式回归要求。

fullgen前仍须关闭三项明确软件契约：BswM_ModeType与BswM_UserType按00214／00216提供uint16；BswM_MainFunction(void)按00053从SchM_BswM.h公开，00075仅评价含DEFERRED输入的rules，当前全IMMEDIATE无需新增周期arbitration但不能省略标准公开入口；ComM_EcuM_WakeUpIndication(NetworkHandleType)按00275从ComM_EcuM.h提供，SynchronousWakeUp=false不关闭00893对应channel的NO_NO_PENDING→REQUEST_PENDING，之后真实Allowed进入FULL并保留NONE最小FULL，不被无user/diagnostic立即抵消。独立向量须覆盖未Init／未知channel无动作、待允许wake-up、允许后实际Full／minimum和公开typed消费者；无需引入完整EcuM、PNC或NM模块。CAN source身份迁移须贯通实际所有controller入口（baudrate／mode／errorstate／interrupts）和BusOff／Mode callbacks及HTH，真实controller7／CanIf9／HTH17不能只在Transmit入口接线。完整生成／BSWMD／catalog／SchM／唯一OS调用／多slot准备／sealed AC-1–AC-8／multi c-check及CI样本仍pending；所有Tasks和8.3–8.5保持原状态。

阶段8验收：正常Cargo的17unit／71builtin／2c_analysis全部通过，multi_component_contracts最终34tests全部通过；2c_analysis包含原5个sealed样本正常生成及CRLF／缩进／comment旧host实际sealed generation，新增业务SWC使00952严格拒绝。实际旧source→生成→编译→协议运行使用正常native `semantic_profiles_preserve_host_and_integrated_protocol`，Linux固定GCC13／objdump／git／Python下1passed（41filtered），涵盖host与integrated既有协议，不宣称Windows执行。固定官方MOD全量definition oracle1passed、正常Clippy passed、quality core36formattingfiles／Csyntaxpassed、assets check0changes及diffcheckpassed。本阶段不改C、C生成模板或登记source bytes，复用阶段6single c-check的完整性及failed源码诊断范围；不宣称actualmulti分析或MISRA符合。

历史回归失败保留：首轮历史身份要求target-capability issues全空，合法SC2／COM初值／CanIf编辑因此触发无关00952错误；现已将完整历史图身份与target参数支持分离，原editor成功、sealed generation与target拒绝预期保持，旧Passed标准标签明确改Unsupported。首轮host exactbytes／界面排序重render错误拒绝已有diagnostic样本；共用实际source解析及完整XML语义比较后，按源entry顺序恢复既有handle分配，原样及格式更改均通过，新增业务数据仍拒绝。独立审阅发现上述初版仍错误地将multiple-instantiation能力flag当成结构身份；把原definition接受／target MULTIPLE_INSTANCES预期改为00952拒绝是回归，不能作为合理兼容结论。此历史失败按下面的结构身份修正关闭。阶段8源码与相关规格冻结交协调者审阅，尚未提交；Story Tasks维持unchecked。

阶段8独立审阅修正：抽取共享的单组件 source_identity，标准component inspector与历史兼容共同消费真实SYSTEM ECU_EXTRACT／唯一ECU／物理channel／root composition／application prototype/type／behavior关系；历史兼容另外核定单application＋既有Dcm service实例的真实composition、双方带rootcontext的ECU mappings、唯一Rte instance指向实际application、Rte events归属该behavior，以及既有九模块reference消费闭包。该识别不检查SUPPORTS-MULTIPLE-INSTANTIATION、CAN-BE-INVOKED-CONCURRENTLY或其他runtime capability值，也不按错误码列表过滤。既有消费者遍历提取为共享Graph helper，保持retained未知配置边界。原true capability向量恢复definition diagnostics.empty／诚实Unsupported coverage和target MULTIPLE_INSTANCES；新增true concurrent能力向量保持definition接受、target REENTRANCY_UNSUPPORTED、没有新增MULTIPLICITY。真实Rte retarget至service或移除application ECU mapping都丧失结构身份并严格00952拒绝；Dcm-only／新ComM-BswM／multi严格反例保持。

fullgen bootstrap剩余义务：frozen epoch0的有效Rx必须成功。所选CAN mode polling下，用户FULL请求后的实际Can_MainFunction_Mode须在首帧输入前发布真实STARTED callback，不能只观察driver实际状态或合成provider confirmation；bootstrap与唯一OS owner共同核定调用来源。epoch0不得推进COM DM周期计数，保证30ms超时边界不偏移；实际production HostBatch epoch0向量必须验证，不新增timer或第二scheduler。

阶段8修正后最终验收：正常 `cargo test --locked --manifest-path core/Cargo.toml` 同轮17unit／71builtin／2c_analysis／35multi全部passed，新增结构与能力识别向量包含在35中，没有ignored／filtered；default下0native不算行为证据。实际native semantic_profiles在最终结构修正后再次固定LinuxGCC13生成、编译、运行1passed／41filtered。Clippy、corequality37formattingfiles／Csyntax、assets check0changes与diffcheck均passed。固定MOD oracle结果仍适用于未再改变的metadata，复用其已通过证据。源码及规格再次冻结交协调者独立审阅提交；完整8.2目标与remaining runtime/fullgen义务保持。


2026-10-09 阶段9（模式模块公开契约与唤醒闭包，完整生成仍pending）：新增真正的 ComM_EcuM.h／ComM_EcuM_WakeUpIndication(Channel)，按SWS_ComM_00275／00893／00895实现所选SynchronousWakeUp=false、单channel、NM NONE的唤醒待处理状态。未Init／未知channel无动作；合法wake-up在未Allowed时维持REQUEST_PENDING，普通NO请求或InactiveDiagnostic不取消实际wake-up。真实Allowed后请求lower FULL，必须等实际Can_MainFunction_Mode发布STARTED callback才报告FULL；configured minimum保持，重复FULL期间wake-up不重置minimum。re-init清除上一ComM lifetime的wake-up，pending时DeInit仍拒绝；不增加完整EcuM／NM／PNC模块。

BswM.h提供标准uint16 BswM_ModeType／BswM_UserType（00214／00216）；真正SchM_BswM.h公开BswM_MainFunction(void)（00053）。00075只要求评价含DEFERRED输入的rules，当前全IMMEDIATE配置的该入口无周期仲裁工作，不为其新增period／scheduler。移除同名fixture SchM_BswM.h，使独立typed消费者确实包含交付header，避免局部fixture掩盖标准header availability。配置来源静态callout与include的producer仍须在fullgen生成，并非本阶段已交付。

所选ComM公开接口审查补齐GetInhibitionStatus（00619）、SetECUGroupClassification（00552）及GetCurrentPNCComMode（91002）。这些C API表没有相邻可选API的availability guard；service-port variation控制RTE ports，不构成C API豁免，这是按原件接口差异作出的所选配置适用性判断。ComM_InhibitionStatusType按00669为Rte_ComM_Type.h中的uint8 bitfield。两个inhibition features=false时合法GetInhibitionStatus真实返回0；未Init／未知channel／空输出拒绝且不改输出。SetECUGroupClassification真实更新ECU-lifetime mask，首次合法Init才从source-derived ComMEcuGroupClassification初始化；后续ComM re-init／DeInit不重置该mask（ECUC00563）。没有NvM descriptor且wake-up inhibition不允许时不引入NvM（00783），mask变化不能开启已关闭的features。ModeRuntimeContract保留实际u8 source值，正式PB config必须消费它。当前配置下mask没有可观察的公开读取行为，独立消费者验证setter成功／拒绝与features仍关闭；其真实初始化、更新和跨ComM lifetime保留由源码审阅核定，不增加测试getter或伪造可观察状态。

GetCurrentPNCComMode使用标准ComM_ModeType OUT与Std_ReturnType返回，静态user未分配PNC时返回COMM_E_NO_PNC_ASSIGNED且OUT保持；未Init／未知user／空输出E_NOT_OK。01022／01023的“as ComMode”措辞与API return表不一致，但91002及ComM_UserRequest操作表将NO_PNC／MULTIPLE_PNC列为Possible Errors、ComMode单列OUT，两份规范接口表支持status error与OUT保持的选择，不留虚构用户决策。ComM_UserRequest原件Possible Errors明确MODE_LIMITATION=2、MULTIPLE_PNC_ASSIGNED=3、NO_PNC_ASSIGNED=4；固定typed消费者独立核验实际数值、宽度、签名与拒绝输出。PncSupport=false不扩展可选PNC machinery；明确有availability guard且当前关闭的其他PNC／inhibition／version API不在该selected闭包内。

上层CopyTxData BUSY适用性核定：本selected Dcm先完整生成并保留response，再将准确response_length交PduR／CanTp；PduR不改变长度，CanTp每次只请求min(total-position, frame payload capacity)且真实confirmation后才推进position，retry为NULL。Dcm的BUSY条件仅是请求长度超过完整response剩余，因此正常所选链没有partial streaming或合法upper BUSY触达路径。保留独立Dcm BUSY输出／cursor不改写向量与CanTp规范N_Cs内upper BUSY分支，不添加假provider以把不可达条件写成真实链行为。阶段5“尚无实际可BUSY provider证据”历史结论保留，但selected退出条件改为上述完整buffer/source推导加真实链验证；未来支持partial streaming provider时须另补真实BUSY重试行为证据。真实lower CAN_BUSY／LSduR E_NOT_OK终止并释放buffer的向量继续有效。

fullgen来源接线仍必需：shared driver所有controller公开入口、BusOff／Mode callbacks以及Can_TransmitPdu内部Can_WriteHost的HTH都必须由source身份派生，不能只改CanIf_Transmit或默用0。正常owner多RX／同epoch COM＋诊断／confirmation drainage与epoch0真实mode completion仍待生成验证，冻结application positions4／5／6保持。fixture实际是三个APPLICATION-SW-COMPONENT-TYPE加一个Dcm SERVICE-SW-COMPONENT-TYPE；caller提供的源码slot应从实际producer ownership派生，event-backed用户runnables归application/<type>.c，现有无event Dcm service bridge仍归generated src/Rte.c，不增加无用第四个用户file或覆盖标准service producer。catalog／BSWMD／SchM wrapper、sealed正常多slot入口、实际OS AC-1–AC-8、multi c-check／CI samples和新资产登记均pending，所有Tasks与8.3–8.5状态保持。

阶段9当前验收：固定Linux GCC13／正常objdump／git／Python工具链下multi_runtime_contracts全部8passed（0failed／ignored／filtered），新增公开typed接口、init／channel／user／NULL拒绝、实际wake-up等待与真实mode poll后FULL／minimum／re-init向量；此前DET／COM／CanIf／TP真实模块链同时通过。source ComMEcuGroupClassification原3及合法编辑1保留测试1passed（34filtered）。正常Clippy native-tests目标passed，quality core36／runtime56 formattingfiles与C syntax passed，assets check0changes与diffcheckpassed。这里是独立模块和来源契约证据，未生成actualmulti profile，未运行其c-check／Windows或宣称完整标准／MISRA符合；所选mask的内部保留没有编造可观察行为测试。实际single C未受本阶段新multi修改影响，复用阶段6分析完整但1808diagnostics／passed=false范围。

历史失败保留：新增BswM_MainFunction首次typed编译被fixture本地同名SchM_BswM.h遮蔽，移除旧fixture后实际公开header编译链接通过。shell PATH未提供clang-format时首次格式命令失败，随后沿项目正常uv run --locked clang-format入口完成，没有修改全局工具配置或关闭格式检查。阶段9作为独立可审阅slice交协调者复核提交，再继续完整生成；完整8.2尚未完成。

阶段9最终normal default同轮17unit／71builtin／2c_analysis／35multi全部passed（无ignored／filtered）；default下0native不计行为证据，native8已独立执行。metadata未改变，复用阶段8固定MOD oracle结果。assets0changes／diffcheck最终再次passed，源码冻结交协调者独立审阅。

fullgen CDD／CanIf ID接缝追加核定：Ecu_HostBusSM_Init当前只接受controller0，RequestComMode中的Can_GetControllerErrorState也把CanIf ID当driver ID，source CanIf9／Can7时必须关闭。SWS_CANIF_91001规定CanIf_GetControllerErrorState(uint8 ControllerId, Can_ErrorStateType *ErrorStatePtr)、service0x4b、CanIf.h，ControllerId为abstracted CanIf ID并调用对应driver；00898／00899核定非法ID／NULL development检查，selected DET flag仍控制报告。优先实现真实mapped CanIf API供CDD消费，不在provider config复制raw driver身份。独立消费者须实际CanIf9→Can7、ACTIVE／BUSOFF、未知／NULL保持OUT和真实恢复；此项作为source-derived fullgen依赖，不冒充阶段9完成。


2026-10-09 阶段10（CAN／CanIf／CDD来源身份接缝，fullgen仍pending）：共享Can.c新增缺省0的CAN_CONTROLLER_ID／CAN_CANIF_CONTROLLER_ID／CAN_TX_HOH，保持旧Can_ConfigType与所有公开函数ABI；标准driver入口用driver ID，所有Mode／BusOff callback用abstract CanIf ID，Can_Write及Can_TransmitPdu内部host adapter统一使用configured HTH。公开Can.h仅修正旧“one handle zero”说明，不改类型／签名。CanIf_ConfigType新增真实driver/controller／RxHOH／TxHOH配置，所有标准请求／查询、方向控制、transmit、mailbox与host Rx适配消费对应身份。SWS_Can_00234及CANIF00218明确Mode／BusOff callback使用abstract CanIf ID；Rx按SWS_Can_00279直接规定Mailbox包含abstract CanIf ControllerId，SWS_CAN_00496的Can_HwType字段表与CANIF00006的Mailbox参数表共同核定其类型和controller归属，不能接raw driver ID再重新翻译。CDD使用实际CanIf配置查询核定可用controller，并经真实CanIf_GetControllerErrorState查询driver，不复制raw driver ID到provider config。新API签名／service0x4b／CanIf.h及实际映射按91001、拒绝约束按00898／00899核定；selectedDevErrorDetect=false不报告developmentDET，拒绝仍E_NOT_OK／OUT保持。

独立原件复核纠正阶段7的配置标准结论：ECUC_Can_00316要求driver ControllerId从0连续，SWS_CANIF_00653要求CanIf ID范围从0开始；当前两命名空间各只有一个controller，实际supported source必须都为0。ECUC_Can_00326的共同HRH／HTH范围建议从0无gap，首批supported source限定两个不同HOH为{0,1}，允许Rx1／Tx0合法交换。此前Can7／CanIf9／HTH17成功只证明metadata range／内部一致性，未核对连续编号语义，不能保留为所选标准profile合法证据；该历史失败现由normal definition与trustedplan共同拒绝CAN_CONTROLLER_ID／CAN_HARDWARE_HANDLES关闭，原metadata保持官方宽范围，不将selected限制伪装成官方定义范围。合法变名／UINT8 IDs／UINT32长度／真实period及HOH交换仍接受。冻结AC不要求非连续ID，不扩展多controller或R11。

non-equal独立module消费者保留driver7／CanIf9／RxHOH11／TxHOH17为明确的机制验证，不能从正常source生成该配置或写为selected profile验收。它使用实际shared driver，覆盖mapped controller／errorstate、ACTIVE／BUSOFF／真实恢复、unknown／NULL拒绝OUT保持、错误provider／mailbox身份拒绝、真实callback9控制mode及Rx／HTH17 Tx，另真实Can_TransmitPdu输出证明host adapter同样消费configured HTH。正常selected polling消费者使用原source Rx0／Tx1，实际零DLC／Read／Write／BusOff轮询向量继续运行；其他独立module fixture的重合HOH仍仅是模块机制，正式PB config不能复制其值。旧native CAN入口回归仍缺省0。

本阶段按既有ABI及第三方来源身份审阅后保持Can.c／Can.h固定CRLF，正常assets update仅刷新这两个已登记来源SHA；未以摘要追认新CanIf config ABI或宣称fullgen交付。所有configured宏／PB配置仍须由trustedplan实际生成，source、catalog／BSWMD／owner与多slot／RTE／sealed正常入口闭包仍pending。下一focused runtime切片须关闭Dcm_00338 GetSecurityLevel、00339 GetSesCtrlType、00520 ResetToDefaultSession在Dcm.h的alwaysE_OK公开契约，消费实际selected session／locked security／S3状态；固定R24-11当前API表未定义旧Dcm_GetActiveProtocol，不从旧版本记忆重引入。未选择Authentication／BndM／VIN／DoIP不扩展，其他适用接口继续按原件审查。所有Tasks及8.3–8.5保持。

阶段10最终验收：固定LinuxGCC13／objdump／git／Python正常native multi_component_contracts36passed、multi_runtime_contracts8passed，无ignored／filtered；native比default多一个真实generated header typed编译链接消费者。最终新增错误mailbox／provider／HTH拒绝后native8再次全部passed。旧normal native standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame 1passed／41filtered。normal default同轮17unit／71builtin／2c_analysis／35multi全部passed，default0native不计行为证据。Clippy native两个test目标passed；quality core36／runtime57formattingfiles与Csyntaxpassed；最终assets0changes／diffcheckpassed。fixedMOD metadata未变，复用既有oracle。

受影响actualsingle C分析重新由正常c_analysis生成五个sealed样本，生成测试1passed；对新standard-ecu使用固定Cppcheck2.21／同源固定addons／GCC13正常c-check，42units／host-batch和legacy-probe完成，summary.error=null、无incomplete／syntaxError工具诊断。最终1808diagnostics、两个programexit1、passed=false；Can.c仍相同三条20.1及十三条8.7，未新增此文件诊断，不以标准公共API或MemMap原因自动申请偏离。actualmulti尚未正常生成，不能以single分析代替；没有逐条人工221项评估／授权主文，不声明完整MISRA符合。

历史验证失败保留：首次cargo fmt全仓check揭示既有无关format差异，未重排无关文件；对本阶段Rust文件误用edition2021使import等偏离项目edition2024 formatter，corequality真实拒绝，改按实际edition2024仅处理相关文件后通过。未关闭格式门禁或改变业务测试断言。AGENTS在现有AUTOSAR契约条目合并metadata range不替代连续编号／实例数量语义、机制-only与合法source证据区分的简短教训。阶段10源码与规格冻结交协调者独立审阅提交，尚未提交；完整Story Tasks与fullgen／Dcm剩余义务保持。


2026-10-09 阶段11（Dcm公开session／security与内部mode接缝，fullgen仍pending）：按00338／00339／00520在真正Dcm.h提供GetSecurityLevel／GetSesCtrlType／ResetToDefaultSession，always E_OK；selected DevErrorDetect=false时未Init或NULL getter保持OUT、未Init reset无状态动作。00977／00978的uint8类型与LOCKED0／DEFAULT1／EXTENDED3常量来自Rte_Dcm_Type.h。selected没有SecurityAccess服务，实际事实始终locked，getter不启用可选security machinery。会话getter消费真实session；public reset清S3／撤销已接纳request的pending session转换，保留进行中TP buffer直到真实confirmation，防止旧0x10成功确认或已接纳未处理请求重新激活旧会话。非默认idle reset释放真实diagnostic demand，进行中request仍持有至正常completion。

Dcm_01062要求reset调用SchM_Switch_<bsnp>_DcmDiagnosticSessionControl，00311要求0x10成功confirmation发布新mode，01670的S3恢复共享同一实际session transition。91019 modegroup的DEFAULT0／EXTENDED2与service session的1／3分开映射，不能直传session值。新增真正SchM_Dcm.h、真实SchM_Switch_Dcm_DcmDiagnosticSessionControl／SchM_Mode_Dcm_DcmDiagnosticSessionControl provider按Rte07255／07260实现；同一递归resource下实际存储provided mode，当前无connected mode-switch events时同步完成，没有空壳fixture函数／第二锁／第二scheduler。删除旧同名SchM_Dcm.h fixture避免遮蔽availability。此为所选BSW内部标准session mode publication，不放行多组件应用可配置mode ports／通信，原non-goal与拒绝范围保持。fullgen必须共同核定真实BSW scheduler prefix、provided／managed／accessed modegroup associations、mode type mapping、producer归属和actualgeneratedSchM，当前source adapter消费者通过不等于该元数据／生成闭包完成。

SWS_Dcm_00085内部读取0xF186使用同一actualsession，无caller DID callback；正常wire默认返回0462F18601、extended为0462F18603、reset再01。multi source application DID不得占用内部F186，两正常入口报DIAGNOSTIC_IDENTIFIER；旧profile行为不扩大整改。module Init也拒绝F186 callback配置，配置碰撞不能覆盖内部标准DID；原0x1234 callerread路径及多DID响应保留。

subfunction服务按00200／00201／00204 mask SPRMIB，仅0x10／0x3e应用；source实际SessionControl／TesterPresent的DcmDsdSidTabSubfuncAvail已经true、ReadDataByIdentifier为false，现有shared diagnostic inspector已经严格核验，新增两入口拒绝反例而不伪补来源。10 83成功extended且无positive frame，按00238／00240和8.10.3在无lower data confirmation时仍完成内部DSP confirmation／session publication；10 85及3e81 mask后仍NRC12 negative，3e80无positive，不能把抑制位当未知subfunction。00311 timing使用已核定两session同P2/P2Star来源，source不同timing仍由现有profile拒绝，不引入未支持session变体。

fullgen后续direct runtime闭包经固定官方原件另核定，须在完整8.2退出前关闭：CanIf_91003／91004 Rx／TxErrorCounter(uint8, uint8*)、0x4d／0x4e无optional availability，实际mapped driver host unavailable允许E_NOT_OK但OUT保持，不以常量空壳代替；00907–00910 invalidID／NULL仍受selectedDETflag控制。CanTp_00257／00260–00263 CancelReceive(PduIdType)、0x4c无Rx取消开关，真实physical FF/CF abort→PduR negative释放ownedbuffer，SF／最后CF waiting拒绝及lateFCconfirmation隔离必须验证；CanTpTc=false仅关闭Tx取消，Change／ReadParameter／version明确false不扩展。Com_00348／00861 TriggerIPDUSend(PduIdType)、0x17不是pull-copy TriggerTransmit；所选零minimumdelay／无callout／PERIODIC仍须实际manualtrigger经PduR，下层BUSY、stopped不存futuretrigger／uninit拒绝及source minimumdelay约束，不加hosttimer。

完整多槽source准备仍须保留两种身份含义：snapshot／guard覆盖全部caller source bytes和producer身份，但from_workspace必须继续验证原manifest磁盘raw bytes与guard source身份；不能统一canonical preparation_identity抹去原rawmanifest来源。三application与generated Dcm service桥归属、epoch0实际Mode完成、normalowner多RX／confirmation drainage、配置宏／PB、catalog／BSWMD、sealed AC-1–AC-8、multi c-check／CI仍pending。所有Story Tasks及8.3–8.5状态保持；不从旧release记忆新增GetActiveProtocol或未选择Authentication／BndM／VIN／DoIP。

阶段11最终验收：Linux固定GCC13／objdump／git／Python正常native38配置tests与8runtime tests全部passed，无ignored／filtered；最终增加module F186 Init碰撞拒绝向量后targeteddiagnostic再次1passed／7filtered。真实诊断链覆盖typed公开getter／reset、未Init／NULL alwaysE_OK输出保持、session1/3与mode0/2独立固定值、S3 mode恢复、queued／accepted未处理0x10 reset后成功confirmation不重新激活且ownedbuffer不可早复用、F186默认／extended／reset wire与原callback保持、10 83无positive但actual内部完成、10 85／3e81 negative与3e80 suppression。normal default同轮17unit／71builtin／2c_analysis／37multi passed，default0native不计行为；source内部DID碰撞及三个SubfuncAvail反例在normal definition／trustedplan均拒绝。正常Clippy两个native目标passed，quality core36／runtime59formattingfiles及Csyntaxpassed，assets check0changes／diffcheckpassed。

本阶段未改变已登记legacyC／交付bytes；复用阶段10实际single C分析完整但1808diagnostics／passed=false范围。新Dcm／SchM headers与producer仍未注册新profile assets，不能把assets0或typed模块运行当actualmulti sourceclosure、OS、c-check或Windows证据；没有授权主文／人工全项评估，不宣称完整MISRA符合。本阶段没有编译／运行失败记录需要覆盖；官方原件纠正相较旧实现的SPRMI、internalF186与mode publication缺项如实保留。源码与相关规格冻结交协调者独立审阅提交；后续CanIf counter／CanTp RxCancel／COM trigger合并一轮必要runtime闭包，再优先actualmulti生成／OS高风险验收，完整8.2目标不缩减。


2026-10-09 阶段12（合并所选通信API闭包，actual生成／OS仍pending）：CanIf.h的91003／91004 Rx／TxErrorCounter服务0x4d／0x4e真实映射configured driver ID后调用已有Can_GetControllerRx／TxErrorCounter；当前host counter unavailable按标准返回E_NOT_OK并保持OUT，未写常量成功或另造counter状态。独立typed消费者覆盖未Init、unknown／NULL、机制-only CanIf9→driver7的ACTIVE／BUSOFF／实际恢复与OUT sentinel保持；映射调用的来源另由代码审阅核定，unavailable结果本身不冒充可观察硬件counter证据。00907–00910 development报告仍按selectedDET=false关闭。

CanTp.h提供00257／00260–00263的实际CancelReceive(PduIdType)。accepted FF在等待／发送FC及尚有多于一个CF时可以取消，实际PduR_CanTpRxIndication(E_NOT_OK)释放Dcm reservation；返回E_OK后state立即IDLE，保留已接受lower FC至真正confirmation以避免旧确认归入新连接。最后CF的拒绝边界是FC真实confirmation后启动N_Cr且remaining<=7，不能仅从FF总长推导：独立真实driver消费者覆盖remaining7／8 × FC confirmation前／后的四组向量、unknown／未Init／idle／SF拒绝、取消后真实SF重用ownedbuffer以及旧FC实际flush后才允许新response发出。已无接收的有效ID返回E_NOT_OK并报告CANTP_E_OPER_NOT_SUPPORTED=0xA0（00352／00260），last-CF拒绝不取消reservation；不启用CanTpTc=false关闭的Tx取消或其他明确gated功能。

Com.h的00348／00861／00388 TriggerIPDUSend(PduIdType)、service0x17复用PERIODIC main的真实序列化／PduR helper，直接返回实际lower acceptance，BUSY／offline拒绝不排队未来trigger、不增加timer。00840规定未分配group的I-PDU在Init隐式启动；所选只有Rx group且Tx不在group，所以initialized Tx的COM_STOPPED不可达，不能制造Tx group以凑向量。消费者实际执行未Init／DeInit、wrong Rx／unknown ID、lower offline拒绝后重新ONLINE无旧trigger、Rx group stop不影响Tx、manual success／BUSY时copied旧payload保持以及第二次显式请求使用新值。正常source fixture显式ComMinimumDelayTime=0且无ComIPduCallout；两个normal入口拒绝nonzero MDT／callout。ECUC_Com_00181是0..1、0..3600秒且无default，00387是function-name0..1且无default，官方metadata保持该事实；未配置MDT表示没有该timer，不伪称官方default0。新multi目标白名单接纳实际零字段，旧profile白名单／行为不扩大。完整producer必须消费trusted configuration记录，不能从独立module fixture复制计时值。

阶段11保留F186拒绝曾复用“缺失或超出uint16”提示，61830合法却是标准内部DID，属于用户反馈错误。本阶段新增准确中英reserved/internal F186产品message；missing／out-of-range原语义保持，normal definition与trustedplan测试同时核对DIAGNOSTIC_IDENTIFIER及实际英文message，中文catalog核对保留语义。

fullgen生命周期义务由固定R24-11另核定，纳入同源RTE／SchM／OS生成，不另拆runtime阶段：Rte_Start／Rte_Stop（91137／02569／01309、91138／02570／01310）始终生成，Rte_Main.h／Rte.h公开typed接口；真正初始化／退役local与freshness资源，并按09035–09038在BSW与SchM初始化后启动、BSW shutdown前停止。当前无InitEvent／RteInitializationRunnableBatch，不虚构应用初始化算法或仅该条件下的Rte_Init_batch／Rte_StartTiming（06750／06755）。SchM_Init(const SchM_ConfigType*)、SchM_Start(void)、SchM_StartTiming(void)、SchM_Deinit(void)始终生成（91170–91173／07271／04547／04550／07275，Rte_Main.h），消费真实配置／owner及OS激活资源；SchM_Start在BswM_Init前，StartTiming实际释放唯一OS的periodic BSW／SWC activation，Deinit在Rte_Stop之后／OS shutdown之前（09055–09057），trusted context／一次初始化约束必须真实保持。明确false的SchM version API不扩展；不能使用空wrapper或第二timer／scheduler替代。

公开多槽source API按trustedplan全部APPLICATION实例及producer owner派生；三application只是固定验收fixture，不是产品数量或名称硬编码，SERVICE DcmService无event桥继续由Rte source生成而非caller slot。snapshot全部caller字节／identity闭包与from_workspace原manifest raw字节身份同时保持。此前epoch0实际Mode callback、DM相位、多RX与confirmation drainage、PB／catalog／BSWMD／modegroup元数据、sealed AC-1–AC-8及actualmulti c-check均继续pending；所有Tasks与8.3–8.5不变。

阶段12验证：Linux固定GCC13／正常objdump／git／Python native39配置tests及8runtime tests全部passed，无ignored／filtered；标准新增API独立typed编译链接和上述真实状态／资源向量执行。固定官方MOD oracle1passed／72filtered，正常17unit／71builtin／2c_analysis全部passed；normal default multi38passed（无ignored／filtered，含两入口实际F186 message最终断言）。Clippy两个native目标通过，quality core36／runtime59 formatting及Csyntax通过，assets check0changes／diffcheck通过。未改变已登记legacy C资源，复用阶段10实际single分析完整但1808diagnostics／passed=false；actualmulti生成与其c-check仍未完成，不声明完整标准／MISRA或Windows验收。

历史失败保留：新增显式MDT0最初遗漏multi目标参数白名单，使18个有效配置向量被TARGET_PARAMETER_UNSUPPORTED拒绝；补齐同一来源闭包后原断言全部通过，未删字段或放宽测试。CanTp remaining算术首次在uint16独立fixture触发GCC -Werror sign-compare，按明确uint32运算修正后native8通过；未关闭警告。新增Rust失败断言一度误用unwrap_err要求成功plan的Debug，改为err().expect；所有失败都已定位并关闭。阶段12完成必要通信API合并后冻结交协调者独立审阅提交，随后优先actualmulti生成／OS高风险关口，完整8.2仍in-progress。


2026-10-09 阶段13实现中（未冻结，尚未完整生成／运行验收）：可信计划现按真实 runnable producer ownership 派生全部 APPLICATION 实例的 caller source slots，no-event Dcm SERVICE 桥仍由 RTE 生成，不固化验收样本三个应用或名字。RTE 候选实现具有各 R 独立初值／fanout、同步 CS、COM 实际 ReceiveSignal 值与 freshness 状态、STOPPED 优先于 NEVER_RECEIVED／MAX_AGE，普通 API 检查 actual owner；Start／Stop 分别采用真实 OS startup/shutdown hook 的 trusted context，重复 Start 拒绝。新增片段验证入口只是当前机制验证手段，完整交付时收敛到正常 prepare／render_ecu_sources 的私有 producer，不作为并列半工程产品入口。

本轮核定并修正 source/runtime 差距：旧 multi fixture 的 ComIPduSignalProcessing=DEFERRED 与实际立即 unpack／notify 不一致。依据 SWS_Com_00300／00301，首批 multi 明确 IMMEDIATE，fixture 两 PDU 改为 IMMEDIATE，两正常入口拒绝 DEFERRED；旧 profile 原行为与 source 保留，这不是实现 deferred 能力。唯一 EcucPartition 由真实 source 明确 ID0、Core0、全部 APPLICATION／SERVICE root-context instance refs；RteComUser 与 COM Rx／Tx main 显式指向同一 partition。官方 metadata 保留一般 multiplicity，首批 inspector 限唯一来源。普通 native grammar／definition validation 有限接纳并核验 ECUC_EcuC_00036 的准确 ROOT-SW-COMPOSITION-PROTOTYPE→SW-COMPONENT-PROTOTYPE 关系，其余未支持复杂 IREF 不自动放行。

SWS_Rte_05088／07422／07424 的 component 短名 scope 用于实际 RTE data/API memory allocation keywords；draft04615／07710的 COM callbacks 使用 source partition 短名，07427为 CALLOUT_CODE，实际映射到 controlled x64 sections，不仅放空 marker。全工程 namespace 在正常 validation 与 trusted plan 阶段拒绝公开 RTE／SchM／通信、实际 OS／target／FreeRTOS、main 和标准 C 全局 producer 碰撞。普通 lifecycle helper 是 additive controlled-host seam，保持 Ecu_Target 两文件 LF 与受信摘要一致；未改变 legacy 普通 task owner 检查。

尚未完成：所有 caller source snapshots／guard（包括原 manifest raw bytes 身份）与 sealed ownership 接纳、正常 multi 完整配置／catalog／BSWMD／SchM/RTE/OS 生成、唯一 owner bootstrap／tick／shutdown 接线、物理输出后实际 driver polling confirmation、多同 epoch RX、epoch0实际 STARTED通知且不推进 COM DM、configured周期推进 Dcm/CanTp而非 IO唤醒、真实 group Stop/Start(TRUE/FALSE) 的网络值／状态向量、AC-1–AC-8及 actual multi／受影响 single c-check。模块和源码片段不能替代以上关口；全部 Tasks 继续 unchecked。


阶段13补核：07592 Note允许无assigned SwAddrMethod的AutosarDataPrototype选定段名，仍保留05088的真实关键词／section义务。05089／05090还要求共同生成的真实 BSWModuleDescription MemorySection 描述usedsegments／attributes，以及 BswImplementation.generatedArtifact 关联实际工件；source注释或JSON catalog不能替代。现有native类型消费者已追加实际RTE源码编译与objdump，含变名及scalar／void CS形状，实际组件text／bss与partition CALLOUT_CODE存在；仅为编译／段属性证据，未执行RTE行为。固定MOD新增metadata oracle通过，默认42配置全通过；随后额外Os_Types碰撞向量与完整native复验仍待必要最终检查。

queued confirmation实际接线选择：保持宿主write／flush后post原kind2，owner验证实际output slot ticket／pdu／state并retire后，以明确Can host completion adapter发布driver TX-complete事件，再由实际Can_MainFunction_Write通知标准CanIf；owner逐项立即poll-drain避免覆盖单pending handle，并保留source-derived周期Write调用。不得切同步或在接纳队列时确认，adapter不另带payload队列。真实Can_Write→queued sink→flush→owner completion→Write polling及STOP/BUSOFF／late cancellation仍待完整normal工程向量，不以手工adapter调用伪造成功。


阶段13 queued接缝已开始接线（仍未完整生成）：shared Can.c 新 controlled host token API 仅在明确 CAN_HOST_QUEUED_COMPLETION=1 生效，缺省关闭保留旧路径。实际 Can_Write 接纳分配 lifetime 单调 token，flush 内实际 output sink 捕获；STOP／busoff／DeInit／re-init取消边界使旧 token 无法消费同PduId新 outstanding，token不在Init归零。completion独立保存确认 handle，不覆盖尚未flush的新frame handle，busy拒绝不覆写，实际MainFunction_Write才通知标准CanIf。新 Ecu_Target profile 分支在私有output slot保存token，复用TakeOutput／ConfirmOutput／consume三重FIFO与ticket／Pdu proof；实际物理write／flush后kind2才retire并发布／poll，取消token丢弃。公开输出格式未增加字段；高水位token要求FIFO，正常owner已严格实施，adapter并不为任意token/Pdu伪造proof。

独立COM链消费者已经实际queueddriver→sink／flush→completion→Write poll运行，涵盖busy不覆写、重复token、STOP和busoff后的same-ID复用旧晚确认不得消费新请求、re-init后token不重用；native8通过，root独立复验native8与配置native43通过。该证据不是完整owner物理stdout／TP/Dcmbuffer复用证据：generated flag、bootstrap/timing/quiescence及正常sealed同IDbuffer向量仍pending。

生命周期拒绝修正：不能依赖fail()中ShutdownOS在非法hook/interrupt上下文不返回。controlled Ecu_TargetCheckLifecycleContext按TRUE/FALSE分别检查真实STARTUP/SHUTDOWN并返回状态，generated Start/Stop拒绝后立即返回标准table的RTE_E_LIMIT，不继续变更资源；普通RTE API直接拒绝非actual owner并保持OUT，void COM callback非owner无动作。actualRTE C原名／变名及scalar／void producer编译和scopedsection objdump再验通过；完整BSW/SchM/OS顺序仍pending。


完整生成期间核定的 TP 身份接缝：CanTp own Rx／Tx NSdu IDs（ECUC_CanTp_00301／00268、SWS_CanTp_00212）与 PduR callback IDs（ECUC_PduR_00311／00322，SWS_PduR_00507／00518／00375）属于不同 namespace，按共同 canonical Pdu refs 绑定；DcmDslTxConfirmationPduId（ECUC_Dcm_00864）也独立于 Dcm 请求 PduR 的 router ID。原模块消费者虽有分层不同handle，CanTp own NSdu／router callbacks 和 Dcm own Tx／router请求仍同值，未覆盖此差距。当前配置结构已拆 CanTp upper_receive／upper_transmit 与 Dcm router_transmit，标准公开函数签名未改，真实调用按各自实际配置翻译；不是修改原 source ID 抹去差距。独立真实诊断链现在使用 CanTp own11／12、PduR callback71／72、Dcm own17／18及router请求81，严格native实际编译链接运行1passed／7filtered，旧buffer／mode／timeout／confirmation向量继续有效。固定LF交付摘要按这项已审阅自定配置ABI更新，完整source→PB mapping／正常工程仍pending。

多槽准备正在接入正常入口：source按实际instance匹配trustedproducer槽，物理文件经既有read_source／链接／size／UTF8检查，精确成员数及canonical file复用拒绝，所有字节及路径纳入guard；live application/<type>.c映射sealed src/<type>.c。原manifest raw字节身份作为不可变seed保留，guard revision同时覆盖完整canonical consumer快照。latest单元18tests实际通过，含全部application与rawmanifest相互独立身份。ledger结构证明从真实sealed ARXML重建同一multi符号producer，验证实际type／instance／Rte binding的DEST及target；不接受ECUC extension定义、不把归属证明写成definition验证通过。native accepted catalog与完整再生成字节比较仍由prepare／reopen负责，尚未完成正常multi renderer／owner及全部验收。

阶段13正常完整工程已取得运行里程碑，仍未冻结／未完成8.2：多槽 caller bytes 经正常 prepare、封存、离线 CLI C99 build 和 production HostBatch／唯一 OS owner 运行，精确比较全部输出 epoch／ID／DLC／payload、全部COMMIT receipts与重复epoch。初始DID在epoch1为00000000，epoch10／20为12345678，TX321各一次；epoch0双帧接收前真实CAN STARTED polling callback已发布，COM DM周期没有启动偏移。root独立复验首个正常owner测试通过；之后本轮实际精确记录测试再通过，未将之等同全AC。

有效来源模型改为实际selected标准契约，不复用legacy17项ABI：source entry与references迁至TPS_BSWMDT_04016／04017规定的/AUTOSAR_<module>/BswModuleEntrys；独立签名／direction／synchronous／reentrant核验，actual runtime/multi与shared CAN受信字节及实际multi_ecu／multi_rte／artifacts／ecu／contracts编译期producer指纹同属runtime_sources。PduR_ComTransmit的实际declaration owner为PduR_Com.h，不能记录不存在的PduR.h声明。实际COM／ComM source名void wrappers定义在Ecu_RuntimeConfig.c，无fixture fallback；新ComM position0及CAN Read／Write／Mode／BusOff positions9–12由同一RteBsw source rows与1ms周期派生，APP4／5／6、COM7、Dcm8不变。Rte position0依据ECUC_Rte_09068范围0..65535合法，legacy非零要求保持。

标准metadata按当前TPS和固定XSD建模：RTE08404生成AUTOSAR_Rte，05177 implementedEntry包含Rte与SchM真实lifecycle／COM callbacks；05179／05180的旧role文字与当前TPS BSWMDT表4.1／4.17不一致，采用明确replacement后的Description implementedEntry／expectedEntry及独立BswModuleDependency targetModuleId／targetModuleRef，未生成已移除role或非法dependency子元素。Rte2／Com50 reciprocal实际依赖，模块ID依据固定General BSW Table13.2核定。CanIf/PduR Transmit与COM Send／Receive的不同对象reentrancy按实际表；COM Send异步，COM callbacks91123／91127同步非reentrant。Dcm新增实际Reset／TpConfirmation／ComM callbacks的metadata使用独立SWS同步／reentrant表，Init非reentrant。Dcm00806 provided session prototype及实际Init／Reset／Main／TpConfirmation managed发布、module-local canEnter areas正接入；无实际production读取不虚构accessed消费者，现有Mode getter作为有意义可用接缝，07261的条件式存在要求不宣称其在本profile强制生成。完整mode/type／service映射与metadata最终schema／reference验收仍pending。

生成配置按SWS_MemMap_00072实际CONFIG_DATA_PREBUILD_UNSPECIFIED类、component／partition／module scopes和controlled section属性映射，实际BSWImplementation ResourceConsumption MemorySections／prefixes与generatedArtifacts关联真实工件；MemorySection并非Description非法直系child。含pointer relocation的prebuild配置从导致assembler incorrect-section-attributes警告的.rodata改为.data.rel.ro，配置分类保持；最终Linux链接GNU_RELRO／实际段和Windows目标内存属性仍须核验，不宣称跨平台只读保证。

网络初始化纠正：selected communication inspector明确ComSignalInitValue=0，producer必须使用该COM source值，不能以R/P ComSpec初值替换。RTE06009／06010 receiver保持自身init至真实接收，sender init不直接由RTE使用／不虚构startup Write；06830 stopped回填RTE实际last/init。新正常sealed --control-source owner探针实际R init7／COM0，未接收group Stop／Start(FALSE/TRUE)仍RTE7、COM0，非ownerRead保持OUT且RteStart/Stop返回LIMIT；在epoch31 main之前或之后真实RX42，epoch60仍E_OK、61超龄，未提前减新counter。local R init7／9也在owner观察。该测试本地与root独立均通过（root1passed、45filtered、无ignored，9.09s）；不把filtered写全验收。

完整生成的optional边界也已实际运行：保留诊断物理route但无用户DID时，真实nullable read config允许内部F186读取默认session1且未配置1234返回NRC31，COM周期仍实际输出；无诊断routes形状保留原始未用DSL/CanTp/BSW source描述，但有效plan、owner调用与generated BSWMD timing events均移除未初始化的CanTp/Dcm周期，不伪造routes或调用uninit mains。两形状经正常caller-source封存／CLI／productionHostBatch通过1test（2真实完整工程，17.05s）；partial route仍必须拒绝，destination-only负向验收待补齐。

当前normal c_analysis入口已产生第六个实际完整multi样本，CI按两个目标四组并行保留原五样本与新增multi、十二个结果摘要及分析complete/source_passed区分；最近样本生成1passed／1filtered、12.56s，不能当作Cppcheck或MISRA通过。历史失败保留：position0的旧非零检查曾拒绝28有效向量，source rows与合法scope修正；新memory CODE scope未登记曾使实际C编译失败，补齐同源mapping后Werror正常通过；external control初次试图调用未交付private Can_Lock声明失败，改用已有host atomic接口复制观测，未增加产品API；future batch completion等待tick曾使探针超时，改为正常PostFrame+owner观察publication验证pre-main，不修改batch契约；optional native test一度在临时ProcessOwner被drop后写stdin得到BrokenPipe，修复测试owner lifetime后两工程真实通过。未关闭warning、削弱assert或更改独立预期。

剩余完整8.2：owner local fanout／同步server执行上下文与逐应用周期count／顺序、DM Disable/Enable及真实接收后Stop/Start向量、late同ID queued TP/Dcm buffer、multi source guard全部拒绝与determinism、完整mode/exclusive-area/dependency/memory/artifact语义与固定schema、actual multi及受影响legacy c-check、normal全回归与最后bytes／assets审阅；temporary public RTE fragment入口交付前收敛。所有Tasks保持unchecked，8.3–8.5和完整标准／MISRA／Windows验收不借本里程碑改done。

2026-10-09 阶段13后续实证：root独立optional-DID／network-only完整sealed工程通过1test、46filtered、0ignored、17.30s，关闭先前MODULE-ID grammar中间态失败。保留真实Linux链接证据：GNU_RELRO VA[0x24790,0x25000)，.data.rel.ro section26 VA0x247a0 size0x456位于其中；实际20项runtime配置对象／数组以及Ecu_SchMConfig、Ecu_OsConfig均在该段。此为当时实际Linux产物的重定位后只读放置证据，不推出Windows属性或后续修改自动通过。root旧generated快照c-check正常exit1、error=None、50unique TU、49host-batch＋49legacy-probe、2580诊断／692 adopted、source_passed=false；两个error与旧single样本同源（mmap fd模型／kernel IDLE边界），不抹除。实际generated wrapper缺兼容声明的R8.4已通过include真实SchM_Com.h／SchM_ComM.h修复；最终修改仍需重新生成分析。

Dcm模式生产归属修正：selected session mode storage及SchM_Switch／Mode真实实现移入generated SchM.c，并使用有实际Service时该Atomic type的CODE／VAR_CLEARED scope、无Service时Dcm BSW fallback；旧module consumer／legacy adapter通过显式ComStack_Cfg.h选择保留，避免宏未载入导致双重定义。getter保留为可用但production Dcm未使用的表面，不声称Rte07261强制生成或虚构accessed consumer。增强owner测试独立观察runnable／server计数、执行顺序、owner／调用上下文、local fanout、NULL拒绝输出保持和重复同epoch RX；另实际canonical RX0x12345678→local Result、CS OUT／INOUT均0x12345679且成功状态E_OK。normal native exact本地1passed、47filtered、0ignored、9.01s；较早count版本root独立1passed、46filtered、9.04s。filtered范围不写成全AC。

固定R24-11 Dcm01327／00777／00807／00781及91021／23／27／31、91034／35／37／39、91022／24／28／32明确兄弟provided groups和固定Service端口声明；端口Variation无SID27／11／85豁免。selected source及generated BSWMD补齐EcuReset（NONE0及标准1..6）、RapidShutdown（ENABLE0／DISABLE1）、DTCSetting（ENABLED0／DISABLED1）、Security（LOCKED0，无配置security rows故不虚构1..63），均EXPLICIT_ORDER、transition255与真实初值。Service存在时逐一同步相同groupType的真实PPort／prototype；无对应Service时按TPS5.11不虚构SwcBsw mapping，BSW声明保留。只有session有实际managed entities及Switch／Mode producer， dormant groups无虚构访问、切换API或可选诊断处理器；CommunicationControl／ROE无配置channel／event故无实例。通用application mode仍拒绝。新normal definition＋trusted plan固定session mode来源拒绝向量已通过1test、43filtered；兄弟声明及最终schema／owner全矩阵仍按实际后续结果核验，所有Tasks保持unchecked。


阶段13继续闭合所选来源与正常工程验收：DCM01477/01478/01480要求每配置connection真实authentication state及provided group，未找到SID29豁免。首批无persist规则／SID29，Dcm_Init实际初始化DEAUTHENTICATED并调用source-derived DcmDslMainConnection短名（Physical）的SchM publisher；91067/91074/91075的两状态声明、固定接口／PPort、type mapping和Service同步mapping实际派生，未增加authentication服务、持久化或虚构生产访问者。独立module consumer观察Init前255→Init后0，正常8 native runtime消费者0failed/ignored/filtered、1.00s；完整owner包含该真实生成publisher的exact本地通过9.54s。固定mode初值引用严格要求该group内真实成员，跨group同短名合法ref已在两个normal入口拒绝。

RTE05180／TPS04017的实际Os依赖补齐：Rte_OsService.c调用GetCounterValue／GetElapsedValue，生成canonical Os entries、Rte expectedEntry和targetModuleId1真实ref，签名及OS00383/00392同步／重入属性独立核定。TPS constr10260–10264要求callType、executionContext、reentrant、synchronous、swServiceImplPolicy；新multi显式STANDARD和CONCRETE。bswEntryKind本身可缺省concrete，不能把其显式输出误记为该列表强制项。Rte COM callbacks、Dcm Tp confirmation及ComM mode callbacks输出CALLBACK，实际周期mains输出SCHEDULED，普通GetCounter/Elapsed/reset输出REGULAR。独立正常prepare metadata断言1passed／45filtered、1.38s。其它所选通信调用依赖仍需同源闭合，未以这些局部条目宣称全部完成。

正常owner DM矩阵已加入真实epoch0 RX21：逐周期1..29返回E_OK且值21，周期30返回MAX_AGE且保留21，与before／after／canonical／controls同一sealed工程一起执行，本地exact1passed／48filtered、9.61s。此前controls扩展版本root独立exact1passed／48filtered、10.14s。zero ComTimeout由既有真实COM consumer第三PDU实际30／50周期无超时观察及normal source aliveTimeout/ComTimeout同改0双入口通过验证；该owner快照配置30ms，不声称source拒绝zero。

实际变名和源序验收：Ingress→Gateway、Process→Compute、ResultService→Calculation、Transform→Calculate，全部真实caller bytes同步修改，ARXML输入逆序后沿normal plural prepare／sealed CLI build／production HostBatch执行，固定完整epoch／CAN ID／DLC／payload／次数与重复COMMIT无重放断言通过，与原工程合计exact1passed／49filtered、16.72s。历史第一次测试错误地按substring替换Process，损坏官方Processing定义引用并被正常validation拒绝；改为实际用户identity／symbol匹配后通过，没有改变官方定义或关闭拒绝。临时rte_runtime_files收为pub(super)，原三项测试改从normal完整prepare消费实际工件；生成组件和partition独立MemorySection身份保留，目标代码段可合并到.rte_code。全8.2仍in-progress，late同ID queued TP/Dcm、完整调用依赖／最终schema和actual c-check／全回归与bytes/assets仍未完成，不更新Tasks为done。

2026-10-09 阶段 13 收敛验证（完整最终门禁仍待刷新）：正常 owner 已覆盖六相位 before／after／canonical／controls／epoch0／late。epoch0 真实接收在 30ms 变为 MAX_AGE；canonical 同时观察本地 Result、C/S OUT／INOUT 为 0x12345679 和 Rte_Call E_OK；late 在实际输出未确认时拒绝推进周期／接收输入，以真实 STOP callback 释放 Dcm buffer，旧帧真正 write／flush／确认后再接纳并确认同 PDU 新响应。独立 queued TP 消费者进一步验证旧取消 token 不释放已复用的新 Dcm buffer，新 token 实际 Write polling 确认才释放：新增正常 native case 1 passed／8 filtered（0.57s）。六相位本地 1 passed／50 filtered（10.02s），协调者独立 1 passed／50 filtered（10.49s）。协调者同一稳定快照完整 default 19 unit／71 builtin／2 c_analysis／47 multi 全部通过，无 failed／ignored／filtered；此证据包含 Auth、PBcfg 和 canonical-path／flags，不替代最后修改后的最终检查。

完整生产 HostBatch 工程同时验证正常身份与 Gateway／Compute 等真实变名、ARXML 源序反转，两者均逐条比较固定 epoch／CAN ID／payload／count 和重复 COMMIT 无重放，PBcfg 拆分后本地 exact 1 passed／50 filtered（16.98s）。最初盲替换 Process 误改官方 Processing 定义导致 REFERENCE_UNRESOLVED，已改为实际组件身份替换，保留该失败而不放宽解析。rte_runtime_files 收为内部入口，原三项片段测试改从正常 plural prepare 消费工件，独立签名消费者 exact 1 passed／49 filtered（4.57s）。

适用 LSduR_00035／PduR_00241 的真实 PBConfigType producer 已分别拆至 LSduR_PBcfg.c／PduR_PBcfg.c，配置对象、route arrays、声明头、MemMap CONFIG_DATA_PREBUILD 属性和 BSWMD generatedArtifact／模块所属 MemorySection 共同迁移；首批仍预编译选择，不支持 ONLINE 再 Init 切换配置。标准 LSduR 的 moduleId 为 132，Arti 为 5，不能归入 CDD255。所选 LSduR 的无 metadata／非 zero-cost／同 partition／一对一路由政策由真实路由派生，不接受额外 LSduR ECUC module override；两正常入口已有拒绝向量，destination-only diagnostic routing 亦拒绝而不伪造 route。该 exact 1 passed／47 filtered（1.26s）；第一版删除 source 而遗留 route 引用只获得 REFERENCE_UNRESOLVED，修为完整有效引用反例后核验 PDU_ROUTE_NOT_UNIQUE。

生成 BSWMD 的实际标准 imported interface 闭包包括 Rte→Com／Os，Dcm／CanTp／PduR／LSduR／CanIf／Can／ComM／BswM 的真实调用与 callback，全部 refs 按完整 canonical path 和 DEST 核对，固定 sync／reentrant／callType 预期独立来自 R24-11 表；不同对象可重入不许可同一对象无同步并发。正常 metadata exact 1 passed／46 filtered（1.32s）。源 multi BSW entry 现显式声明 STANDARD policy 和 CONCRETE kind；catalog 拒绝缺 callType／executionContext／sync／reentrant／policy、错误实际 callType、MACRO policy 或 ABSTRACT producer，native grammar 按固定 XSD 枚举接纳。constr_10260–10264 强制 policy 等属性；bswEntryKind 的 CONCRETE 默认仍与强制存在区分。新增两入口反例 exact 1 passed／51 filtered（8.39s）。首次 fixture 编辑只补 UNSPECIFIED 而漏 TASK，导致 CanIf_Transmit BSW_SIGNATURE_CONFLICT；已补齐全部源 entry，未调整拒绝预期。旧 profile 不套新增必填 guard。

Auth connection 实际改名 Physical→Link 正常生成真实 publisher 和 source-derived service mapping；非法 authenticated 值／缺 PPort 两入口拒绝，exact 1 passed／46 filtered（2.46s）。standard session 名称拒绝反馈现有准确中英消息，未改变 legacy 通用 session 判断。共享 Can_GeneralTypes 旧“唯一 handle zero”注释修为实际配置身份，无 ABI 改动；最终固定换行和资产摘要仍须在最后字节状态核对。全部 Tasks 仍待最终六样本／schema／实际 multi 和受影响 single c-check、完整 native／quality 验证后勾选，不声明完整 MISRA 或目标硬件符合。

2026-10-09 最终稳定源码快照常规验证：正常 native-tests 完整 52 multi_component_contracts（36.79s）和 9 multi_runtime_contracts（1.26s）全部通过，0 failed／ignored／filtered；包含六相位实际 owner、真实变名／源序、完整与无用户 DID／无路由工程、独立 typed producer／module consumers。正常 default Cargo exit0：19 unit（0.37s）／71 builtin（21.32s）／2 c_analysis（13.36s）／48 multi（33.03s），全部通过、无跳过；默认未编译 native-only cases，不能以其 runtime 0 cases 代替已执行的 native9。core quality 56 格式文件、runtime quality66 格式文件及 C syntax 均通过；Python 正常44 tests 和 Ruff 通过；Clippy 按项目既有 correctness／suspicious 策略、all-targets＋native-tests 通过（12.92s）。第一次 core quality 因 PATH 无 rustfmt 拒绝，补正常 Cargo PATH 后通过；额外 -D warnings 探查停于 baseline 未改 build.rs 的 too_many_arguments style finding，未改实现或既有策略来掩盖。最后 source bytes：82 个固定 EOL 文件核对通过、assets check 0 changes、git diff --check 通过。协调者仍在最终六实际样本／固定 XSD／multi＋standard-ecu c-check；这些与人工 MISRA 全面评估保持分开，Tasks 尚未勾选。

最终独立生成与标准 metadata 关口：协调者在全新正常 AUTOSAR_C_ANALYSIS_SAMPLES 目录生成六样本，c_analysis exact 1 passed／1 filtered（17.31s）；实际35份 ARXML（capacity1／diagnostic1／multi12／signals1／standard-ecu10／user-application10）均通过固定 R24-11 XSD，官方 ZIP SHA 和解出 XSD 原始字节重新核对。multi Host_Implementation 的645个完整 path＋DEST refs 均解析到真实生成或封存输入对象，0 errors；初版只读 collector 将 XML MODE-GROUP 标签误当 M2 类，按固定 XSD 的 MODE-DECLARATION-GROUP-PROTOTYPE 修正，未修改产品内容。最终多组件封存 CLI 正常 host-batch build exit0，实际 Linux ELF GNU_RELRO VA[0x24770,0x25000)，section26 .data.rel.ro VA0x24780／size0x472 全在范围内；实际 LSduR_Config／PduR_Config／route arrays／Ecu_OsConfig／Ecu_SchMConfig 位于该段。这是 PBcfg 拆分后的 Linux 重定位只读证据，不推导 Windows 内存保证。

最终 actual multi c-check 使用固定 Cppcheck2.21.0（binary SHA256 434155fc2a092b4dd98042052fef93a501111bb18849e5b1db05e8854f8ba795）及已核对 addons，正常命令 exit1、error=None，52 unique TUs，host-batch／legacy-probe 各51 TUs／exit1；工具及 addon receipt／TU 完整性成立。原始2608 diagnostics：2574 style／30 warning／2 error／1 portability／1 information，692 adopted，source_passed=false。两 error-severity 仍为既有 mmap fd=-1 平台模型和 adopted kernel IDLE bound；非style项目诊断仍在原 Os_Time depth／Os_Backend pointercast／mmap 模型路径，未新增实际 producer warning／error。新增 Ecu_RuntimeConfig／Rte／SchM／PBcfg 可见兼容声明的 R8.4 缺口已关闭。原始诊断完整保留，未 suppression／追认偏离；200 rules＋21 directives 自动/人工 assessment 仍 not_assessed，不声明完整 MISRA／源码符合。最终受影响 single c-check 尚在运行，结果待追加。

最终 single standard-ecu c-check 已独立执行完成：正常 exit1／error=None／source_passed=false，42 unique TUs；host-batch／legacy-probe 各41 TUs、各 exit1。1814 diagnostics＝1780 style／30 warning／2 error／1 portability／1 information，691 adopted；固定工具与扫描完整性成立。两 errors 与 multi 同源；只读核查 adopted IDLE 字符串拷贝读到第五字节 NUL 即 break，Os_Time depth guard 为原配置验证／断言路径且本 profile 无 counter-chain action，mmap 原 MAP_PRIVATE|MAP_ANONYMOUS／-1 调用与模型判断有差异。这些局部分析保留原始诊断，不作为关闭全部诊断或完整 MISRA 通过的依据。

冻结前最后命名审阅确实发现 component／partition memory scope 会与实际 Rte／Ecu／ComM 等固定 producer 合并；有效引用的 Rte 组件向量在修复前正常入口被接纳。只修改 Rust source validation，按实际已交付12个 runtime scope 拒绝 component type 和 EcucPartition 同短名，源位置返回 CONTRACT_NAME_COLLISION，不改生成模板／C／资产。独立两入口反例 exact 1 passed／48 filtered（12.74s）。首次测试还同时改 ECUC instance 容器短名，Com 向量只得 OBJECT_DUPLICATE；修为仅实际 component type 及对应完整 type refs，排除无关重复路径后核验原碰撞，不弱化预期。

新增 guard 后正常六样本重新生成 exact 1 passed／1 filtered（12.82s）。792个完整交付文件中35个来源身份／seal／编译后 verifier metadata 随真实校验源变化，未冒称 seal 相同；所有599份 C／H／ARXML 逐字节相同，六 target 的 ABI、编译参数、includePaths、sources、kernel source／patches、linkLibraries、nativePort、requiredSections、scope／target／toolchain 均相同，因此以上 XSD／实际 C 分析／ELF 放置证据按同一 C 与构建范围复用，不把旧 seal 追认为新 identity。core quality 再次通过（56 files）；正常完整 native／default 正在最后刷新，产物来源再次冻结。

2026-10-09 阶段 13 最终实施冻结：最后 memory-scope guard 后，正常完整 native53 multi（40.37s）＋9 runtime（1.17s）全部通过，无 failed／ignored／filtered；正常 default19 unit（0.44s）／71 builtin（19.71s）／2 c_analysis（12.64s）／49 multi（30.75s）全部通过。core quality 最后刷新56 files／C syntax通过，项目既有 Clippy correctness／suspicious all-targets＋native-tests最后刷新通过（7.11s）；runtime quality、Python44／Ruff 和固定 EOL82 files 复用未变来源的既有通过证据，assets最终check 0 changes、diffcheck通过。协调者另独立核对792完整inventory／599 C-H-ARXML原字节及两 actual target17个存在的 build-contract字段一致；旧分析 summary 的 sourceSeal 不变，复用仅对应完全相同 C／build scope。四项8.2实施Tasks已满足并勾选，frontmatter仍in-progress供正常独立审阅；尚未声明review完成、提交该最终切片、Windows实际执行、硬件／完整AUTOSAR或MISRA认证。8.3 live初始化／再生成和8.5完整异地交接继续各自未开，不纳入8.2完成声明。


独立实施审阅与矩阵核验：主代理已完整读取自 baseline 的统一差异（含阶段提交与新增文件），逐项核验四项 Tasks。通信／任务行由 sealed_multi_project_builds_and_runs_production_owner 的原名、变名、倒序输入和实际 HostBatch 输出覆盖；初值／隔离与同步调用行由 sealed_multi_owner_preserves_network_init_and_deadline_phase 的 canonical／controls 实际 owner 观测覆盖；Rx DM 行由同一测试 before／after／epoch0／controls 与 com_standard_consumer_executes_group_and_reception_monitoring 的独立多 PDU／零 timeout／重复到期覆盖。非法连接、类型、调度及 group／callback／timebase 行由 multi_component_contracts 的两入口拒绝向量覆盖；公开接口行由 generated_headers_compile_and_link_independent_scalar_and_void_server_signatures 和全部九项 multi_runtime_contracts 覆盖；来源行由 plural_application_preparation_freezes_every_real_producer_and_refuses_bad_members、实际生产者／metadata 测试和既有 builtin／c_analysis／Python 回归覆盖。上述 covering tests 均在最终完整 native53＋runtime9／default19＋71＋2＋49 中实际执行通过，0 ignored；native-only 测试以 native 输出为据。实际生成 multi 与 single 的 C 分析完整性已核验，原始源码诊断与未完成人工规范评估保留。审阅状态仅表示进入独立 review，不代表 Windows／硬件或后续 8.3–8.5 已完成。

## Review Triage Log

三层独立 reviewer 已全部返回后逐项裁决；下列编号保留原发现，不在裁决前去重。修复仍待执行和验证，原实施子代理已不在可继续的 agent 列表中，按 Step 4 由主代理应用最小 patch。

| 发现 | 裁决 | 证据与处理 |
| --- | --- | --- |
| B1：N_As／N_Ar 到期保留 lower_pending 导致永久阻塞 | false | 超时释放上层但保留真实下层未退役确认，防止同 ID 复用误认。实际 CanIf STOP／BusOff 的 CancelOutstanding 经 LSduR 负确认清除该 token；正常 owner 也禁止未 flush 输出时推进时间。不能在仍有物理帧时无证据清除身份；补查超时后实际退役和恢复向量。 |
| B2：旧 FC 未确认时新 FF 导致新接收损坏 | false | 新 FF 先 FinishRx(E_NOT_OK)，重新建立 RX_FLOW_SEND；下层已复制旧 FC，旧确认只清 lower_pending，不会将新事务推进 RX_DATA。下一 main 才发送新 FC；其独立确认才开启 N_Cr。 |
| B3：CanTp Shutdown／Init 清 pending 后旧确认误完成新事务 | high | 两个生命周期函数均清 lower_pending，而标准回调只有 PDU ID；独立重启不取消实际 driver 已复制帧。patch：保留下层退役身份并按实际记录 ID 接纳确认，OFF 时只退役、不向 upper 回调；新事务待旧确认退役后接纳，真实 queued 链验证。 |
| B4：接收 WAIT 未用 WFTmax 限次 | false | 固定 SWS_CanTp_00315 要求每个收到的 FC(WT) 重启 N_Bs；WFTmax 限制接收方发送的 WAIT（00223／ECUC 00251），不是发送方收到的 WAIT。独立 fixture 已覆盖超过配置值的 WAIT 后 CTS 成功。 |
| B5：polling RX 跨 STOP／BusOff 后重启仍交付旧帧 | high | Can_SetControllerMode STOP 与实际 BusOff 清 TX 而未清 rx_pending；Can_MainFunction_Read 不按控制器生命周期拒绝旧帧。patch：仅新 polling 路径在真实停止／busoff 边界清待接收槽，验证旧帧丢弃及新帧成功；保留 legacy 即时路径。 |
| B6：HostCompleteTransmit token 与错误 handle 可伪确认 | false | host adapter 契约要求 caller 已证明物理 write／flush 与 ticket／handle／token；唯一生产 caller Ecu_Target consume 校验 slot state3、真实 ticket、PDU、output_pending 后才读取该槽 token 并调用。产品输入不能任意替换 handle；不存在所述未验证调用链。 |
| B7：OFFLINE 后保留 outstanding 并 STOP 负确认错误 | false | SWS_CANIF_00073／00489 禁止 OFFLINE／TX_OFFLINE 的 positive upper callback；00739 要求 STOPPED 时 every outstanding TxConfirmation 负确认。计数表示 upper 未报告事务，不是物理 slot；reviewer 按固定原文复核后撤回。 |
| B8：Dcm CopyRx／CopyTx 忽略 MetaDataPtr | false | 所选 CanTp 实际全部 chunk MetaDataPtr=NULL，whole-message StartOfReception 拒绝不支持的 metadata。固定 Dcm 00443／00996／00346／00350 定义 payload copy，不要求把 chunk unused metadata 作为另一寻址输入或自动拒绝；未证明所选调用链的错误结果。 |
| B9：COM callback 修改生命周期使循环状态失效 | false | 所选生成配置固定使用 Rte_COMCbk／RxTOut，仅访问 endpoint 状态和 Com_ReceiveSignal，不调用用户 runnable 或 group／DM／DeInit；任意外部 callback 并非生成配置。reviewer 沿实际 producer 复核后撤回。 |
| B10：Silent 下 P2 到期无响应是缺陷 | false | Dcm 01142 要求等 FullCom 且不得超过 P2ServerMax，00153／00156 禁止 Silent 发送；01143 只适用于未选择的 FORCE_RCRRP。实际 bounded release 符合选定同步请求，reviewer 按固定原文撤回。 |
| E1：合法 P2* 毫秒值超过 uint16 | high | source inspector 接纳至 655350ms，但 Dcm_ConfigType 使用 uint16 毫秒字段，100000ms 会严格编译失败／截断。patch：内部配置字段 uint32，正常生成工程和 0x50 wire 的 10ms 单位独立断言。 |
| V1：合法 Rx1／Tx0 仅 plan 测试 | medium | verification-gap 独立核对正常生成测试均仍 Rx0／Tx1，CAN_TX_HOH 和 CanIf 配置两个 producer 可独立漂移。patch：现有 sealed production-owner 测试增加真实 swapped ARXML，复用精确 RX、COM、DID 输出断言；当前代码尚未发生该回归。 |
| R1：全本地连接且无系统信号触发 unwrap panic | high | multi_com::inspect 对空 signals 返回 None，inspect_multi 后续两次 unwrap；合法引用的全本地连接和去掉 COM 路由可达此状态。首批已选择真实 Rx group 的网络 profile，patch：在构造 trusted plan 时返回源定位 COM_CONFIGURATION，两个正常入口拒绝且不写输出；不扩展另一个无网络 ECU profile。 |

B3、B5、E1、V1、R1 各有独立根因，均为所选配置内直接更正或既有测试扩展，不新增公开接口；route=patch。其余逐项按上述实际调用链和固定条款拒绝，无 defer 台账。

审阅 patch 已实施并验证：Dcm 私有配置的 p2_star_ms 扩为 uint32，标准公开函数签名、P2* 的 10ms wire 编码、第三方身份不变；固定 ECUC_Dcm_00768 的合法范围实际为0–100秒，正常完整 source validation 保持此范围。实际封存工程新增100秒（wire2710）及65.54秒（wire199a）SessionControl精确响应，两者均严格编译链接并经真实 HostBatch 输出；同时 Rx1／Tx0 通过全部原 CAN／DID／周期输出。TP 私有 pending_lower_id 保留实际下层已复制帧的身份，Shutdown 释放连接并关闭上层动作，OFF 或重启后的迟到确认仅退役旧帧；真实 queued 链验证新请求暂不发送、旧确认退役后新请求实际发送，只有新确认释放新 Dcm buffer。新 polling RX 在 STOP／BusOff／host 非STARTED边界清待接收槽，独立 driver consumer 核对旧帧不交付、新帧仍成功；缺省即时 legacy 路径不变。全本地有效连接的来源输入现返回源定位 COM_CONFIGURATION，两个正常入口不 panic。

本次 patch 完整 native54 multi（59.12s）＋runtime9（1.22s），default19 unit（0.36s）／71 builtin（25.81s）／2 c_analysis（14.09s）／50 multi（44.04s）全部通过，无 failed／ignored／filtered。Clippy 项目 correctness／suspicious 策略 all-targets＋native-tests通过（5.74s），core56／runtime66 quality和实际Csyntax通过；Python44／Ruff复用未改来源证据。已核对固定LF／Can.c CRLF及上述私有配置ABI变化后正常assets update，仅本次受影响源摘要及模块ABI摘要变化，无第三方身份变动，最终check待收尾。

历史失败保留：首次构建在旧Dcm.h摘要上明确拒绝，完成ABI与固定换行审阅并更新摘要后通过。最初额外655.35秒边界向量被正常ECUC source validation正确拒绝（VALUE_RANGE），按官方0–100秒范围修正为100秒和65.54秒，不放宽范围；首次全本地反例只删除一个mapping，得到未绑定Tx端点，改为删除完整DATA-MAPPINGS并提供真实本地连接后正常源定位拒绝成立。六实际样本重新生成 exact1 passed／1 filtered（13.26s）；35 ARXML与前次固定R24-11 XSD通过的实际字节一致，0 changed，复用该XSD和645引用闭包证据。受影响multi与single新C内容正在分别重跑实际c-check，未复用旧C诊断作为新源码通过证据。

审阅后实际C分析已完成：固定Cppcheck2.21／e73bf44、binary SHA434155fc2a092b4dd98042052fef93a501111bb18849e5b1db05e8854f8ba795及addon identity保持；multi52 unique TUs、host-batch与legacy-probe各51完整TU／exit1，2607原始诊断＝2573style／30warning／2error／1portability／1information，692adopted，error=None／passed=false。受影响single42 unique TUs、两个程序各41完整TU／exit1，1814诊断＝1780style／30warning／2error／1portability／1information，691adopted，error=None／passed=false。按ID、severity、message、完整source locations对比前次原始结果，两profile新增非style诊断均为0；两error仍是既有mmap平台模型和adopted IDLE bound。221条assessment仍not_assessed，不声明源码/MISRA全面通过，不关闭规则或追认偏离。最终assets check 0 changes、固定EOL82文件和diffcheck通过。

8.2 独立三层 review 及主代理逐项审查完成，五个独立patch已关闭并由上述正常检查验证；无延期发现、无新增台账。done仅指本story所选受控Linux生成／通信／调度及审查范围，8.3–8.5、Windows运行、真实MCU与完整标准／MISRA认证不由此完成。六样本、源码诊断和未评估范围保持既有准确边界。
