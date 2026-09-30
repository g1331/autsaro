---
title: '4.18 完整适用生命周期、Extended Status、Hook 与模拟中断'
type: 'feature'
created: '2026-09-29'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '81492c568c9d359fd1dc2e0fb406754e383eee7b'
story_key: '4-18-补齐生命周期-extended-status-hook-和中断义务'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

使既定单核 SC1／EXTENDED 的真实 FreeRTOS Win64 后端完成全部适用错误、Hook、启动关闭、逻辑 ISR 与 OSEK 四一致性类最低容量义务。标准服务的成功、错误及清理路径由真实对象和实际内核调度证明；完整等级仍须经4.19～4.22出口。

## Boundaries & Constraints

继承4.2封存507行本故事义务：160 applicable、346 conditional_na、1 conditional；逐条建立结果，不因参考ECU未用而删除能力。缺少同步、多核、OS-Application或IOC等前提时保留封存分类并说明具体条款前提；一般公共类型定义不能借SC1选择跳过。保持SC1／EXTENDED／单核／NONE、已采用FreeRTOS版本及真实原生栈，不重开选型、不扩展Epic2/5/6、不推送、不新增汽车线程或墙钟tick。

所有实现使用C99及MISRA C:2012修订基线。Hook在真实转换边界运行，不用私有trace替代；不得在汽车owner或Hook中阻塞输出。Startup/Shutdown原有关闭控制栈、不可逆关闭与栈故障保护保留。标准调用层级按OSEK表和R24-11逐服务处理，拒绝必须无副作用，输出参数在拒绝时保持。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 四一致性类合法容量配置，实际启动、激活、等待、资源和Alarm动作 | BCC1/BCC2分别至少8非挂起Task、8优先级；ECC1/ECC2至少16与16；资源分别1/8/8/8、各2内部资源、1Alarm、1模式；ECC各Task至少8事件能力。实际运行及独立观察证明，不仅检查常量 |
| BCC1/ECC1唯一优先级及单次激活；BCC2/ECC2重复优先级及多次Basic激活 | 独立轨迹符合类规则；适用抢占/非抢占、FIFO、事件和资源能力复用已有效证据并补缺；RES_SCHEDULER显式配置，不凭空创建 |
| Task入口返回，仍持有资源／禁中断／排队激活 | 在离开RUNNING前ErrorHook得到E_OS_MISSINGEND；资源及未配对屏蔽恢复，PostTaskHook和正常终止发生，下一激活或Task真实运行；不能直接整OS关闭 |
| 服务错误、ErrorHook内再次失败、参数宏配置开关 | 标准service ID／准确参数快照与状态可读取，外层快照不被覆盖且不递归；开关配置可追踪；不同层级符合标准许可表 |
| 实际抢占、WaitEvent、Terminate/Chain及显式ShutdownOS | PreTask/PostTask按真实离开／进入RUNNING顺序且Hook查询身份/状态正确；ShutdownOS不调用PostTask；Startup模式与GetActiveApplicationMode一致，成功StartOS不返回 |
| Disable/Enable、嵌套SuspendAll/ResumeAll和SuspendOS/ResumeOS交错 | 保存/恢复原状态，All屏蔽全部可屏蔽应用ISR、OS仅Cat2；无配对恢复无效果；非中断OS服务被忽略并按适用签名返回E_OS_DISABLEDINT |
| Cat2返回时泄漏屏蔽或资源、模拟嵌套ISR | 自动清理并报告E_OS_DISABLEDINT/E_OS_RESOURCE；GetISRID恢复外层；源屏蔽保留pending，ClearPending和Enable(clear)操作真实pending；非法ID/层级无副作用 |
| GetTaskID、GetISRID、isOsStarted、ControlIdle及标准类型 | 无汽车Task/ISR返回规定INVALID值；启动前isOsStarted=false、调用StartOS后true；单核ControlIdle按规范省略CoreID检查，实际支持的host idle动作及无效模式有证据；R24-11公共类型、常量、范围及签名独立编译核对 |

</frozen-after-approval>

## Code Map & Tasks

- runtime/os/include/Os.h、Os_Target.h、Os_Hooks.h及Os_Error.c／内部错误声明：补标准服务、类型、错误码与参数宏；显式可选Hook和两个错误宏配置开关，默认未配置时保留既有消费者行为。已有初始化器和生成配置同步更新，不擅自实现SC2/3/4行为。
- runtime/os/src/Os.c、Os_Time.c、Os_Schedule.c及内部声明：统一服务级错误身份/参数报告和调用层级/禁中断保护；ErrorHook递归抑制及准确输出保持。软件Counter/Alarm配置与私有受控tick owner要求分开验证，纯Basic容量配置不依赖Extended owner。
- runtime/os/src/Os_Backend.c、FreeRTOSConfig.h：接入真实内核SWITCHED_OUT/IN及当前汽车身份；missing-end在原生Task trampoline中清理资源/屏蔽后正常结束；Shutdown路径不补PostTaskHook；保持真实栈关闭机制和StartOS不可返回。
- runtime/os/patches/：仅在实际移植边界需要时增加受控补丁，保留来源/顺序/摘要。模拟pending位派发前统一判断全局、OS类、源及资源门槛；实际源清除与嵌套上下文恢复，不以后台线程模拟新调度。
- runtime/os/tests/sc1_errors_hooks.c、scripts/epic4_os.py：独立字面Hook/错误/参数/ISR/清理/容量轨迹，新进程隔离配置；保留全部原生既有向量并更新被新正确行为改变的missing-end独立预期，记录依据。
- core/tests/end_to_end.rs、生成模板及offline引用：加入epic4_sc1_errors_hooks_and_isr正式入口，生成交付闭包包含新实现/参数配置，实际参考ECU/HostBatch编译链接和行为回归。
- docs/assurance/evidence/epic4/story-4-18.json、spec/sprint和既有说明：507行逐条结果与具体配置/实现/测试可双向追踪；拒绝或未跑如实记录，完整SC1、ARTI、221项和交接未通过不关闭。

## Tasks & Acceptance

- [ ] 核对实际R24-11/OSEK原文、公共类型及完整调用表，冻结独立错误/Hook/中断/四类容量预期。
- [ ] 补标准服务和错误/参数快照、Hook上下文及真实任务转换；实现missing-end正常终止与清理。
- [x] 补全中断屏蔽配对/嵌套、源pending/ID及Cat2退出清理；实际idle动作与启动状态验证。
- [ ] 四类成功容量与关键拒绝向量、所有适用服务和类型独立验证；507行逐条处置并保留封存适用性。
- [ ] 既有OS/栈/时间/参考ECU/HostBatch回归、最终源码/二进制身份、静态/MISRA核查、完整增量门与三路独立复核，本地提交及sprint同步。

## Implementation Notes

下一公共类型增量选择：原文p151将DONOTCARE／TotalNumberOfCores限定在多核环境；仍提供明确的公共常量并将所选核数固定为1，不能据此宣称多核运行能力。p229的RTE类型通过新Rte_Os_Type.h提供，TimeInMicrosecondsType为uint64；同一消费者可同时包含Os.h和RTE头，因此CounterType须有唯一uint32定义，统一现有native配置、API和有类型错误参数，禁止通过不同include顺序产生两个不兼容CounterType。已配置Counter容量保持原有8个；新增256、65536和UINT32_MAX标识的真实拒绝／输出保持／ErrorHook参数验证，证明高位不会截断成合法Counter0。公共消费者按实际规范约束核对类型和常量，生成离线闭包包含新头，不以RTE头交付代替Counter服务接口和生成时间换算义务。

后续507行逐项核查已发现“SC1 public interfaces”95行不全是C类型：封存section标签在8.8.2.2后未继续更新，实际PDF p238/239属于配置校验，p321为ARTI配置，p391～393为OS生成义务。保留封存行及适用性，最终处置须按原文实际章节核对，不能按标签批量通过。p223的ReceiverPullCB明确可选且依赖IOC接收配置；p229另规定Rte_Os_Type.h中的TimeInMicrosecondsType为uint64、CounterType为uint32。p393还包含可重定位中断向量段、内部timer信息、OS_TICKS2单位换算宏和Memory Mapping封装；必须追到实际生成工件及适用配置，并与4.19/4.20出口对齐，不能用现有公共头编译或原生行为测试代替这些义务。

真实嵌套ISR增量起始94efe9345c98b58ba3be37338fa4b4550884bba2。配置附加可选Os_IsrConfig，32个静态源优先级，应用源2～31使用1～31（高值先行），0为未配置；私有yield0/tick1保留，显式配置时tick为最低1。NULL保留全部应用源同级31的平坦配置。实际原生S栈/既有mutex内在pend与critical出口递送更高优先级源，同级和低级保持pending，不创建ISR线程或另一Task选择器。每层保留身份、Hook阶段和有类型错误参数；资源归属为独立Task/ISR槽，子层清理不得释放父层资源。共享ISR资源的ceiling属于ISR优先级域，持有它的Task提升到所有Task之上；Task-only资源仍用Task优先级。配置校验保留内核源、Cat1/资源ACL和输入源的完整前提。

独立验收须证明真实三层抢占/返回、同级与反向编号的优先级、子层资源/屏蔽清理、父ErrorHook快照恢复、All/OS屏蔽下Cat1/Cat2行为及最外层Task调度。OSEK2.2.3 PDF p73 §14.2.3.1明确Cat2可以抢占Cat1，但子层激活/事件不在最外层Cat1退出时触发重调度；保留请求至后续合法调度点。原生端口回调重入的R17.2范围和物理栈深度须明确记录，不能以有界深度冒充已批准偏离。生成模板/离线补丁闭包、所有初始化器、正式测试与来源摘要同步；当前实现中，尚无本增量通过声明。

调用上下文/idle独立增量起始a9dbeeff93cdcb838480fa9afb2266baa65551fd。已读取并渲染R24-11 p72/73表，优先该版本扩展而非旧OSEK更窄许可；所有六屏蔽服务在各已支持Hook/Alarm阶段许可，Hook有独立逻辑屏蔽归属，保护原调用者状态同时对Hook自己屏蔽的服务执行00093规则。GetISRID仅ErrorHook许可；Mode在Error/Pre/Post/Startup/Shutdown许可；Shutdown仅Task/Cat2/Error/Startup，Start不从活动OS/Hook再次启动。ShutdownHook以无普通mutex的专用阶段设置调用，消除实际关闭栈错误服务取得受损actor锁的死锁。无当前ISR及私有yield的S actor普通服务拒绝。ControlIdle按单核省略CoreID检查，支持真实虚拟核既有NO_HALT，未知模式拒绝；isOsStarted按DRAFT语义记录StartOS入口而非Ready/成功，不改标为定稿。新增九调用上下文、五真实idle过程及两正式入口；原屏蔽向量扩展至31非中断服务。Alarm场景通过配置的软件Counter/Alarm回调真实递送；无ISR身份场景在真实原生ISR栈故障注入，不能证明嵌套支持。固定端口IRQ0为私有yield、IRQ1为受控tick；第十补丁保护两内核源的handler/源控制所有权，真实tick遵守All/OS屏蔽。两新增原生向量确认实际过滤器观察到pending tick时kernel仍0、票据未完成，恢复后同票据完成且kernel/Counter各增一次，时间回归共44向量，含两种消费Task自身屏蔽及ISR阶段／mutex交接的确定性验证；第十一补丁在释放mutex前结束ISR阶段。

下一Cat2资源出口增量（起始9c6c7ece29b54e34e3598b415a77749b269c8119）：按本地R24-11 SWS_Os_00369，实际native dispatcher仍持mutex期间，复用资源释放实现LIFO清理真实配置资源，恢复所有权/ceiling后调用可选ErrorHook报告6，原currentISR在回调后才退出；遗留屏蔽仍按00368先恢复并报告9。独立预期覆盖单/双资源、Disable/All/OS/mixed、未配置Hook和平衡路径，实际pendingISR6和已激活Task1必须先后取得全部释放资源，再返回原Task0取得。完整嵌套ISR及SC1整体不据此关闭。

中断独立增量（起始cd647a934790bce89c56ed6a22827a1517c6515c）：六配对服务采用实际原生actor的PE TLS状态，All与OS独立嵌套，Disable非嵌套，首/末配对更新共享原子owner数；实际端口分发遵守屏蔽，内核私有yield保持可用。25个StatusType及Start/Shutdown/Mode/GetISRID边界忽略被屏蔽调用者的非中断服务，保留输出；ErrorHook合法查询可观察原错误。Task missing-end和Cat2返回自动恢复遗留屏蔽；Cat2清理在原ISR身份下报告9/service252。GetISRID对实际Cat2返回身份、其他返回INVALID；尚未证明嵌套恢复。第九补丁在真实pending/handler表增加源开关及清除，三个标准源API在原有interrupt mutex内操作，Disable保留pending，Enable(clear)/ClearPending按真实位清除，非法/私有yield/Cat1/未安装源拒绝，Task/Cat2之外拒绝。公共boolean来自共同Std_Types.h，Windows RPC重名由平台边界隔离。源API入口同样检查真实原生栈和不可逆关闭门。

当前中断证据：28个独立原生过程通过，包括四类配对交错/未配对、三种屏蔽下29非中断服务无副作用/准确错误参数、Cat1/Cat2身份与配对、四种Cat2泄漏恢复、四种missing-end恢复后真实pending递送和monitor运行、启动前拒绝，以及八个实际源控制场景（含真实Cat2内三个源API的清除/保留待决递送，以及在实际ISR栈上故意清除当前逻辑身份后的无副作用拒绝）。正式入口epic4_standard_interrupt_pairing接入核心测试。公共类型三包含顺序和ErrorHook八配置回归通过；部分静态扫描覆盖九本地OS模块、生成Hook及九补丁后的tasks.c/port.c共12单元，exit1诊断完整保留。R17.2有界wrapper/change重入仍未批准，完整MISRA/SC1不声明通过。完整ISR资源清理、实际嵌套ISR、全部调用表、ControlIdle/isOsStarted、507行最终结果及4.19～4.22仍待推进。

返回Task/资源增量实施：trampoline在RUNNING状态通过标准错误通路报告E_OS_MISSINGEND11、内部合成service253，然后使用同一真实完成事务按LIFO清理外部资源/ceiling，正常发Post并消费激活队列；新激活重入真实原生栈，其他Task继续。移除未配置RES_SCHEDULER自动隐藏槽；既有成功资源向量改为真正显式配置最高Task ceiling，并新增未配置Get/Release双拒绝及状态不变。旧missing-end最低关闭预期按R24-11 p74～75更新为正常终止后monitor继续，全部22Finish/Chain回归通过。七个返回Task向量、26资源、22Finish回归、最终正式门和三路复核均通过，标准中断配对恢复仍开放。

人工调用图核查补充：虽然错误报告函数和ErrorHook自身不递归，外层失败查询GetTaskState等标准wrapper可能仍活跃，ErrorHook依法再次查询同一服务时会重入该wrapper（error_hooks向量4已有真实路径）。这是不同于嵌套ErrorHook的有界函数重入；完整MISRA出口须对R17.2 Required作明确消除或受控偏离处置，目前没有批准，standard-error-static-analysis已记录，不能把部分编译/扫描或回调递归抑制冒充该规则全通过。

后续missing-end清理已查明一处必须同时修正的资源契约：Os_BackendResource仍保留未配置RES_SCHEDULER时自动建立第八槽的旧分支，而R24-11 OS p37明确不自动创建。本故事冻结约束要求显式配置。下一资源/终止增量须移除自动分支，既有scheduler成功向量改为真正配置该资源，并补未配置拒绝；资源清理只能索引实际配置资源，不依赖该隐藏槽。当前容量增量已显式配置，不能用它证明这个未配置分支已关闭。

Task Hook增量：第八受控补丁只观察原FIFO候选，在pxCurrentTCB赋值前发Post；真实SWITCHED_IN取得内部资源后发Pre。用单一运行Hook对象标记过滤无切换yield，同时Wait/Finish/Chain显式在状态变化前发Post并清除标记，防止内核出口重复。相同TCB的新激活重新发Pre，Shutdown关闭门后不发Post。生成离线闭包、八补丁来源摘要和工程说明同步；八独立过程字面轨迹覆盖抢占/等待/唤醒/Chain/Term、内部资源、未配置Hook、自身Chain、排队同TCB、Shutdown、noop-yield和已满足Wait。最终八向量、正式测试、三路独立复核和完整门均通过；不据此关闭4.18。

ErrorHook独立增量已接入25个标准StatusType服务边界、有类型参数快照、两个访问宏开关及可选回调配置；真实Task/Cat2/Startup三类执行上下文覆盖。Startup无汽车Task的嵌套错误报告返回INVALID_TASK；ErrorHook内再次失败绕过活跃报告函数，不递归、不覆盖外层快照，回调结束恢复之前Hook限制。Alarm动作错误在标准回调已配置时仅报告一次。三头文件分离避免循环include，生成交付闭包包含新头文件、Os_Error.c和Ecu_OsHooks.c。Pre/Post函数已绑定但真实转换调用仍待实现，绝不按本增量关闭4.18。八个独立新进程覆盖四种开关与配置/未配置，已配置每次31条独立错误记录；部分Cppcheck诊断保留，完整221项仍开放。

下一实施增量的接口选择：ErrorHook参数快照使用逐服务的有类型结构/联合成员，保留TickType与各类输出指针原类型，不通过uintptr_t或void指针强转取回参数。一个错误报告边界负责回调前后上下文与递归抑制；ErrorHook内再次失败只返回状态，不覆盖外层快照。标准ErrorHook与现有Time.error_hook私有兼容通路须明确单次报告责任，原40向量作为回归依据。新可选Hook配置和两个宏开关按真实生产配置接入，旧未配置消费者不引入未定义回调符号。

下一项已定位实际边界：configOS_REQUEST_FIFO的taskOS_SELECT_READY直接选择ready-list头并赋予pxCurrentTCB，Hook必须在候选确定、赋值前后接入且过滤相同Task；WaitEvent/Terminate/Chain要在waiting/activation状态改写前发Post，避免报告已WAITING/SUSPENDED的任务。当前参考ECUC明确OsErrorHook/Pre/Post及两个访问开关为true，生成配置尚无完整实现；后续须同时补runtime与生成闭包，不能只补测试回调。

容量增量已实现：标准软件Counter/Alarm可独立于私有Extended-owner确认通道使用，wake_event0/ownerINVALID显式选择；该模式没有宿主硬件tick来源，硬件Counter配置拒绝，参考非零wake_event路径保持。GetTaskID读实际kernel选中Task，GetActiveApplicationMode读实际StartOS模式；完整Hook/错误/调用表仍待后续实现。四类最低容量、四类含非抢占Task、两类重复优先级/三次Basic排队激活、模式2及七类配置越界拒绝共18向量通过，实际资源/内部优先级/Alarm二次Task入口、ECC每Task八事件置位清除状态均有独立预期。

首个类型增量：已补公共类型及范围/指针/常量，独立C99消费者覆盖header-only、Windows-before、Windows-after三种真实包含顺序及16项AccessType真值表。原生生命周期46向量通过；初次原生编译暴露Windows COM ApplicationType重名，现由既有Os_Windows.h边界隔离。类型/源码证据在public-type-contracts-4-18.json；本故事其他服务/Hook/ISR/容量仍未完成，不因类型编译通过升级等级。

需求/授权无待确认缺口；无外部不可逆操作。新增公共服务与Hook配置影响OS声明、生成配置和全部本地静态初始化器，按所有消费者同步更新。纯标准公共类型先独立编译核对，条件类型不代表对应SC2/多核行为已实现。

2026-09-29三路只读探查已返回。Task trampoline目前返回即E_OS_STATE关闭；只有Startup/Shutdown Hook，缺标准错误参数宏和Pre/Post；模拟ISR仅单current_interrupt和资源门槛，缺配对/源控制。现有数组上限足够表达四类容量，但没有完整成功容量向量。Os_TimeValidate强制Extended owner，须为纯软件Counter/Alarm实际使用消除不适用的私有tick条件。已读取本地R24-11 OS p74–76、131、194、201–202、205及OSEK最低容量/调用表；ControlIdle单核明确不查CoreID，idle模式硬件相关，isOsStarted在R24-11标为DRAFT，不能冒充已定稿条款。

## Spec Change Log

## Review Triage Log

- 嵌套ISR增量三路独立复核：blind与edge无发现；verification-gap的旧isr_cleanup ErrorHook内要求Cat2已可递送为medium/patch。OSEK §11.1明确Hook不能被Cat2打断，新Hook gate导致旧断言与实际契约冲突。改为Hook内仍屏蔽Cat2，保留随后真实ISR6 probe、helper和原Task获取/释放全部资源的断言；不删除资源重获和LIFO清理验证。完整门首轮已观察到该旧测试失败，日志保留；修正后须定向复验并完成最终门。

- 第十一补丁／44时间向量／repeat-start修正的追加blind、edge、verification-gap三路复核均无新增发现。最终行为门92集成测试全部通过（567.52秒），Python29、UI lint/build、桌面build与两Clippy通过。随后按既有LF属性归一OS文本字节，原始补丁和上游不变；七套原生回归、三调用错误变体、两旧ISR时序变体及12单元部分扫描均从规范化来源刷新，500摘要匹配，Python／增量质量复验通过；生成ECU包亦在该来源重编复验。507总结果已建立，61行关联增量证据，全部最终处置仍待逐项评估，4.18保持in-progress。

- 调用上下文/idle增量blind提出“StartOS失败或被忽略后isOsStarted不应为TRUE”：false。已核对R24-11 p205的DRAFT91034，以是否调用过StartOS为返回依据，非Ready或成功；当前入口原子记录调用，屏蔽查询另遵守00093，无两者混淆。edge无发现。verification-gap提出“屏蔽时isOsStarted也应TRUE”：false，p74 SWS_Os_00093要求Task/ISR/Hook自身屏蔽时忽略任何非中断OS服务，DRAFT查询未声明例外；当前返回无效FALSE且ErrorHook报告9，恢复后TRUE，原状态未被改写。两条建议均不修改实现或弱化断言。

- 专项tick探查指出native生产者读取kernel可能有普通Task服务上下文问题：固定portmacro.h将32位tick标为原子，xTaskGetTickCount路径不进入普通critical；去掉该读取与递送前printf的复制实验仍有失败。失败后采集定位到票据已完成而Wait返回仍WAITING的原断言。缓存yield条件的复制端口不能修复；实际dispatcher在释放mutex后才到下一轮清除共享xInsideInterrupt，Task可能先取得mutex并错误跳过critical出口等待。第十一补丁在mutex内完成ISR阶段后才释放；六次对应复制实验全部通过，新增两种消费Task自身屏蔽的确定性握手与原有40/两monitor屏蔽合计44向量通过。复制恢复旧标志时序时两种握手均被原断言拒绝（isr-phase-release-mutations-4-18.json）；复制native观察者失败后等待健康controller报告，避免生产者先退出99遮蔽Task断言，产品成功断言未弱化。原失败日志保留，完整嵌套及SC1出口仍开放。

- 当前完整门91通过/1失败为旧repeat-start轨迹ISID；源码确认Startup内重复StartOS应按R24 Table7.1/00088忽略，而非重入原生初始化并增加第二个I。生命周期场景在重复调用返回后核对实际Mode1/DRAFT已调用状态并加R，独立预期ISRD；保留显式Startup随后Shutdown7及资源失败拒绝，46原生向量通过。历史生命周期工件不覆写。

- Cat2资源出口增量blind/edge/verification-gap三路独立复核均无发现。十个真实过程、28屏蔽/26资源回归通过；actual旧backend副本在single/maximum/mixed/unconfigured-mixed四个新独立断言场景均因原断言关闭被拒绝，证明本次测试识别原问题；12翻译单元部分静态诊断保留，不升级完整MISRA声明。

- 中断增量blind/edge/verification-gap三路完成；Cat1屏蔽服务建议按OSEK原文驳回，源上下文及Cat2覆盖缺口已修正。追加两条独立pending/拒绝副作用覆盖缺口已修正，最终定向复核均无剩余发现。五个真实编译变异（Cat2两种清除no-op及三个拒绝操作仍产生副作用）全部被新原生断言拒绝，证据interrupt-oracle-mutations-4-18.json；实际28向量、公共类型三顺序、八ErrorHook配置与最终交付摘要匹配。完整故事仍开放。

- 返回Task/资源增量blind／edge／verification-gap三路独立复核均无发现。七个返回Task、22Finish/Chain和26资源真实向量均通过；Python首轮因仍读取历史finish-chain/resource-preemption记录不符合新标准行为而报红，已改用本次真实回归工件，同时增加旧整OS关闭和隐式scheduler的拒绝变异验证，历史记录不覆写。完整故事中断恢复和SC1仍开放。

- Task Hook增量blind／edge／verification-gap三路独立复核均无发现。首轮完整门86通过/1失败，实际Pre/Post增加诊断标记后，20tick总139标记超出127前缀，trace_dropped12；旧生成测试按未截断片段计数失配。改为独立字面139序列的精确保留前缀和丢弃数量，保持completed20、两个实际CAN输出、全部初始化失败断言；额外只读verification-gap复核无发现。失败原始日志与Windows链接占用日志保留，源实现未因此改动。

- ErrorHook增量edge与verification-gap独立复核均无发现。blind提出“GetTaskID标准service ID应0x04”：false，实际R24-11 OS p167的0x04属于CheckTaskMemoryAccess，p147将GetTaskID列为OSEK服务，OSEK2.2.3 §13.8.3只规定各OSServiceId_xx唯一，没有该数值；当前0x80～0x90与AUTOSAR规定的Counter/Schedule ID分开且独立字面预期检查。保留现有值，不将其他模块/实现常量当作本规范义务。

- 容量独立增量三路blind／edge／verification-gap均未发现可定位缺陷或验证缺口；完整故事的Hook/错误/ISR工作仍开放。正式容量/真实生成两项测试163.73s通过；18容量、40受控时间、43计时向量封存，26最低容量义务直接映射到四个主场景，部分静态exit1原样保留。容量增量完整门exit0：85个集成测试（660.24s）、29Python、UI/桌面构建和两组Clippy通过；本故事Hook/错误/ISR仍未完成，sprint保持in-progress。

## Verification

嵌套ISR增量已通过26个原生向量和修正后10个Cat2清理回归。七个实际编译变异均被既有断言识别：串行分发、丢父身份、共享ISR资源栈、丢Hook快照、允许Cat2打断Hook、Cat1出口提前调度由native成功契约断言拒绝；源编号代替优先级的副本正常退出但产生AGCPB，正式Python独立轨迹预期ACPGB明确拒绝，不把进程exit0当行为通过。12个实际翻译单元部分静态exit1诊断保留；固定深度的回调重入仍是未批准R17.2。四个当前原生／负向／静态工件的产品源码摘要逐项匹配。最终完整门exit0：93个核心集成测试全部通过（1108.27秒）、29Python、增量质量／C99、UI lint/build、桌面build和两组Clippy通过，日志为.scratch/epic4/story418-nested-final-gate.log，退出记录为同名.exit。首轮旧测试失败及缺最终汇总日志保留，没有改写为通过。三路独立复核的唯一发现已修正；追加定向复核无剩余问题。全部507封存行保留，62行关联增量证据，所有最终处置仍待逐项核查；4.18与Epic4保持in-progress。

Cat2资源出口增量最终完整门exit0：90个核心集成测试567.09s、29Python、增量格式/C99、UI lint/build、桌面构建及两组Clippy通过。十个出口过程、28屏蔽/26资源回归与最终源码摘要匹配，实际旧backend四场景均被拒绝；12单元部分静态exit1保留。三路独立复核无发现，sprint保持4.18/Epic4 in-progress。后续完整调用表已以官方R24-11 p72/73实际PDF图像核实：六屏蔽API全部Hook/Alarm列为OK，当前较窄Hook限制须在下一增量修正；GetISRID/Shutdown/Mode各自不同许可也须覆盖。当前出口清理不冒充全调用表、实际嵌套ISR、221项或交接完成。

中断增量最终完整门exit0：89个核心集成测试566.71s、29Python、增量格式/C99、UI lint/build、桌面构建和两组Clippy通过。原生28、公共类型三包含顺序、八ErrorHook配置、五个被拒绝的真实变异和最终源码身份一致；12实际翻译单元部分静态exit1保留。首轮77通过/12失败均因新增boolean后的共同头清单旧摘要，审阅源码后同步BSW契约清单并重编译；次轮88通过/1失败仅因运行期间新增场景而旧测试程序仍预期26，最终重新编译28后完整门通过，失败日志均保留。三路复核及修正复核完成，没有剩余增量发现。4.18与Epic4保持in-progress，ISR资源清理/实际嵌套/全调用表/idle/全部507行、221项与交接未关闭。

返回Task/资源增量最终完整门exit0：88个核心集成测试532.30s、29Python、增量格式/C99、UI/桌面构建和两组Clippy通过。七个返回Task/22Finish/26资源原生证据与最终源码身份匹配；实际八补丁后tasks.c及九本地C翻译单元部分静态exit1诊断保留。三路独立复核无发现。正常Task入口返回已不再关闭整个OS；SWS_Os_00239标准中断配对恢复、完整ISR/调用表、R17.2处置及完整221项/交接仍开放，sprint不关闭。

Task Hook增量最终完整门exit0：87个核心集成测试541.62s、29Python、增量格式/C99、UI/桌面构建和两组Clippy通过。真实生成定向复验222.76s通过，最终完整门也覆盖该项。八原生Task Hook向量与源码身份/八补丁摘要一致；九本地C翻译单元及实际八补丁后tasks.c部分静态exit1诊断保留。三路独立复核和生成轨迹断言定向复核均无发现。sprint保持4.18/Epic4 in-progress，missing-end/ISR/全调用表、R17.2处置、完整221项和最终交接仍开放。

ErrorHook增量完整门exit0：86个核心集成测试600.13s（包含正式epic4_standard_error_hook_parameters、真实参考ECU/HostBatch/栈及所有既有回归）、29Python、UI/桌面构建和两组Clippy通过。最终仅内部声明参数名校正；八配置原生编译/运行、增量quality及桌面构建已针对复验，随后文档身份刷新。部分Cppcheck九个翻译单元exit1，诊断和人工范围核查记录于standard-error-static-analysis-4-18.json。四条相关义务均按部分支持记录，不把未来服务/missing-end/ISR清理或整体SC1标为通过。

正式epic4_sc1_errors_hooks_and_isr、四类最低容量成功运行、错误/Hook/参数/层级/ISR独立向量；受影响既有OS/时间/实际栈/参考ECU/HostBatch回归。完整门python scripts/verify.py --scope all --base <4.17完整本地提交>，测试线程按Owner Guide固定2。实际C99编译、部分Cppcheck与人工重点核查不替代完整221项原文/Required批准或SC1最终出口。全部验收无头后台，原生UI/IPC只能在隔离桌面验证。

中断增量复核逐项：blind的Cat1不得调用六屏蔽服务为false，OSEK2.2.3 §13.3.2各服务明确允许Cat1/Cat2/Task，现有isr-balanced实际覆盖；不按通用非中断服务上下文删掉合法Cat1能力。edge的注册S栈无当前逻辑ISR仍可操作源为medium/patch：内部通用ServiceContext确实接纳current_interrupt>=32，源边界已显式拒绝无当前ISR或私有yield，新增source-outside-isr实际ISR栈边界故障注入验证三API均CAL/源位不变。verification-gap的Cat2源调用未覆盖为medium/patch：source-isr在真实Cat2内调用全部三API，真实pending清除两次、最后保留一条，另一源实际恰好递送一次；没有新线程、mock pending或嵌套ISR声明。

验证覆盖复核追加两条medium/patch均已修正：同一pending位的连续清除存在后续动作掩盖错误的缺口，现ClearPending用源6、EnableTRUE用源7、EnableFALSE用源8，只有8可实际递送；任一清除无效果都会触发不允许的源6/7回调。拒绝副作用场景现Disable8后立即检查enabled，Enable6TRUE与Clear7分别面对预置真实pending，返回后必须实际递送6/7各一次（ODEC），不会由后续反向操作恢复或无pending掩盖。独立28向量通过，未将故障注入当作正常嵌套ISR支持。
