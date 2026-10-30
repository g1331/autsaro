# 公共标准类型的未使用报告

本篇只记录当前四 profile 中的公开声明诊断，不把接口存在等同于符合性。
授权依据是 ADR 0023 及其原用户决定；以下15项已获独立复核支持，并落实精确源码 MR12。
保留原始诊断、源码身份和 not_assessed，不改变规则或基线。

## 必要性与风险

SWS_Platform_00002 明确要求平台抽象类型与符号在 Platform_Types.h 发布；固定 R24-11 的各类型定义表进一步规定类型名、表示／范围及 Available via 头。所选程序未使用的标准声明仍真实未使用；删除或私有化会改变相应标准头。声明不启用 OS-Application／保护／可信调用／多核服务，也不把这些未选服务记为已实现。单核 SC1 未选功能的类型另按下节裁剪，不援引发布冲突。

同名类型改为宏或复制底层类型不能同时保持公开 typedef 契约；不以增加空引用消除报告。风险限公共名称及未来误用，既有配置拒绝与 C99 公共头消费者继续约束实际支持。标准／类型表示、支持配置或调用方变化时重评。

| 规则 | 精确范围 | 原文与所保留的契约 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8` | SWS_Platform_00016 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16` | SWS_Platform_00017 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint64` | SWS_Platform_00067 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8_least` | SWS_Platform_00023 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16_least` | SWS_Platform_00024 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint32_least` | SWS_Platform_00025 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint8_least` | SWS_Platform_00020 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint16_least` | SWS_Platform_00021 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint32_least` | SWS_Platform_00022 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float32` | SWS_Platform_00041 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float64` | SWS_Platform_00042 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `VoidPtr` | SWS_Platform_91001 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `ConstVoidPtr` | SWS_Platform_91002 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/LSduR.h` 中 `LSduR_StateType` | CP_SWS_LSduR_91005 要求的 UNINIT／ONLINE 枚举；LSduR 原条款为 DRAFT。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/PduR.h` 中 `PduR_StateType` | SWS_PduR_00742 要求的 UNINIT／ONLINE 枚举。 | 维护者 原用户决定，ADR 0023；独立复核支持 |

## 单核 SC1 未选功能的声明裁剪

当前目标由配置入口拒绝 OS-Application、可信调用、保护、多核 spinlock 和 peripheral 区域。
复核固定 SWS_Os_00241 的 PDF130–131 表格、00764、00516、00542 后，未发现当前配置无条件发布
这些类型的义务。原声明只有 public_types.c 的清单消费者，已移除15个 typedef 及专用宏，
不新增虚假功能开关。支持功能的类型／服务签名和对象布局保持；源码合同变化见 CHANGELOG。

不适用（配置：ECUC_Os_00259 只接受 SC1、目标固定单核，所列功能未选择且配置入口拒绝，Os_Types.h 不发布其声明）：SWS_Os_00772、SWS_Os_00773、SWS_Os_00774、SWS_Os_00775、SWS_Os_00776、
SWS_Os_00777、SWS_Os_00778、SWS_Os_00779、SWS_Os_00780、SWS_Os_00781、
SWS_Os_00787、SWS_Os_00788、SWS_Os_00791、SWS_Os_00792、SWS_Os_91000。

public-types 正常入口继续按六种头顺序编译／运行 SC1 的独立范围及指针消费者；
另逐个编译15个未支持类型的引用，拒绝必须定位到输入第2行和对应类型名。
恢复一个类型声明会使该项拒绝断言失败，不能由其他头错误冒充。

## 类型与初始化的人工评估

使用[共同配置、原件与冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)；
本节不扩大上表R2.3的范围。`TU`为Single Translation Unit，`System`须核对跨TU身份；
C11条目逐项按规定对象和当前C99源码构造判断，不豁免GNU扩展及自有调用方。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| R6.3 | Required | TU／C99 | satisfied | 位域不能直接为union成员，union内struct子对象不在禁止集合（AMD3.15，8–9）。实际Dcm_MsgAddInfoType的两个uint8位域直接属于struct；[类型/位域依据](../dcm/interfaces.md)及双平台所选项目/补丁声明没有直接union位域，SDK原件不改归自有。 |
| R7.5 | Mandatory | TU／C99 | satisfied | 整数常量宏实参为单个无后缀literal且在相应exact-width范围（AMD3.16，9–10；AMD4.55仅交叉引用）。[整数常量闭包](../types.md#整数常量初始化与浮点构造)及本轮两组八份交付分别28/26组实参均合法，最大restart值0x4155544f53525431。ecu.rs::configure_os把u32配置转u64后以十进制输出，tick_ms乘1000000不超u64；非负有界输入不产生符号/后缀/表达式实参。 |
| R7.6 | Required | TU／C99 | satisfied | 不用宽度小于int的minimum-width常量宏（AMD4.4，7–8；正文/AMD4.64为Required，AMD4.65表冲突不降级）。两目标int32；[启用RES_SCHEDULER路径](../types.md#整数常量初始化与浮点构造)已以1u替换UINT16_C(1)，实际producer及两组交付没有INT8/UINT8/INT16/UINT16_C用途。无诊断并非证据。 |
| R8.15 | Required | System／C11 | not_applicable | 同一对象各声明的显式alignment须在宏展开后词法相同（AMD3.17，10–11）。所选自有/补丁源码无_Alignas/alignas/stdalign.h；[对齐边界](../types.md#整数常量初始化与浮点构造)中的GNU aligned另按ABI与实际对象布局核查。 |
| R8.16 | Advisory | TU／C11 | not_applicable | alignment表达式不能取零（AMD3.17，12）。同一源码集合没有C11 alignment说明符；不把GNU对齐参数或没有alignment说明符当作显式零。 |
| R8.17 | Advisory | TU／C11 | not_applicable | 一个声明至多一个显式alignment说明符（AMD3.17，12–13）。同一源码没有该说明符，故没有需要合并的多个alignment；平台属性仍守实际ABI约束。 |
| R9.6 | Required | TU／C99 | not_applicable | chained designator所在initializer list要求各项显式指定，内层独立花括号有原文例外（AMD4.5，8–9）。[初始化构造](../types.md#整数常量初始化与浮点构造)中.priorities={[OS_INPUT_INTERRUPT]=1u}是两层列表，当前无混用chained/隐式位置项；sigaction SDK宏用于赋值，不是chained initializer。 |
| R9.7 | Mandatory | System／C11 | not_applicable | 原子对象先初始化后访问；静态默认初始化有效，非静态用声明初始化或atomic_init（AMD4.6，10–11）。[原子对象边界](../types.md#c11-原子对象的适用边界)及当前交付无_Atomic/stdatomic.h/标准atomic入口；GNU__atomic不是C11原子对象初始化。 |
| R11.10 | Required | TU／C11 | not_applicable | 不能把_Atomic施加于不完整void（AMD4.15，12）。同一有限源码无该类型；普通void指针转换仍受R11系列及[精确地址类别](../os/native-facilities/addresses.md)约束。 |
| R12.6 | Required | TU／C11 | not_applicable | 不可直接访问原子struct/union成员，应以整个对象的原子操作处理（AMD4.16，13）。同一源码无原子聚合对象，CONTEXT的FP/字节复制不是原子成员访问。 |
| R17.13 | Required | TU／C99 | satisfied | 不能限定函数类型；限定返回类型/函数指针对象不违反本条（AMD3.38，20–21）。[真实callback typedef](../os/configuration.md#c99-数组与回调类型)均为函数指针，配置const限定指针或数据对象；Task声明宏形成函数入口，没有const/volatile/restrict限定的函数typedef。 |
| R23.1 | Advisory | TU／C11 | not_applicable | generic selection只应从函数式宏展开，控制表达式取宏实参（AMD3.51，27–28）。[泛型边界](../types.md#泛型选择的适用边界)及当前所选C/H无_Generic；未用宏例外来批准普通重复求值。 |
| R23.2 | Required | TU／C11 | not_applicable | 非宏generic控制表达式不得有潜在副作用，函数调用也计入（AMD3.51，28–29）。同一源码无generic选择，因而无待核控制表达式；普通sizeof/逻辑表达式仍独立评估。 |
| R23.3 | Advisory | TU／C11 | not_applicable | generic至少一个非default association（AMD3.51，29–30）。同一源码没有association，不由C99标签推定已有列表合法。 |
| R23.4 | Required | TU／C11 | not_applicable | association类型不能是会经lvalue转换消失的顶层限定、原子、数组、函数或匿名struct/union定义（AMD3.51，30–31）。同一源码无association；不混淆指向限定对象的指针类型。 |
| R23.5 | Advisory | TU／C11 | not_applicable | 指针generic选择不依赖隐式指针转换决定default（AMD3.51，32–33）。同一源码没有指针选择，算术选择本身也不触发本条。 |
| R23.6 | Required | TU／C11 | not_applicable | controlling expression的essential/standard type须匹配，特定非char/非Boolean整数常量表达式有例外（AMD3.51，33–35）。同一源码没有该表达式，未借常量例外扩大普通转换许可。 |
| R23.7 | Advisory | TU／C11 | not_applicable | generic宏实参应在整个展开中恰求值一次；各association均为整数常量表达式的type query有例外（AMD3.51，35–36）。同一源码无generic宏，没有以宏名或无副作用推定任意重复求值可接受。 |
| R23.8 | Required | TU／C11 | not_applicable | 有default时须在association首或尾（AMD3.51，36–37）。同一源码没有association/default；没有default本身不违反此条。 |
<!-- misra-manual-end -->

## D4.8的内核类型别名与对象边界

沿用[完整TU集合及记号](../bsw-general/language-tokens.md#d48完整表示与隐藏边界)；其余声明/转交TU保留完整表示，不计作满足。

| 类型／完整定义owner | 声明/派生指针TU | D与完整表示O |
| --- | --- | --- |
| AlarmBaseType/AlarmBaseRefType、Os_ErrorParametersType：Os_Types.h、Os_Hooks.h | A | Time.GetAlarmBase写Info三个字段；所有其它Info只包装/捕获地址/保存为service_GetAlarmBase.Info。Error整体读取sources[]的对应服务对象；Os/Backend/Interrupt/Schedule/Time在栈上构造const arguments，调用时借用并由Error复制，不读被指向AlarmBase。地址捕获不等于结构访问，不外推R18批准。 |
| Os_ErrorContextType：Os_Backend.h | B | Error读写context字段；Backend创建完整局部context供保存/恢复。其它TU仅见协议声明。隐藏需改变这类调用方栈存储或分离完整/不完整接口。 |
| Os_ArtiAlarmStateType/Os_AlarmStateType、Os_ArtiScheduleStateType：Os_Arti.h | B∪{Os_Arti} | 两个名字解析到同一Alarm底层类型；Time读写alarm_states[i]，Schedule访问schedule_states/target指针。全局tool view指针/其它TU不借这两个TU的访问称满足。 |
| Os_NativeStackType：Os_StackTypes.h | B∪{Os_Arti}，Linux另加Os_HostLinux | D:{Os,Os_ArtiServices,Backend,Error,Interrupt,S,P}，Linux另有Mailbox读actor->role；Windows Mailbox仅判NULL。其它TU的Os_StackIndex只比较&Os_ArtiStacks[i]，需要元素步长而不读字段。原21槽及地址/只读工具视图保持，移出inline需新的链接/包含闭包复核。 |
| Os_InputRecordType：Os_Mailbox.h | B∪{Ecu_Config} | Mailbox以*record=records[read_index]复制；Target有局部record借出供写入。 |
| Os_HostTextType：Os_HostIo.h | {ecu_host_batch,ecu_probe,Backend,Os_HostIo,S}，Linux另加Os_HostLinux | 只有HostIo指针读写length/capacity/failed；其余创建caller-owned storage/text或只见声明，转交Append/Write。完整对象及容量不是指令例外。 |

上述缩写Backend/Error/Interrupt/Mailbox/Schedule/Time分别指Os_同名文件；R为Ecu_RuntimeConfig。
源码还逐条核对了间接路径：Os_Config->{tasks,resources,internal_resources,interrupts,timing,hooks}，timing->{counters,alarms,schedule_tables}，
Schedule的expiry_points/actions，Error的sources[]及嵌套service指针；不能只搜局部变量名或箭头。
配置/数组和成员的完整owner仍是表中header，R的实例不改归emitter之外的SDK。

内核的TaskHandle_t/ConstTaskHandle_t、tskTCB/TCB_t声明在task.h；StaticTask_t别名在FreeRTOS.h，
它们指向task.h的同一struct tskTaskControlBlock，不能把StaticTask_t另当一个surrogate。
QueueHandle_t/QueueSetHandle_t/QueueSetMemberHandle_t、xQUEUE/Queue_t在queue.h，StaticQueue_t/StaticSemaphore_t在FreeRTOS.h；
完整QueueDefinition也在queue.h。完整TCB/Queue及静态别名的统一来自项目0015-typed-port-state补丁，保持逐行原件/补丁归属。

- TCB/TaskStatus_t/TimeOut_t声明闭包为K=B∪{tasks,queue}；List_t/ListItem_t/MiniListItem_t为K∪{list}。
  tasks实际读写TCB、TaskStatus/TimeOut；queue仅转交mutex holder的TaskHandle_t，另创建TimeOut局部对象交给任务API。
  list/tasks/queue真实访问List/Item字段；其它仅见嵌套成员/声明。Backend的task_handles数组只传递/比较，task_slot比较&task_memory[i]；
  idle回调*ppxIdleTaskTCBBuffer只写出指针，不读取TCB。完整task_memory/idle缓冲是本TU静态存储需要。
- Queue各别名的实际值操作仅queue.c，pxQueue及强转后的返回表达式访问字段；QueuePointers_t/SemaphoreData_t为u的完整子对象。
  QueueSet/StaticSemaphore/ConstTaskHandle别名存在不等于当前配置实际创建对象。MemoryRegion_t的唯一实际指针参数为tasks.prvInitialiseNewTask.xRegions，
  当前MPU关闭分支仅(void)xRegions；完整定义是task.h原件部分，不伪称字段消费者或改归项目TCB补丁。
- tasks/list/queue的同型别名、sentinel和TaskStatusArrayWriterContext_t转换逐位置核对；NativeTaskStart.argument→task_slot及路由回调只传递，不冒充调用者解引用。


数组和整对象另外核对：Arti_Event在Arti、Os_ArtiTaskStateType在Backend、Os_InterruptFrameType/Os_OrderReferenceType在Backend、
Ecu_BatchToken在HostBatch、CanTp_TxType/RxType在CanTp、Dcm_PublicationType/Dcm_ActorsType在Dcm，以及Linux S的匿名altstacks结构；
取址/数组指针和整元素读写追到本地实现。EcuPolicyConfig、Dcm_MsgAddInfoType/MsgContextType、Arti_AddressCapture/Os_ArtiCallerType仅按值/嵌入成员使用，不误写成“没用”。

这些声明/转交TU按[共同保留理由、替代、风险和重评条件](../bsw-general/language-tokens.md#d48完整表示与隐藏边界)处理，
不把栈存储、inline步长或类型化保存当作规范例外。const配置、显式复制、actor/owner和容量约束保持，新增实体不自动承接本记录。

## D4.8的SDK指针接缝

SDK字段正文、provider及原头摘要沿用[#401冻结SDK记录](https://github.com/g1331/autsaro/blob/a93fb32acabcf35286ebf7fe713934b35671e2c9/docs/standards/types/public-types.md#d48的sdk指针接缝)。
本轮逐SHA核对[#401八份实际摘要](https://github.com/g1331/autsaro/pull/401#issuecomment-6524748476)的scope.sdkHeaders和sdkTranslationUnits：
每target四profile原头集合相同，Windows284份与d58实际展开全相同。Windows正文仍是d58展开（原运行failure），不是原头全文或新head成功；Linux原头与有效收据匹配。
只补调用方/成员链；原件不改归项目，来源complete不代表人工完成。

| 实际结构指针/对象边界 | TU内用途与处置 |
| --- | --- |
| Linux HANDLE/Os_HostHandleType | 自有完整struct仅HostLinux.c，H=A∪{Arti,Os_HostServices,Os_HostLinux,Os_LinuxNative}的其余TU见不完整类型，已隐藏；Windows同名HANDLE是SDK void*，不混合。 |
| Linux自有CONTEXT | 声明闭包H；Native/StackLinux指针访问字段，Backend与HostLinux仅转交context。Backend、HostLinux、S、P另有完整对象/嵌入captured.native。GetThreadContext只转给CaptureContext，不冒充本TU结构解引用。 |
| Linux_Unwind_Context / _Unwind_Exception | PortRestart的前者在unwind.h不完整，prepare_trace不用、Personality借给SetIP；后者是完整TLS restart_exception，按对象身份核回调并写exception_class，再借给原生unwinder，不将两个类型合并。 |
| Linuxsiginfo_t/stack_t/timespec | siginfo仅HostLinux/StackLinux；只有guard_signal读si_addr（实际展开为_sifields._sigfault.si_addr），park/fault_stop不读。stack/timespec在Native只给syscall，HostLinux/StackLinux有完整对象和字段构造，保持这三个TU的分别判断。 |
| Linuxpthread_attr_t/pthread_mutex_t/sigset_t、sigaction/pollfd、ucontext_t/mcontext_t及_fpstate/_libc_fpstate | HostLinux的attr、mutex、mask、poll对象；Can_HostLock的mutex；StackLinux的mask/signal/altstack；Native的ucontext及FP复制/offsetof/sizeof。native.uc_link/fpregset_t等原头内部指针随快照借用/复制，字段/值由布局断言及原头身份约束。 |
| WindowsCONTEXT、EXCEPTION_RECORD/POINTERS、DISPATCHER_CONTEXT、UNWIND_HISTORY_TABLE | Backend/P有完整CONTEXT；PortRestart.context和dispatch->ContextRecord只借给RtlUnwindEx，S读取Rsp并以*context复制；异常记录在PortRestart和S读字段，异常双指针在S读两层。dispatch->HistoryTable也只转交：winnt.h:9111是完整表，Entry的FunctionEntry留在provider边界。 |
| WindowsSECURITY_ATTRIBUTES/LPSECURITY_ATTRIBUTES、MEMORY_BASIC_INFORMATION | 前者在minwinbase.h:13，Backend的PortEvent/Mutex/Thread仅转交；Linux同名别名为项目void*。后者在winnt.h:5836，S创建region交给VirtualQuery，并读State/Protect等；分清完整存储与SDK中的写入。 |

原头同时发布所需完整类型，本轮保持原件/当次借用；隐藏需窄适配或typed存储重构，不重声明SDK。
控制和重评沿共同记录，不推定SDK内部符合/资格。
