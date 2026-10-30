# 声明用途、链接与存储期

本篇承接[公共类型](../types.md)，区分实际用途、源码可见性与对象存储期。

## 未使用声明与真实消费者

R2.3／R2.4／R2.5均为Advisory、System scope、适用C99（主文印刷43–44／PDF51–52；AMD2增列C11）。R2.4直接区分tag与typedef名，原文允许省去仅在定义处出现的tag；TC1确认#undef算宏使用，TC2.5／6将R2.5用语改为macro definitions，不能只比最终宏表或把replacement-list名字出现当成实际展开。

Linux私有兼容头的ULONG／ULONG_PTR、CALLBACK／VOID没有所选Linux消费者，已删除；Windows的同名使用由Windows SDK提供。portBASE_TYPE与portINTERRUPT_APPLICATION_DEFINED_START也没有真实消费者，受控POSIX补丁删除两条冗余定义，实际BaseType_t／UBaseType_t和向量起点逻辑不变。编译期宽度／对齐拒绝typedef仍有声明本身的约束作用，不能因名字后续无引用而删除；Os_Atomic64Type在这些约束中仍被sizeof／alignof消费。

采用内核的xSTATIC_EVENT_GROUP／xSTATIC_STREAM_BUFFER／xTASK_PARAMETERS只在typedef定义的tag位置出现，既有0015补丁将其省去，保留三个typedef名称、全部成员和条件布局；上游原件不改写，派生身份按完整有序补丁重新计算。直接引用这三个struct tag的外部源码须改用保留的typedef名，来源兼容影响已记CHANGELOG；当前实际交付／ARTI／内核消费者没有这些tag引用。LSduR、PduR、TryToGetSpinlockType及eSleepModeStatus的四处tag报告则落在匿名enum上，没有可删除的tag；这些具体报告按原始C语法判为误报，typedef名是否使用另按R2.3判断。

Dcm六个私有NRC重复了已交付的标准常量，实际负响应路径现直接使用生成的DCM_E_*；SWS_Dcm_00980原表PDF365的0x10／11／12／13／14／31分别约束general reject、service、subfunction、format、response too long及range。该头由Dcm.h原有包含链提供，没有新增消警报引用。Ecu_TargetPrepare的初始CAS同样使用已有ECU_TARGET_UNPREPARED，仍表示原子状态0，拒绝顺序不变。

其余所选程序未使用的标准公共类型、版本／状态／NRC常量、RTE句柄／runnable映射、OS访问宏及可选内核接口保留真实未使用状态；固定SWS发布要求、配置／接口稳定性与采用接口的理由不等于实际消费，也不将Advisory违反自动判为不适用。旧MemMap的HEADER_CHECK按#ifndef路径裁决；现随无请求smoke分支删除，不改写旧工件结论，见[请求拒绝](../memmap/README.md#映射请求的拒绝边界)。完整逐位置的来源、实际消费者／配置条件及仍待处置项集中PR #371；上述删除／误报裁决不关闭整组三条或全部221项。

条件定义进一步按每个可执行程序的TU集合及定义／撤销时序核对（Glossary code／project，印刷223／225、PDF231／233）：上述四份旧工件的R2.5报告中，前三份各196处、no-diagnostic为194处定义在真实配置下从未发生，包括152个MPU映射、未导入的co-routine／newlib／picolibc、10个MemMap HEADER_CHECK及部分多核／动态创建／tick变体；前三份另有两个被RTE_CORE排除的DcmService别名。不能仅凭最终宏表缺席判定，须同时排除曾定义后#undef的情况。相反，timers头被tasks.c无条件包含；其消费者关闭并不排除宏定义，22个timer宏等61个实际生效定义仍未使用。完整精确位置与有效宏证据归PR，不把条件排除转为整组R2.5通过。

固定addon按宏名合并并以同名后续#define覆盖位置，还会把真正的未使用定义定位到排除分支。上述196／194个位置中7个名字另有实际生效且未使用的定义，分别是FreeRTOS.h的configPRECONDITION_DEFINED及三个空registry宏、tasks.c的单核taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD、projdefs.h的32位pdINTEGRITY_CHECK_VALUE、timers.h的xTimerDelete；这些实际定义不能随错误位置一起销项，也不和原始位置重复计数。已生效定义仅在未展开宏的replacement list中出现同名引用，同样不能当作使用证明。原始报告、真实未使用状态及未复评部分分别保留。

## 当前宏定义的具体裁决

R2.5主文印刷44／PDF52、TC1对#undef用途的更正与TC2.5／6共同适用；对象是宏定义，不是同名标识符的所有拼写。以下是现有四个Linux工程的作者核查，按每个host-batch／probe的真实TU分别判断，未改写summary或checker availability。

standard-ecu／user-application各734处报告中，233处所指定义没有发生；其余501处为实际生效但未使用的定义。component-pipeline的738处为233／505，no-diagnostic的743处为231／512。旧08239cfc的196／194及61个生效定义属于旧字节；0024／0025已把部分私有定义、timers头入口及相关声明绑定配置，不能把旧分类原样套到当前代码。上述数字是物理报告位置，不是宏名、TU或程序出现次数。

实际GCC每TU的-dM收据表明，前三份各221处、最后一份219处为唯一且无#undef的定义，其名字在全部实际TU末尾均不存在；同时核对唯一来源与入口条件，排除“先定义再撤销”。另七个名字的多个候选定义也都未生效：五个picolibc/runtime-TLS辅助、xTimerDelete和taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD。实际TLS、timers和Task-delete配置关闭；不能把未导入timers.h与导入后关闭使用混同。另五个报告位置属于排除分支，却掩盖同名实际定义：configPRECONDITION_DEFINED在FreeRTOS.h的0分支、三个registry空宏在FreeRTOS.h，以及32位pdINTEGRITY_CHECK_VALUE在projdefs.h。它们仍有未使用义务，不随错误位置销项。空FREERTOS_SYSTEM_CALL则是真正生效、未使用的当前报告定义。

其余实际生效的唯一来源定义，分别为500／500／501／508处；每处均在两个程序各自至少一个真实TU中有定义见证。保守词法闭包把排除C分支、所有条件检查及宏replacement-list依赖也纳入候选用途，仍有487／487／488／495处没有展开消费路径。四个token-pasting生产者仅生成Os_TaskEntry_／Os_IsrEntry_函数名字，不构造这些宏名；字符串与注释不是宏用途，未展开的replacement list也不自动构成消费。此判断不只依赖addon的used=false。

每份余下13个有词法候选的名字均逐路径核对：tskNO_AFFINITY的消费在关闭的多核affinity默认值；listGET_OWNER_OF_NEXT_ENTRY的FIFO替代宏分支未选择，优化选择器亦关闭；configTICK_RATE_HZ只进入未展开的ms/tick转换宏。三个taskSCHEDULER_*的C用途受INCLUDE_xTaskGetSchedulerState或timers保护，二者均0。errQUEUE_BLOCKED／YIELD只在关闭的co-routine路径；pvPortMallocStack／vPortFreeStack的分配、删除及独立heap路径均未选择。portENABLE_INTERRUPTS的C用途在多核、TCB critical nesting或co-routine分支，实际分别为1核、0、0；两个queue type常量的counting semaphore／queue-set消费者同样关闭。因此这些是生效但未使用，不是“功能关闭所以定义不属于code”。

组件管线和no-diagnostic另各有三个跨组件同名报告：Rte_InitValue_Value_Value在Ingress／Process／Observe三份头分别为1／7／9，RTE_RUNNABLE_Periodic分别指向三个实际runnable，Rte_InitValue_Result_Value在Process／Observe为2／11。它们由各自应用TU在RTE_CORE未定义时生效，八个定义都没有消费；fixed addon按名字合并，三个报告遗漏了另外五个独立定义。连同上述五个错位报告对应的实际定义，不能把摘要条数当作全部义务数，也不能用另一TU同名宏作用途证明。

这些真实Advisory项保留公开接口与采用源码的用途理由：标准版本／状态／NRC、OS getter和声明宏、生成符号／runnable／初值契约，以及FreeRTOS公开与已启用兼容接口均供对应调用者使用；当前程序未引用仍如实保留。删除它们会改变交付头及来源接口，不是零行为影响的私有死代码清理。该理由没有把每个名字宣称为AUTOSAR强制，也不授予采用代码一般豁免；固定SWS必要性及BSW00115仍按逐处证据记录。这个结论完成上述报告及已识别漏报的有界分类，不声称全部R2.5、SDK或任意新配置符合。

## 链接可见性与实际引用

R8.7为Advisory（主文印刷70／PDF78；TC2.15印刷6／PDF12）。定义在一个TU、从另一个TU调用是合规情形；“只有一个调用方”不等于“只在一个TU引用”。standard-ecu当前完成工件报告142处；检查器的MisraUsage生产者用dict列表保存名字，却以字符串检查是否已存在，未按TU去重，汇总又用记录数等于1作判断。同TU有多个usage记录的对象可漏报，不能以142作为全部义务清单。

按实际cache/files.txt连接两个程序的现成CTU、将使用按TU去重并加入定义TU后，分别得到165／168个单TU候选；各有23名因多条同TU记录未进入原报告。这是定位候选，不是新增诊断或对外部汇编／工具引用的完整判定。五个仅所属文件使用的私有helper已收窄：Os_HookServiceAllowed、Os_TimeTick、Os_HostCurrentThread、Os_PortThread、Os_BackendMissingEnd。需要前置声明的MissingEnd移到Backend的原有CODE区，声明及定义均为static；两个平台PortThread定义同步。函数体及其他C token不变，不新增MemMap请求或改变状态、调用顺序和参数类型。

其余候选按真实接口保留：Arti_Events／EventCount／EventsDropped／DevelopmentError、Os_ArtiTasks／TaskStacks用于外部工具视图；pxCurrentTCB、vTaskGetInfo、xQueueGenericReset保留既有采用内核／调试及公开能力；三个Com／ComM配置MainFunction、两个OsService_SystemCounter包装、Rte_Call及生成Task入口属于交付头、配置／应用绑定。静态化会改变相应可见性或客户端源接口，不能只因当前例子没有额外具名调用就删除。Ecu_TargetAssertOwner仍是交付头声明的内部BSW边界能力；Os_HostStackRange还有os_signal_lifecycle独立C消费者，不能按标准样本的单TU结果删除。上述保留理由不把项目接口或上游惯例写成AUTOSAR强制，也不消除所选程序中成立的Advisory违反；固定SWS确实要求的接口仍逐项按ADR0023记录，不据此作整个R8.7通过。

## 单函数对象与存储期

原文、历史收据及当前22个自有对象的逐项冲突记录见[显式映射对象](storage.md)。

## 类型前缀与首次映射请求

`48907b3b` 的自有头文件还有24处首次MemMap包含位于本文件普通内容之后，按R20.1预处理前布局均须保留判断，不能由段属性正确推出规则已满足。其中21处的前置内容是类型声明：

- BSW公共头BswM.h、CanIf.h、CanTp.h、Com.h、ComM.h、Dcm.h、Det.h、LSduR.h、PduR.h；Host公共头Can_Host.h、Ecu_HostBusSM.h。
- OS公共头Arti.h、Os_Arti.h、Os_Hooks.h、Os_HostIo.h、Os_Mailbox.h、Os_Target.h、Os_Time.h；私有头Os_Backend.h、Os_Stack.h及host/linux/Os_HostLinux.h。

另外三处是SchM_BswM.h、SchM_Can.h、SchM_Dcm.h中条件C++分支的`extern "C" {`。R20.1在预处理前应用，不能只因当前C99配置不选择这个分支而排除该物理布局；C++链接包装与类型定义也不是同一构造。

573df2db将SchM_Dcm.h的Rte_Dcm_Type.h和Rte.h两个普通依赖前移到链接包装之前，版本检查仍跟随真实导入。四份现成Rte_Main.h各自维护C++的C链接包装，独立BSW的Rte.h夹具只含状态宏；未改SchM／Dcm服务自身的链接包装或映射对。这两处普通include不再留在C内容之后，也不据此为首次MemMap位置批准保留。

固定MemoryMapping00003要求声明、定义各自完整包装，并不要求这些类型或链接包装必须放在首次START之前。已经实施的Os_HostTypes／Os_PortTypes分离说明类型owner可以重组；因而“类型必须先声明”不能证明以上21个START位置不可调整。它们不因服务原型必须映射就自动取得ADR0023的不可兼得结论；公开类型/API同头组织只是当前源码组织选择。类型头分离须保持实际类型、依赖版本检查、包含次序、资产及既有C/C++消费者，不以增加一个间接MemMap包含隐藏同一违反。

实体之后消费STOP的包含另有00003第五步的直接依据，不能与首次START共用一个替代排除结论。SchM的三个链接包装还须保持普通C++消费者的C符号绑定；把条件指令搬进映射对会违反00003禁止段内额外预处理的要求，删掉包装或手写替代声明也不是等价整改。这里仅明确具体构造和可行方向，不为24处填批准表、添加MR12或关闭R20.1；既有逐处实体记录和原raw状态继续保留。


后续将其中18处START/include前移：上述BSW九头、Host两头及Os_Arti／Hooks／HostIo／Mailbox／Target／Time／Backend。15处移除同一START/include后全文字节相等；Com／ComM／Hooks另先完整前移既有2／5／44个宏及其注释，C token顺序、指令内容和注释保持。类型不使用段属性宏，不分配代码或对象；新映射区间无额外预处理，实体顺序、类别、STOP及版本检查不变，首次包含不再位于本文件C内容之后。其余六处条件／inline／C++前缀仍分别核查；旧位置记录不改标当前结论。

## SDK typedef与处理来源

R5.6／R5.7的Required义务及关联tag例外分别见主文印刷54／55、PDF62／63。对48907b3b四份type-owner完成工件的逐TU展开，先在AUTSAR_VALUE探针之前截断，再提取typedef候选；不同声明拼写除已记录的多应用Rte_Instance外还有__sighandler_t。候选提取没有源码来源标记，不能据此证明所有同名定义来自同一头，也不把没有其他不同拼写写成整条R5.6通过。

既有四份sdk-inline-closure的来源栈显示：asm-generic/signal-defs.h:83将__signalfn_t指针命名为__sighandler_t；signal.h:72直接声明同一void(int)函数指针。后一个声明位于ISO signal.h处理范围，按主文6.7处理；前一个是非ISO UAPI，不能随之豁免。两个拼写不证明类型不兼容，也不是两个一律待批准的真实R5.6对象。相同材料中的pid_t实际来自sched.h:38，而size_t来自GCC标准stddef.h:214；同名或system标记都不能代替具体定义来源、处理栈及原文例外判断。

UAPI还有独立的R8.2义务：signal-defs.h:82的__signalfn_t(int)缺参数名。尝试移除Os_LinuxNative.h的显式包含后，Native实现语法／展开通过，但HostLinux在使用SA_ONSTACK处编译失败，StackLinux未执行；已恢复原包含。真实宏收据为_POSIX_C_SOURCE=199506L，未启用__USE_XOPEN_EXTENDED或__USE_MISC，而glibc bits/sigaction.h:60–62在这些条件下不提供SA_ONSTACK。不能只因另一个头也写有该宏就判为多余依赖，也不能拿掉备用栈行为、手写数值或定义__ASSEMBLY__来隐藏声明。当前SDK身份、原始失败、替代排除及恢复情况见[调查与更正](https://github.com/g1331/autsaro/pull/371#issuecomment-6434698119)。本项没有接受SDK保留、批准偏离或扩大ADR0024；其他typedef、未命名参数和最终provider责任继续分别核查。

进一步核对现成四份.i的进入／返回标记，pthread.h:161、204、559、1298中的四个回调类型，其内层void *参数均未命名；分别属于cleanup buffer、pthread_create、cleanup frame和pthread_key_create。外层__start_routine／__destr_function有名字不满足内层函数类型的R8.2；(void)的无参数原型不在此列。四份TU各出现这四处，且均不在ISO标准头处理栈内。linux/posix_types.h:30的__kernel_sighandler_t(int)和signal-defs.h:82另在HostLinux、StackLinux、LinuxNative三份出现；PortRestart没有这两个UAPI声明。因此本子集是六个物理声明、22次TU出现，不是六种产品运行失败或整个SDK的穷尽计数。

依据为主文R8.2（Required，印刷64–66／PDF72–74）及6.7；AMD2追加C11，TC2.10与AMD3.63只更新交叉引用，没有取消内层参数命名。pthread.h及linux/posix_types.h实读摘要分别为26eb5331…98dc22b／bc68636a…b196f；材料仍绑定sdk-inline-closure原字节，不冒充当前head的重新展开。项目实际调用pthread_create，其他声明即使没有运行调用仍已进入该TU；通过包装调用或仅删除未调用声明的消费者不能消除头内构造。更换feature flags、私改已安装SDK或手写替代原型均不是本次已验证整改。受控SDK来源补丁只补参数名可保持函数类型，但仍须核定来源、交付、许可和实际包含次序；当前保留真实Required缺口，不填批准表或MR12。


## 剩余首次映射位置的整改

六头的类型归属、C++ linkage 与直接验证见[首次映射布局](mapping.md)。

## 单实例对象与函数地址的人工评估

沿用[共同配置及冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)。
本轮按主文§6.2.3的Advisory替代记录方式，对以下R2.8实体选择保留现有表示：
保持逐组件独立const指针对象，避免在本项文档/评估入口工作中改变公开符号、对象地址/存储及组件头合同。
这是本轮有界风险委托下的新选择；不是 #382 的159键、#381的124键或R8.7记录所批准的内容。
R2.8仍真实未满足，未重分类、未disapply，也不称MISRA总体符合。

冻结 `1306e32a` 的五个唯一名字、11个物理定义如下（两目标交付字节相同）：

| profile | Rte.c中的独立指针定义 | extern位置 |
| --- | --- | --- |
| standard-ecu、user-application | Rte_Inst_EchoApplication:67、Rte_Inst_DcmService:89 | 各自Rte_EchoApplication.h、Rte_DcmService.h:32 |
| component-pipeline | Rte_Inst_Ingress:68、Rte_Inst_Process:89、Rte_Inst_Observe:102、Rte_Inst_DcmService:117 | 三应用头:34；DcmService头:32 |
| no-diagnostic | Rte_Inst_Ingress:60、Rte_Inst_Process:81、Rte_Inst_Observe:94 | 三应用头:34 |

每对象去注释后只有extern与定义两处引用；host-batch/probe不读取，ABI测试读取不改变此结论。
私有Rte_Instance_* CDS的初始化引用不是公开指针的用途。producer为 `core/src/integration/rte/`；
实际字节/程序和ABI消费者见
[#393冻结收据](https://github.com/g1331/autsaro/pull/393#issuecomment-6518916990)。

SWS_Rte_03711/03712/03715/03793只分别规定CDS、私有前缀、const及单实例handle名；
[既有RTE原文说明](../rte/README.md#单实例句柄的实际用途)没有把独立指针对象说成唯一合法表示。
宏/别名或删除会改变当前对象合同，编译成功不足以证明等价。风险为多余只读存储/符号及未来误用；
既有C99/C++11消费者约束类型/值/链接，不证明生产必要性。配置、表示、CDS、目标、消费者或合同变化即重评。
Required正式偏离及SWS_BSW_00115等独立义务不由本条Advisory取舍代替；缺失证据仍须显式保留。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| R2.8 | Advisory | System／C99 | retained | 对象定义/声明可删除而程序仍编译即为未使用（AMD4.3，7／PDF14）。仅上列5实体/11定义保持真实违反，按本节理由和§6.2.3替代记录保留现有ABI；[producer/实际程序/回归边界](../rte/README.md#单实例句柄的实际用途)分开。其它静态配置通过初始化借用、读写状态或明确工具消费者实际使用，不因R8.7保留便获得R2.8豁免。 |
| R17.12 | Advisory | TU／C99 | satisfied | 函数标识符只用于调用或前置&（AMD3.37，19–20）。[入口/跟踪宏闭包](../os/platform.md#入口及跟踪宏的实际展开)连接实际中断绑定/比较、线程/信号入口、once初始化、HostBatch回调及生成通信/诊断/Task配置；0022和平台补丁覆盖采用接缝。函数指针对象、声明符和汇编字符串内符号不是函数designator值，不机械加&。 |
<!-- misra-manual-end -->
