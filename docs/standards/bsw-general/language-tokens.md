# 语法与原始诊断

## 原始 token 分类的具体误报

当前 master `3e93e436` 的 Linux standard-ecu 工件保留 20 条 R7.3：
Os_Backend 的 `OS_RESOURCE_CREATE(call)` 四处，Os_HostServices 的 `cell` 六处，
Os_HostLinux 的 `cell` 四处及 `all` 两处，Os_LinuxNative 的 `cell` 四处。
逐列回到原源码均为标识符；固定 2.21.0 的 `misra_7_3` 使用十六进制字符集合加小写 l
后缀的正则，未先确认 token 是数字，故这些名字恰好匹配。
R7.3（Required，主文印刷61／PDF69）约束数字字面量后缀，不约束这些标识符。
不能为了消除误报更名真实参数、改 addon 或删除原始记录。

同一工件的 12 条 R12.3 位于 Os_Stack.h 的七个成员声明分隔符、Os.c 的 i/j
声明、Os_Backend 的四个计数器声明和 registered/gate 成员声明。
它们在 C99 语法中是声明的逗号分隔符，不是 §6.5.17 的 comma operator；
R12.3（Advisory，主文印刷106／PDF114）禁止 comma operator；声明分隔符不属于该运算符。
类型头拆分只改变七个成员分隔符的位置，成员顺序与类型不变；其余声明不改写。

## DET 参数原型的具体误报

冻结 `ca6627b4` 的 standard-ecu 工件中，`bsw/src/Det.c` 的
`Det_ValidHooks(const Det_ErrorHookType *hooks, uint16 count)` 位于第 16 行，
两个形参均具备类型和名字；`Det.h` 中的 `Det_ErrorHookType` 也以四个具名参数定义。
R8.2（Required，主文印刷64–65／PDF72–73）要求函数原型及具名参数；
TC2.10 只追加 R8.3 交叉引用。这里既非 K&R 定义，也没有无名或空参数列表，
因此该位置的 R8.2 原始报告为误报。此判断绑定这两个实际声明，
不扩大到 SDK 回调或其他函数；保留原始报告，不修改分析器。

## 回调的 const 建议

通用 constParameterCallback 建议须按调用 ABI 复核。watch、control_actor、native_task_start、task_entry、actor_start
以及两个 Linux signal handler 和 guard_signal 的参数类型分别由原生线程或 sigaction
回调原型约束；不能把参数类型改为 const void * 等不兼容函数类型。
Os_LinuxAltStack 的 previous 和 native_call 输入由汇编约束实际读写，固定分析器未建模，
其 constParameterPointer／R2.7 建议不作为删除或收窄这些参数的依据。
这些是限定位置的工具判断，不覆盖别的 const 建议或 SDK 实现。

## 词法与接口表达的人工评估

以下使用[共同配置、原件及冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)。
Advisory仍尽合理可行遵循；主文§6.2.3（印刷13／PDF21）允许用替代安排记录不符合，而非必须走正式偏离。
本轮的有界工程选择只保留下列明确机制及真实违反，不改变类别或R15.5以外的GRP，不借 #381 的124键扩大授权。

D4.9的宏与底层函数分开判断。固定SpecificationsARXML的SWS_Rte_07404/07405/07406
分别给出Rte_IsInfrastructureError、Rte_HasOverlayedError、Rte_ApplicationError的函数式宏，
SWS_Os_00393要求生成OS_TICKS2NS/US/MS/SEC_<Counter>(ticks)。当前SystemCounter四宏转发
类型化inline转换；三RTE宏的实参各出现一次，保留128/64/63掩码及表达式结果。
这些公开宏不因正常程序未使用或底层有函数而成为D4.9满足。OSEK OS2.2.3 §11.2及§13.8.4
明确OSErrorGetServiceId及所选OSError_<service>_<parameter>访问宏；原件摘要
2ec77d3c4259797f4d0190f938131b641d415d8e0a85ce40a72e4201a9114746已核，
实际Os_Hooks.h按当前SC1接口集合发布，闭包见[OS兼容契约](../os/kernel.md#审阅确认的条目)。
本条独立保留这些宏接口的真实形式；风险为宏类型/求值差异，控制为单次实参使用、类型化底层函数、
[独立RTE状态向量](../rte/contracts.md#输入状态和执行)及正常OS转换/ErrorHook消费者。
标准改名、位值、计时类型、接口集合、上下文或目标变化时重评，不由旧R2.5/R8.7记录授权新D4.9位置。

其余自有宏按实际语言角色区分：TASK/ISR/ALARMCALLBACK/Declare*生成声明或拼接名字，ARTI_TRACE保存
调用点字面身份，STACK_POINTER取得调用点RSP，不能以普通函数保持同一展开合同。
FreeRTOSConfig.h的两个trace宏沿内核配置宏入口绑定Backend；OS_RESOURCE_CREATE的失败注入分支
必须能不求值call，不能改为先求值的函数实参；Linux sanitizer START/FINISH还在调用点借用fake栈槽。
这些精确冻结定义的保留理由是现有配置/调用点语义，不把“宏通常更快”作为全部函数式宏的通用例外。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| D4.4 | Advisory | 指令／C99 | satisfied | 不以注释停用实现（主文27；AMD2扩展语言）。[实际词法边界](language.md#注释与字面量的实际词法边界)中的Doxygen示例是说明而非停用实现；commented_defines输出生效define，未选择功能以条件编译表达。所选307种交付文件字节与正常producer没有另发现注释停用实现。 |
| D4.5 | Advisory | 指令／C99 | retained | 同namespace且重叠可见的名字应避免字形歧义（主文27–28）。[既有重名整改](../os/declarations.md#条件接口与声明owner)保留标准COM_INIT/Com_Init、COMM_INIT/ComM_Init及本机HANDLE/handle、CONTEXT/context、SIZE_T/size_t；标准发布名/原生类型与项目局部角色有类型和模块前缀区分。仅这些实际名字选择保留，理由是避免改变标准/原生接缝和既有调用合同；风险为读错名字，控制为类型检查、声明契约和真实双平台消费者。新增同形私有名字须整改，不将不同成员namespace或已结束原型作用域并入。 |
| D4.6 | Advisory | 指令／C99 | retained | 数值类型使用宽度/符号typedef（主文28–29；TC1排除plain char，AMD2.23–25）。汽车值沿Platform_Types/stdint；仅[原生表示](../os/configuration.md#原生类型表示与封装)的BOOL/int、pthread错误int、syscall long及altstack long-double存储保留ABI类型及自然对齐。主文允许为控制外接口作取舍，typedef/main例外不扩至任意局部数值；平台/ABI变化重评，已知宽度本身不消除违反。 |
| D4.8 | Advisory | 指令／C99 | retained | 按TC1逐TU合并同一底层结构/联合的全部指针后判断（主文30–31；TC1印刷1）。[实际声明/对象/回调与TU闭包](#d48完整表示与隐藏边界)区分隐藏、实质访问及完整表示需要，按§6.2.3独立保留所列共享配置/协议包、native/kernel/SDK表示和五CDS的真实Advisory不符合；不是满足，不外推159/124或任意新源码。 |
| D4.9 | Advisory | 指令／C99 | retained | 可互换时优先函数（主文31–32；AMD2.26，AMD3.6增加_Generic例外）。本节独立记录三RTE错误宏、四OS tick宏、所选OSError访问宏及实际配置/调用点宏的范围、原文、类型/求值控制；[声明宏](../os/implementation.md#声明宏与表达式参数)不属普通函数可替代语法。保留真实宏形式，不由底层inline、旧诊断选择或_Generic例外宣布全项满足。 |
| R1.1 | Required | TU／C99 | satisfied | 不违反C语法/约束/实现翻译上限（主文37–38；AMD2.27）。正常入口逐TU的-Wall/-Wextra/-Werror编译与真实链接/运行覆盖所列封存源码；[受控扩展](../os/platform.md#c99-平台扩展的使用边界)由所选GCC接受，原文明确允许实现支持的扩展。扩展的R1.2建议和ABI/运行语义另核，不以生成成功代替编译。 |
| R1.5 | Required | System／C99 | satisfied | 不使用所选C版次的过时特性（AMD3.14、60及Appendix F）。[C99构造检查](../types.md#c99过时形式与名称有效长度)无K&R/空参数/gets等；[Windows实际ld](../os/implementation.md#windows-外部名字与所选工具)的2.47.20260726字节与已核发行包一致，COFF/hash链按完整名字处理。Linux已核ELF/hash链同样不截短至255以下；不靠当前名字短、编译成功或版本推出实现上限。 |
| R3.2 | Required | TU／C99 | satisfied | 行注释内不可反斜杠续行（主文46；AMD2.31删除说明Note）。所列双平台两组八份C/H原始物理行及307种文件字节的续行候选为空；[Doxygen转换及宏续行](language.md#注释与字面量的实际词法边界)逐构造区分，宏指令/完整块注释中的续行不属行注释。 |
| R12.5 | Mandatory | TU／C99 | satisfied | 不对数组形式形参本身取sizeof（AMD1§2.1，3；AMD2.27）。[逐函数数组形参](language.md#sizeof的对象与数组形参)连接Can data、Host data、ExecutionTransmit、Target frame、read_line、内核名称检索和PipelineTest_Copy；实际sizeof取得局部数组、元素或具名对象，容量另验证。不是一般禁止sizeof表达式，也未使用Mandatory偏离。 |
| R21.22 | Mandatory | TU／C99 | not_applicable | tgmath操作数须为允许的essential算术类别，部分宏不允许complex（AMD3.49，24–25；不把输出指针当操作数）。[所选库构造](README.md#源码语言边界)与两组双平台交付C/H无tgmath.h或其宏调用/展开；这是触发构造缺席，不是C99语言豁免或Mandatory偏离。 |
| R21.23 | Required | TU／C99 | not_applicable | 多操作数tgmath宏在整数提升后须有相同standard type，frexp/remquo输出参数除外（AMD3.49，25–26）。同一有限源码/实际入口没有这些宏，沿[库构造证据](README.md#源码语言边界)；不从R21.22缺诊断推定满足。 |
| R21.24 | Required | TU／C99 | not_applicable | 禁止rand/srand调用及相应宏展开（AMD3.50，26）。所选自有/补丁C/H及producer没有rand/srand用途；[固定上游与项目补丁责任](../os/adoption.md)不把原始SDK实现内名字归项目调用。引入随机入口时重评。 |
<!-- misra-manual-end -->

## D4.8：完整表示与隐藏边界

D4.8为Advisory（主文印刷30–31／PDF38–39）。TC1印刷1／PDF7将前提限定为：同一TU中，
**同一底层struct/union的全部指针均未解引用**。先合并typedef、const/volatile、二级指针、成员及返回值的类型关系，
再逐TU判断；别的TU读过字段不能替本TU销项。以下D表示展开源中有实质字段/下标/整对象访问，
O表示完整对象、初始化、sizeof或步长需要；O不充当规范例外，sizeof(*p)、&array[i]和写出二级指针不冒充读取结构。
仅在实现.c定义的类型对外隐藏；Linux不完整HANDLE和原生_Unwind_Context保持隐藏。

源证据仍绑定[共同冻结范围](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)，不自动评估新seal。
Linux四profile逐份读取已有有效compiler-origins展开（component-pipeline取实际完成输出）；其C/H与#397输入逐字节相同。
Windows读取d58的54份实际展开，其219份C/H与#397 user-application逐字节相同。
两target各四profile间仅36个C/H路径不同：四应用/模板、生成配置/路由、RTE/SchM头和源；这些路径按同profile跨target逐字节相同，且没有target条件分支。
故Windows的公共native/kernel展开复用已核输入，profile差异逐一按Linux四份展开和这36路径核对；没有把Linux端口当作Windows端口。
变化包括ECU_MAX_PDU_PAYLOAD的64/4及组件CDS集合，均保留具体配置，不假定全部宏相等。
#401八份实际summary的目标/预定义、逐TU纳入集合及SDK原头集合另行相连；每target四份原头SHA映射相同，Windows284份与d58全相同。
这里只核对已有原始输入/展开和来源，没有新编译或扫描；typedef used和定位索引均不作为解引用证明。

每份程序的TU集合来自summary.programs：Linux S/U各54、C/N各56，Windows S/U各53、C/N各55；
两个程序仅入口分别为integration/src/ecu_host_batch.c、ecu_probe.c。S/U应用为app/EchoApplication/EchoApplication.c；
C/N替换为app/{Ingress,Process,Observe}/同名.c；DcmService没有独立app TU。下文T为两程序TU并集：

- bsw/src/{BswM,Can,CanIf,CanTp,Com,ComM,Dcm,Det,LSduR,PduR}.c；
- generated/src/{CanIf_PBcfg,Det_Callouts,Ecu_Config,Ecu_Execution,Ecu_RuntimeConfig,LSduR_PBcfg,Os_Arti,PduR_PBcfg,Rte,Rte_OsService,SchM_Main}.c；
- integration/src/{Can_Host,Can_HostLock,Det_Host,Ecu_HostBatch,Ecu_HostBridge,Ecu_HostBusSM,Ecu_OsHooks,Ecu_Target,SchM_CanIf_Host,SchM_Diagnostic_Host,SchM_Mode_Host,ecu_host_batch,ecu_probe}.c；
- os/src/{Arti,Os,Os_ArtiServices,Os_Backend,Os_Error,Os_HostIo,Os_HostServices,Os_Interrupt,Os_Mailbox,Os_PortRestart,Os_Schedule,Os_Time,Os_Vector}.c；kernel/{tasks,list,queue}.c；
- Linux另含os/src/host/linux/{Os_HostLinux,Os_LinuxNative,Os_StackLinux}.c及kernel/portable/ThirdParty/GCC/Posix/port.c；Windows另含os/src/Os_Stack.c、os/src/host/windows/Os_HostWindows.c及kernel/portable/MSVC-MingW/port.c；以及上述应用TU。

下表basename均按此展开；S为相应Os_StackLinux/Os_Stack，P为相应port，R为Ecu_RuntimeConfig。
B={Ecu_Target,Os,Os_ArtiServices,Os_Backend,Os_Error,Os_Interrupt,Os_Mailbox,Os_PortRestart,Os_Schedule,S,Os_Time,P}；
A=B∪{Det_Callouts,Ecu_Config,Ecu_Execution,R,Os_Arti,Rte,Rte_OsService,SchM_Main,Ecu_HostBridge,Ecu_OsHooks,ecu_host_batch,ecu_probe}；
G=A\{Rte_OsService}；E={Det_Callouts,Ecu_Config,Ecu_Execution,R,Rte,SchM_Main,Ecu_HostBridge,Ecu_Target,ecu_host_batch,ecu_probe}。
“声明TU”含实际激活的原型、成员及返回指针；它不是实际运行调用的声明。D/O另核全部对象、取址、数组退化、强转和间接调用，未造新用途。
完整定义按下列header及[OS/内核/SDK接缝](../types/public-types.md#d48的内核类型别名与对象边界)追溯；生成副本按manifest追到runtime或core producer，不按目录重定owner。

| 类型／完整定义owner | 声明TU（均省略.c） | D及O；其他声明/转发TU保留完整表示 |
| --- | --- | --- |
| BswM_ConfigType：BswM.h | BswM,ComM,R | D:BswM；O:R静态配置。 |
| ComM_ConfigType：ComM.h | BswM,ComM,Dcm,CanIf_PBcfg,R,Ecu_HostBusSM | D:ComM；O:R。 |
| Can_ConfigType、Can_PduType：Can_CfgTypes.h、Can_GeneralTypes.h | Can,CanIf,Can_Host,Ecu_Execution,R,Ecu_Target | D:Can；O:R的driver及CanIf的发送描述符。 |
| CanIf_{Config,RxPduConfig,TxPduConfig}Type：CanIf.h；Can_HwType：Can_GeneralTypes.h | Can,CanIf,LSduR,CanIf_PBcfg,Ecu_Config,R,LSduR_PBcfg,PduR_PBcfg,Ecu_HostBridge,Ecu_HostBusSM,Ecu_Target,ecu_probe | D:CanIf；O:PBcfg路由数组/R配置，Can.c的硬件描述符。 |
| CanTp_ConfigType：CanTp.h | CanTp,R,LSduR_PBcfg,PduR_PBcfg,Ecu_Target | D:CanTp；O:R。 |
| Dcm_ConfigType：Dcm.h | Dcm,R,LSduR_PBcfg,PduR_PBcfg,Ecu_Target | D:Dcm；O:R。 |
| Com_ConfigType/Com_PduConfigType：Com.h | Com,R,LSduR_PBcfg,PduR_PBcfg,Rte,Ecu_Target | D:Com；O:R的PDU表/配置。 |
| Det_ConfigType：Det.h | Can,CanIf,CanTp,Det,LSduR,PduR,R | D:Det；O:R。 |
| LSduR_{PBConfig,RxRoute,TxRoute}Type：LSduR.h | CanIf,CanTp,LSduR,R,LSduR_PBcfg,PduR_PBcfg,Ecu_Target | D:LSduR；O:LSduR_PBcfg及R。 |
| PduR_{PBConfig,RxRoute,TxRoute,TpRoute}Type：PduR.h | Dcm,PduR,R,LSduR_PBcfg,PduR_PBcfg,Ecu_Target | D:PduR；O:PduR_PBcfg及R。 |
| PduInfoType：ComStack_Types.h | Can,CanIf,CanTp,Com,Dcm,LSduR,PduR,CanIf_PBcfg,Ecu_Config,R,LSduR_PBcfg,PduR_PBcfg,Rte,Ecu_HostBridge,Ecu_HostBusSM,Ecu_Target,ecu_probe | D:CanIf/CanTp/Com/Dcm；O:Can的Rx info。PduR和LSduR所有info参数只判空/转交，回调在下游解引用不计入本TU。 |
| RetryInfoType：ComStack_Types.h | CanTp,Dcm,PduR,R,LSduR_PBcfg,PduR_PBcfg,Ecu_Target | D:Dcm；PduR的retry只沿CopyTxData回调转交。 |
| SchM_ConfigType：Rte_Main.h | T除CanIf_PBcfg,Can_Host,Can_HostLock,Det_Host,Ecu_HostBusSM,list,queue,tasks,Arti,Os_HostIo,Os_HostServices,Os_Vector及各target的Host适配TU | D:SchM_Main；O:R；Linux的Host适配TU为Os_HostLinux/Os_LinuxNative，Windows为Os_HostWindows，S仍在集合中。 |
| Ecu_HostBusSM_ConfigType：Ecu_HostBusSM.h | CanIf_PBcfg,R,Ecu_HostBusSM | D:Ecu_HostBusSM；O:R。 |
| Os_TargetConfigType、Os_TaskConfigType、Os_ResourceConfigType、Os_InternalResourceConfigType、Os_IsrConfigType、Os_ActivationInfoType：Os_Target.h | G | TargetConfig在{Os,Backend,Error,Interrupt,Mailbox,Schedule,Time}读Os_Config成员，Windows另有P；Task在{Os,Backend,Schedule,Time}，Resource/InternalResource/Isr在{Os,Backend}。ActivationInfo仅Backend的*info=snapshot，Os.c只转交；R构造const配置，Target借用Ecu_OsConfig。Os即Os.c，其余简称省略Os_前缀。 |
| Os_TimeConfigType、Os_CounterConfigType、Os_AlarmConfigType、Os_ExpiryActionType、Os_ExpiryPointType、Os_ScheduleTableConfigType、Os_TickCompletionType：Os_Time.h；Os_HookConfigType：Os_Hooks.h | G | Time/Counter在{Os_Time,Os_Schedule}读字段；Alarm在Time；ExpiryAction/Point/ScheduleTable在Schedule；Hook在{Backend,Error}。TickCompletion仅Time的*record=completion；Target.batch_time只转交，Target/Bridge/probe另有完整局部对象。R定义静态配置/到期点数组。 |
| Ecu_HostBatch：Ecu_HostBatch.h | E∪{Ecu_HostBatch} | D:Ecu_HostBatch/Ecu_HostBridge/ecu_host_batch；其他TU见既有inline Initialize的memset/sizeof，不借它称运行时解引用。 |
| Ecu_BatchFrame：Ecu_HostBatch.h；Ecu_OutputRecord/ProtocolRecord/BatchCompletion：Ecu_Target.h | Frame为E∪{Ecu_HostBatch}；其余E | Frame在HostBatch整元素赋值、HostBridge及Target字段/下标访问；Output在Target整写和host-batch字段读；Protocol/Completion在Target整写。O:HostBridge局部记录，Probe的output/tick存储；不把这些借用当本TU指针解引用。 |

CanTp_TimerType/WorkType、Dcm_WorkType、Can_HostLockType分别仅在CanTp.c、Dcm.c、Can_HostLock.c定义并访问；
Ecu_Input/OutputSlot/ProtocolSlot仅在Target，Ecu_HostWatch仅在Bridge，完整定义不向别的TU发布。
PortTask仅Linux port.c、ThreadState_t仅Windows port.c：完整局部对象由字节复制与按值返回承载原生句柄，保持实现私有。
Os_ActivationQueueType/Os_NativeTaskStartType、Os_CounterWorkType、Os_InterruptStateType的完整定义分别在Backend/Time/Interrupt；registered/gate两个HANDLE成员均已纳入，未遗漏逗号后的声明。
Std_VersionInfoType定义在Std_Types.h；Arti.h已有inline GetVersionInfo在A∪{Arti}确实通过指针写字段，
这属于该TU源代码事实，不宣称每个程序轨迹都调用它，也不扩大供应商身份结论。

五个Rte_CDS类型完整定义在各组件头，Rte.c定义私有const实例和Rte_Inst_*；Rte_Instance按组件解析，不合并同名别名。
Echo的声明TU为{EchoApplication,R,Rte}；DcmService为{R,Rte}；C/N的Ingress/Process/Observe分别为{同名应用,R,Rte}，N没有DcmService。
这些TU均无CDS指针解引用；Rte.c有实际完整对象/初始化需要，其余仅声明/传递。五实体/11处定义与[R2.8](../types/declarations.md#单实例对象与函数地址的人工评估)分别处置。
SWS_Rte_02311/03711/03712/03715/03793/03724没有强制应用头暴露完整struct，ABI probe不是生产用途或D4.8满足。

本轮按主文§6.2.3作**精确Advisory保留**：保留表内共享完整配置/协议表示、上述仅转交端点和五CDS头布局；不声称AUTOSAR唯一要求此表示。
替代为分离完整定义/不完整接口头、为调用方对象另设构造与存储接口、或改变CDS/静态配置交付；它们会改变这些实际header依赖、初始化/借用或生成合同，不能只删一个字段/加一个假读取。
当前选择保持已受验的静态const配置、类型化回调/输出、原有布局及局部对象生命周期。风险是转发TU可见不需要的布局并可能误改字段；
控制为const配置/const CDS、既有配置拒绝、单owner与借用期、正常公开消费者/字节seal及原始诊断。可隐藏的Linux句柄/私有实现不扩展此保留。
新增字段/指针、回调、调用TU、初始化/重启/存储、宏边界、provider或target变化时重评；不借159/124、R18或旧R2.8决定授权D4.8，不宣称总体符合。
