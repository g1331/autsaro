---
title: '经真实 OS 调度运行多组件通信'
type: 'feature'
created: '2026-10-09'
status: 'in-progress'
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

- [ ] 模型／fixture／定义：真实 Rx group、timeout、通知 handle、主函数 timebase 和完整调度共同核定，所有非法关系有拒绝测试。
- [ ] 模板／运行时／调用方：生成多组件 RTE、标准选定链路配置、回调及真实任务调用；最小可信应用准备入口接纳所有计划槽。
- [ ] 正常工程入口：用户应用真实执行矩阵及 AC-1–AC-8；变名、fanout、DM 和下层拒绝验证全部运行，不跳过。
- [ ] 交付资源／BMad：审阅 ABI／第三方／换行；实际 multi 与受影响 single c-check，记录完整性与诊断，必要回归通过。

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
