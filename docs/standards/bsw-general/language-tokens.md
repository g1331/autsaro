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
| D4.8 | Advisory | 指令／C99 | not_assessed | 仅某类型在一个TU中的全部指针都不解引用时，应隐藏完整布局（主文30–31，TC1澄清）。[类型owner、对象用途及两target四profile程序/TU集合](#d48完整表示与隐藏边界)分别记录私有实现、不完整句柄、静态配置/整对象需求及公开CDS不符合边界。补丁与SDK的逐TU全部指针链仍须闭合；单字段消费者、泛ABI或159/124保留不能替代。 |
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

D4.8 为 Advisory（主文印刷30–31／PDF38–39）；TC1把前提明确为**同一TU中该类型的全部指针均未解引用**。
指令没有规则的System/TU Analysis属性，但这个前提仍须逐TU判断。下列核对不能用某个函数读过一个字段，
推成其他TU也满足。完整对象定义、数组步长、sizeof、整对象赋值和字段访问分别记录；
其中“需要完整类型来分配对象”是当前表示的工程需要，不是指令提供的豁免。

源集合绑定上文引用的#397最终收据和#393的1306冻结交付，不推广到后续head。
每份summary.programs[].translationUnits给出两个独立程序的实际TU集合：Linux S/U各54、C/N各56；
Windows S/U各53、C/N各55。S/U指standard-ecu、user-application，C/N指component-pipeline、no-diagnostic。
两个程序仅入口分别为integration/src/ecu_host_batch.c、ecu_probe.c；S/U应用TU为app/EchoApplication/EchoApplication.c，
C/N替换为app/{Ingress,Process,Observe}/同名.c，DcmService没有独立app TU。
公共TU是bsw/src/{BswM,Can,CanIf,CanTp,Com,ComM,Dcm,Det,LSduR,PduR}.c，
generated/src/{CanIf_PBcfg,Det_Callouts,Ecu_Config,Ecu_Execution,Ecu_RuntimeConfig,LSduR_PBcfg,Os_Arti,PduR_PBcfg,Rte,Rte_OsService,SchM_Main}.c，
integration/src/{Can_Host,Can_HostLock,Det_Host,Ecu_HostBatch,Ecu_HostBridge,Ecu_HostBusSM,Ecu_OsHooks,Ecu_Target,SchM_CanIf_Host,SchM_Diagnostic_Host,SchM_Mode_Host}.c，
os/src/{Arti,Os,Os_ArtiServices,Os_Backend,Os_Error,Os_HostIo,Os_HostServices,Os_Interrupt,Os_Mailbox,Os_PortRestart,Os_Schedule,Os_Time,Os_Vector}.c，
及kernel/{tasks,list,queue}.c。Linux另有os/src/host/linux/{Os_HostLinux,Os_LinuxNative,Os_StackLinux}.c与
kernel/portable/ThirdParty/GCC/Posix/port.c；Windows另有os/src/Os_Stack.c、os/src/host/windows/Os_HostWindows.c、
kernel/portable/MSVC-MingW/port.c。头内声明只归入实际包含它的TU，不能另算一个程序或把未激活的#ifdef分支当消费者。

交付名不是源码owner：generated/src/Os_Arti.c来自runtime/os，Ecu_RuntimeConfig/配置表及Rte来自core/src/integration的producer，
其它bsw/os/integration副本按交付manifest追溯runtime。内核原件、每个有序补丁及补丁后字节分开核对。
#393各target四profile的kernelPatches、补丁字节和kernelSha256一致；按正常git apply包含集合在临时副本重放后核验全部摘要。
其include/task.h、queue.h及FreeRTOS.h与#397相同；tasks.c、queue.c和端口实现有差异，不以类型头相同宣称两个交付完整等同。

| 类型集合／完整定义owner | 实际对象、指针用途与结论边界 |
| --- | --- |
| CanTp_TimerType、CanTp_WorkType：bsw/src/CanTp.c；Dcm_WorkType：Dcm.c；Can_HostLockType：integration/src/Can_HostLock.c | 完整定义留在各自实现TU；本地状态/锁对象及其指针不向公共头发布完整布局。 |
| Ecu_HostWatch：Ecu_HostBridge.c；Ecu_Input、Ecu_OutputSlot、Ecu_ProtocolSlot：Ecu_Target.c | 实现TU内的observer上下文、输入/输出队列槽；结构定义私有，回调借用按真实生命周期，未把void*转换视为新的公开结构。 |
| Os_ActivationQueueType、Os_NativeTaskStartType：Os_Backend.c；Os_CounterWorkType：Os_Time.c；Os_InterruptStateType：Os_Interrupt.c | 队列/启动参数/计数器/中断状态对象及字段操作在定义TU，头不暴露完整表示。Linux PortTask和Windows ThreadState_t亦仅各自port.c定义，按补丁归属核对，未把所有port行归为采用原件。 |
| Linux struct Os_HostHandleType：Os_HostLinux.c；HANDLE/Os_HostHandleType指针别名：Os_HostLinuxTypes.h/Os_HostTypes.h | Backend、Time、StackLinux、HostServices、HostBridge及两个入口传递/比较句柄；这些TU只见不完整类型。实际成员访问在定义TU。Windows HANDLE是SDK void*，不是这个Linux结构。 |
| BswM_ConfigType、CanIf_ConfigType/RxPduConfigType/TxPduConfigType、CanTp_ConfigType、Com_ConfigType/Com_PduConfigType、ComM_ConfigType、Dcm_ConfigType、Det_ConfigType、LSduR_PBConfigType/RxRouteType/TxRouteType、PduR_PBConfigType/RxRouteType/TxRouteType/TpRouteType | 完整定义各在bsw/include/同模块.h；模块实现保存const配置指针并访问表/路由字段。Ecu_RuntimeConfig及各PBcfg实际定义配置对象/数组；Ecu_Target等传址不因此成为字段消费者。保留公开完整配置是现有静态配置表示，转发TU的隐藏建议不能由模块内部字段访问销项。 |
| Can_ConfigType：Can_CfgTypes.h；Can_HwType/Can_PduType：Can_GeneralTypes.h；PduInfoType/RetryInfoType：ComStack_Types.h；Std_VersionInfoType：Std_Types.h | Can读取配置/发送描述符；CanIf读取硬件/PDU描述符；CanIf/CanTp/Com/Dcm/LSduR/PduR按各API访问PduInfo，Dcm/PduR处理RetryInfo；生成配置实例化driver。Arti的VersionInfo指针原型仍须与实际程序用途区分，公开声明本身不证明读写。 |
| Ecu_BatchFrame/Ecu_HostBatch：Ecu_HostBatch.h；Ecu_BatchCompletion/Ecu_OutputRecord/Ecu_ProtocolRecord：Ecu_Target.h；Ecu_HostBusSM_ConfigType：Ecu_HostBusSM.h | HostBatch解析和入口有实际帧/批对象，HostBridge访问批字段，Target访问帧并以*record赋值输出/协议记录，以*result复制完成结果；HostBusSM读配置。仅HostBridge转交output指针的路径不借Target的解引用宣称满足。 |
| Os_TargetConfigType/TaskConfigType/ResourceConfigType/InternalResourceConfigType/IsrConfigType/ActivationInfoType：Os_Target.h；Os_HookConfigType/ErrorParametersType：Os_Hooks.h；Os_ErrorContextType：Os_Backend.h | Ecu_RuntimeConfig定义const对象，Os_Config的嵌套tasks/resources/internal_resources/hooks/interrupts指针由Os/Backend实际索引/访问；Error保存/恢复上下文及整个参数对象。Os_TargetInspectActivation在Os.c仅转交info，BackendInspect才以*info=snapshot写整个对象，不能把前者判成已有解引用。 |
| Os_CounterConfigType/AlarmConfigType/ExpiryActionType/ExpiryPointType/ScheduleTableConfigType/TimeConfigType/TickCompletionType：Os_Time.h | 配置/到期点静态对象来自producer；Time/Schedule访问对应层级字段。Time以*record复制tick完成结果，Target有实际局部结果对象及转发。须检查同类型所有参数/成员/别名，不能只搜箭头。 |
| AlarmBaseType/AlarmBaseRefType：Os_Types.h；Os_ArtiAlarmStateType/Os_ArtiScheduleStateType：Os_Arti.h | Time的GetAlarmBase写Info三个字段；Os_Service包装与Os_Arti地址捕获仅转发。Time/Schedule的状态数组及别名有真实读写；工具view指针的只读捕获不等于该指针已解引用，也不承接R18或159/124的其它批准。 |
| Os_NativeStackType/Os_StackFaultType：Os_StackTypes.h；Os_HostTextType：Os_HostIo.h；Os_InputRecordType：Os_Mailbox.h | Backend/各target栈实现的数组、sizeof/offsetof/字段，HostIo与host-batch文本缓冲，Mailbox与Target输入对象需要当前完整表示。Os_StackIndex通过&array[index]取得地址需要元素大小；不把它描述成字段读取。 |
| Linux CONTEXT：Os_HostLinuxTypes.h；TCB完整定义：kernel/include/task.h；[各别名声明及队列](../types/public-types.md#d48的内核类型别名与对象边界) | CONTEXT含完整ucontext，Backend/PortRestart/LinuxNative/HostLinux/StackLinux有实际字段或对象。TCB完整定义由项目补丁移入task.h；Backend的task_memory静态数组及task_slot地址/步长，idle回调输出StaticTask_t**，内核任务字段访问均保留来源。Backend不读TCB字段，不把其完整类型需要等同隐藏建议已满足。 |
| Rte_CDS_EchoApplication/DcmService/Ingress/Process/Observe：各Rte_组件.h；Rte_Instance是各组件独立别名；SchM_ConfigType：Rte_Main.h | Rte.c定义私有const CDS对象并初始化五个公开const指针；各profile精确对象见[R2.8](../types/declarations.md#单实例对象与函数地址的人工评估)。没有production解引用公开指针；组件头仍暴露Rte_Dummy。SchM_Main访问SchM配置。Rte_Instance不能因同名合并成一个类型。 |

最后一行的完整CDS表示是明确的Advisory不符合边界：SWS_Rte_02311要求应用头中的type declaration，
03711/03712/03715/03793规定CDS、私有实例命名、const及handle命名，03724约束空CDS内容；
它们没有证明完整struct必须在应用头定义。当前保留头布局属于工程取舍，ABI probe只是回归证据，
不是D4.8满足或AUTOSAR必需冲突。隐藏布局需连同实际producer/组件头/配置消费者评估，不在#382冻结范围竞争改写。

SDK来源另算：Linux Os_PortRestart使用原生_Unwind_Context不完整指针和_Unwind_Exception对象/指针；
LinuxNative的timespec指针直接传futex，stack_t输入/输出通过syscall借用，ucontext/sigcontext/_fpstate涉及真实布局检查与复制。
Windows PortRestart的EXCEPTION_RECORD/DISPATCHER_CONTEXT指针访问异常字段/上下文，CONTEXT参数按回调合同保留；
Os_Stack的EXCEPTION_POINTERS访问两层记录，MEMORY_BASIC_INFORMATION对象供VirtualQuery写入。
这些SDK原始定义保持采用边界，自有调用仍负责，不能按调用方目录重定owner；原生API要求完整对象也不自动豁免所有转发TU。

本节已固定program/TU集合、直接类型/声明owner及上述对象用途；词法索引与CTU typedef可见性仅辅助定位，
不证明全部指针均未解引用。D4.8整体仍not_assessed：还须逐TU完成补丁内TCB/队列/端口别名及SDK来源指针的全部成员链、
整对象/间接调用用途核对，结合最终SDK取证身份，把每个转发TU明确分入已隐藏、完整表示需要但保留不符合、或实际解引用。
现有实例/字段证据不外推尚未闭合的全部指针集合；#398最终来源取证集成后复核这部分身份，不重新扫描已有效冻结样本。
