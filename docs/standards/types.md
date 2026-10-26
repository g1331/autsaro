# 标准类型、平台类型与通信栈类型

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |

## C99 essential type 与实际常量

Boolean／character诊断、原生位掩码及复合表达式的具体判断见[essential type与表达式](types/essential-types.md)。

## 源码诊断的语法边界

R7.3（Required，主文页61–62／PDF69–70）禁止字面量后缀中的小写 l。所选 Linux 样本报告中的 cell／all 是变量或参数标识符，不是字面量；固定 addon 的正则未先要求数字 token，能同时匹配这些名字与真正的 `1l`。`OS_RESOURCE_CREATE(call)` 两种定义中的 call 同样是宏形参，两个替换列表均保留该形参的括号，不是带后缀的字面量。原始报告保留，不以重命名合法标识符代替裁决；此结论不覆盖其他字面量。

R12.3（Advisory，主文页106／PDF114）针对逗号运算符。所选 Linux 样本报告的 OS 字段／局部变量及采用内核局部变量均为声明中的 declarator 分隔符，不求值逗号表达式；固定 addon 对拆分声明标记直接报告。最小声明对照有报告，真实逗号运算符置于 return 表达式的对照却没有，故同时存在误报与漏报边界。两项规则在 AMD2 中增加 C11 适用性，不改变这些 C99 语法条件；完整位置、原始输出及对照收据见 PR #371。

`Ecu_ExecutionTransmit` 的 sink 为 `Can_TxSinkType` 函数指针，`(void)sink` 只显式丢弃此参数；实际发送沿 Can 的 flush → Ecu_ExecutionTransmit → Ecu_TargetEnqueueTransmit 进入受控队列。R11.1（Required，主文页93–94／PDF101–102）的例外2明确允许函数指针转 void，此处没有转成 void 指针、整数或其他函数指针类型；AMD2.58 只修订例外3的说明，不撤销例外2。固定 addon 对丢弃函数指针与改成不兼容函数指针的对照均报告；只对前者按既有原文例外裁决，不扩大为通用指针转换许可。

当前standard-ecu的Det.c:16一条R8.2报告也须按真实声明判断：Det_ValidHooks的hooks／count均具名，Det.h中的Det_ErrorHookType四个参数为ModuleId、InstanceId、ApiId、ErrorId，没有省略的参数类型或名字。主文R8.2（Required，印刷64–66／PDF72–74）及适用修订不要求把已具名的函数指针typedef重新内联进形参。本处按源码判为误报，不与生成runnable的真实nameless role冲突、SDK的无名参数或整个R8.2混为同一结论；原始报告保留。

## C11 原子对象的适用边界

AMD4 新增的 R9.7（Mandatory，AMD4.6，印刷页10／PDF17）、R11.10（Required，AMD4.15，页12／PDF19）、R12.6（Required，AMD4.16，页13／PDF20）及 R21.25（Required，AMD4.32，页18–19／PDF25–26）均适用于 C11。已核对的双平台四样本及当前四个 Linux 样本，交付 C／头文件与采用内核均没有 `_Atomic`、标准 `atomic_*`／`memory_order*` 标识符或 `stdatomic.h` 入口；逐文件词法收据与冻结源码身份留在 PR #371，不外推系统头的预处理闭包。

R9.7 没有需要按该条初始化的 C11 原子对象；R11.10 没有施加 `_Atomic` 的 void 不完整类型；R12.6 没有被直接访问成员的原子结构体／联合体；R21.25 没有使用其规定内存序的 C11 标准库同步操作，故在上述源码范围分别不适用。这不是 Mandatory 豁免，也不把 GNU 原子内建视为 C11 标准库函数。Linux 实际使用的 `__atomic_*` 操作、发布顺序与 lock-free 前提仍按[平台扩展及并发边界](os/platform.md#c99-平台扩展的使用边界)核查；这些操作存在的事实没有从收据中排除。

## 泛型选择的适用边界

AMD3.51 的 R23.1–R23.8 均只适用于 C11 泛型选择（AMD3 印刷页27–37／PDF35–45）。已核对的双平台四样本及当前四个 C99 生成样本，其交付 C／头文件和采用内核没有 `_Generic`，也没有 `tgmath.h` 泛型宏入口；这是实际构造的缺席依据，不仅是编译参数的语言标签。冻结源码身份及逐文件核对收据留在 PR #371。

R23.1（Advisory）的泛型宏展开位置与 R23.2（Required）的非宏控制表达式副作用条件均不存在，故在上述源码范围不适用。R23.3（Advisory）没有可检查的非 default association，R23.4（Required）没有 association 类型列表，分别不适用。R23.5（Advisory）没有基于指针类型的泛型选择，R23.6（Required）没有需要比较 essential／standard type 的控制表达式，分别不适用；未借用 R23.6 的整数常量例外。R23.7（Advisory）没有泛型宏实参的求值次数，R23.8（Required）没有 default association 的位置，分别不适用；未借用 R23.7 的常量结果例外。

这八项的适用性结论只针对上述源码；新增泛型选择或引入会展开它的库宏时重新核查。C99 中已有的 essential type、宏参数求值、指针转换和持久副作用仍按各自适用规则评估，原始 checker 的 `not_assessed` 目录状态不由这段人工结论改写。

## 整数常量、初始化与浮点构造

R7.5（Mandatory、C99，AMD3.16，印刷9–10／PDF17–18）约束整数常量宏的原始实参；AMD4.55只增加R7.6交叉引用。已核`ab2ea459`双平台八份补丁后工程及`eea9f25d`四份Linux工程：项目实际调用为`UINT32_C`／`UINT64_C`，实参均为单个无后缀十进制或十六进制literal，分别在32／64位无符号范围内；最大者为restart标识`0x4155544f53525431`。实际宏展开追加U／UL／ULL不等于原始实参带后缀。Counter转换宏的输入配置与生成十进制常量另沿生成器范围检查，不能用这十二份样本覆盖所有合法配置。

R7.6（Required、C99，AMD4.4，印刷7–8／PDF14–15）按目标`int`宽度限定small integer宏；AMD4.65属性表的Advisory与正文／AMD4.64不一致，保留正文Required。两目标`int`为32位、`int_least16_t`为16位。默认样本未启用RES_SCHEDULER，不能证明启用路径满足：`ecu.rs::os_configuration`原来的`UINT16_C(1)`已改为`1u`，保留`uint16 task_access`的bit0、资源优先级和指针／计数。实际四输入原生消费者与恢复小整数宏的源码观察分别验证行为和编码约束，不借用ADR例外。

R9.6（Required、C99，AMD4.5，印刷8–9／PDF15–16）由同一initializer中的chained designator触发。上述工程的项目／采用／补丁后初始化器未发现该构造；`.priorities = {[OS_INPUT_INTERRUPT] = 1u}`是两层独立花括号列表，ErrorHook快照也是单个外层designator加内部位置初始化。`sa_sigaction`的SDK宏展开虽有两级成员，实际用途是初始化后的赋值，`handler = {0}`没有chained designator。此结论不是C99语言不适用，也不由checker缺席推导。

D4.15（Required、C99，AMD3.10，印刷5–6／PDF13–14）针对可能生成NaN／Inf的浮点算术或数学函数结果。上述十二份交付C／头文件、宏及补丁后内核中，浮点类型仅见`Platform_Types.h`的float32／float64定义、两个portmacro的portFLOAT／portDOUBLE定义，以及Linux备用栈的`long double`存储／sizeof；没有浮点值运算、浮点literal或数学函数入口。诊断字符串、注释和`#error`文本不是浮点表达式，FP上下文的字节复制也不求浮点值。因此这些具体源码没有待检测的NaN／Inf结果；R14.1（Required，主文印刷117／PDF125，AMD2增加C11）也没有essentially floating循环计数器。两项都仍适用C99，不能把构造缺席写成语言豁免；系统头展开和provider内部不由本段证明。

R8.15／R8.16／R8.17仅适用C11，类别分别为Required／Advisory／Advisory（AMD3.17，印刷10–13／PDF18–21）；本项目C99源码未使用`_Alignas`／`alignas`／`stdalign.h`，三项分别不适用。GNU aligned属性仍按平台扩展和实际布局约束核查。R1.4仅适用C11（AMD2.30，印刷11–12／PDF18–19）；合成AMD3.11–13与AMD4.53–54后不能再沿旧清单批量禁止已另设指导的泛型／noreturn／对齐／线程／原子设施。当前C99不适用本条，Annex K及平台扩展仍按实际接口检查，不能据此升级语言或免除R1.1／R1.3。

## 未使用声明与真实消费者

R2.3／R2.4／R2.5均为Advisory、System scope、适用C99（主文印刷43–44／PDF51–52；AMD2增列C11）。R2.4直接区分tag与typedef名，原文允许省去仅在定义处出现的tag；TC1确认#undef算宏使用，TC2.5／6将R2.5用语改为macro definitions，不能只比最终宏表或把replacement-list名字出现当成实际展开。

Linux私有兼容头的ULONG／ULONG_PTR、CALLBACK／VOID没有所选Linux消费者，已删除；Windows的同名使用由Windows SDK提供。portBASE_TYPE与portINTERRUPT_APPLICATION_DEFINED_START也没有真实消费者，受控POSIX补丁删除两条冗余定义，实际BaseType_t／UBaseType_t和向量起点逻辑不变。编译期宽度／对齐拒绝typedef仍有声明本身的约束作用，不能因名字后续无引用而删除；Os_Atomic64Type在这些约束中仍被sizeof／alignof消费。

采用内核的xSTATIC_EVENT_GROUP／xSTATIC_STREAM_BUFFER／xTASK_PARAMETERS只在typedef定义的tag位置出现，既有0015补丁将其省去，保留三个typedef名称、全部成员和条件布局；上游原件不改写，派生身份按完整有序补丁重新计算。直接引用这三个struct tag的外部源码须改用保留的typedef名，来源兼容影响已记CHANGELOG；当前实际交付／ARTI／内核消费者没有这些tag引用。LSduR、PduR、TryToGetSpinlockType及eSleepModeStatus的四处tag报告则落在匿名enum上，没有可删除的tag；这些具体报告按原始C语法判为误报，typedef名是否使用另按R2.3判断。

Dcm六个私有NRC重复了已交付的标准常量，实际负响应路径现直接使用生成的DCM_E_*；SWS_Dcm_00980原表PDF365的0x10／11／12／13／14／31分别约束general reject、service、subfunction、format、response too long及range。该头由Dcm.h原有包含链提供，没有新增消警报引用。Ecu_TargetPrepare的初始CAS同样使用已有ECU_TARGET_UNPREPARED，仍表示原子状态0，拒绝顺序不变。

其余所选程序未使用的标准公共类型、版本／状态／NRC常量、RTE句柄／runnable映射、OS访问宏及可选内核接口保留真实未使用状态；固定SWS发布要求、配置／接口稳定性与采用接口的理由不等于实际消费，也不将Advisory违反自动判为不适用。MemMap的HEADER_CHECK由同文件#ifndef路径核对，不能套用START／STOP关键词#undef的结论。完整逐位置的来源、实际消费者／配置条件及仍待处置项集中PR #371；上述删除／误报裁决不关闭整组三条或全部221项。

条件定义进一步按每个可执行程序的TU集合及定义／撤销时序核对（Glossary code／project，印刷223／225、PDF231／233）：上述四份旧工件的R2.5报告中，前三份各196处、no-diagnostic为194处定义在真实配置下从未发生，包括152个MPU映射、未导入的co-routine／newlib／picolibc、10个MemMap HEADER_CHECK及部分多核／动态创建／tick变体；前三份另有两个被RTE_CORE排除的DcmService别名。不能仅凭最终宏表缺席判定，须同时排除曾定义后#undef的情况。相反，timers头被tasks.c无条件包含；其消费者关闭并不排除宏定义，22个timer宏等61个实际生效定义仍未使用。完整精确位置与有效宏证据归PR，不把条件排除转为整组R2.5通过。

固定addon按宏名合并并以同名后续#define覆盖位置，还会把真正的未使用定义定位到排除分支。上述196／194个位置中7个名字另有实际生效且未使用的定义，分别是FreeRTOS.h的configPRECONDITION_DEFINED及三个空registry宏、tasks.c的单核taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD、projdefs.h的32位pdINTEGRITY_CHECK_VALUE、timers.h的xTimerDelete；这些实际定义不能随错误位置一起销项，也不和原始位置重复计数。已生效定义仅在未展开宏的replacement list中出现同名引用，同样不能当作使用证明。原始报告、真实未使用状态及未复评部分分别保留。

## C99过时形式与名称有效长度

R1.5为Required，本仓库C99适用（AMD3.14，印刷7／PDF15；AMD3.60重置的Appendix F，印刷41／PDF49）。C99对应#1–5、#7–9；表中的ATOMIC_VAR_INIT和零长度realloc属于C18，不能混入此判断。所核四个Linux工程的实际声明、生成入口和采用补丁闭包没有旧式空参数／K&R定义、标准storage-class位置倒置、bool/true/false重定义或gets／受限ungetc调用；POSIX原件中的prvFatalError已由所选补丁整段删除，不拿未补丁输入判交付违反。此源码结论不涵盖任意外部应用。

Appendix F #2／C99 §6.11.3另要求核对外部名字的实现有效长度，不能由名字较短或GCC版本替代。[GCC13.3实现说明](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc/Identifiers-implementation.html)将外部名字长度交给linker。当前解析的GNU ld为binutils2.42-4ubuntu2.10，字节与官方deb一致；对应发行源码的bfd_link_hash_lookup进入bfd_hash_lookup，按完整字符串计算长度、复制len+1并用strcmp比较，没有小于255字符的前缀限制。已核发行补丁未改变这两个函数；ELF名字使用字符串表，当前构建没有名字截断选项或编译器搜索路径覆盖。该证据支持这一Linux工具组合，不是所有linker的属性。Windows2.47-4的实现材料及实际linker选择边界见[OS](os/implementation.md#windows-外部名字与所选工具)；没有运行名字边界实验。完整身份与原文证据集中PR #371。

callback-parameter旧7处R2.4均为匿名类型误报；新增xSTATIC_TIMER为真实未用tag，现由0015省去，保留StaticTimer_t及布局（Breaking说明同步）。原报告保留，typedef用途另按R2.3判断。

## 链接可见性与实际引用

R8.7为Advisory（主文印刷70／PDF78；TC2.15印刷6／PDF12）。定义在一个TU、从另一个TU调用是合规情形；“只有一个调用方”不等于“只在一个TU引用”。standard-ecu当前完成工件报告142处；检查器的MisraUsage生产者用dict列表保存名字，却以字符串检查是否已存在，未按TU去重，汇总又用记录数等于1作判断。同TU有多个usage记录的对象可漏报，不能以142作为全部义务清单。

按实际cache/files.txt连接两个程序的现成CTU、将使用按TU去重并加入定义TU后，分别得到165／168个单TU候选；各有23名因多条同TU记录未进入原报告。这是定位候选，不是新增诊断或对外部汇编／工具引用的完整判定。五个仅所属文件使用的私有helper已收窄：Os_HookServiceAllowed、Os_TimeTick、Os_HostCurrentThread、Os_PortThread、Os_BackendMissingEnd。需要前置声明的MissingEnd移到Backend的原有CODE区，声明及定义均为static；两个平台PortThread定义同步。函数体及其他C token不变，不新增MemMap请求或改变状态、调用顺序和参数类型。

其余候选按真实接口保留：Arti_Events／EventCount／EventsDropped／DevelopmentError、Os_ArtiTasks／TaskStacks用于外部工具视图；pxCurrentTCB、vTaskGetInfo、xQueueGenericReset保留既有采用内核／调试及公开能力；三个Com／ComM配置MainFunction、两个OsService_SystemCounter包装、Rte_Call及生成Task入口属于交付头、配置／应用绑定。静态化会改变相应可见性或客户端源接口，不能只因当前例子没有额外具名调用就删除。Ecu_TargetAssertOwner仍是交付头声明的内部BSW边界能力；Os_HostStackRange还有os_signal_lifecycle独立C消费者，不能按标准样本的单TU结果删除。上述保留理由不把项目接口或上游惯例写成AUTOSAR强制，也不消除所选程序中成立的Advisory违反；固定SWS确实要求的接口仍逐项按ADR0023记录，不据此作整个R8.7通过。

## 单函数对象与存储期

R8.9为Advisory、System（主文印刷71／PDF79；AMD2.27追加C11，AMD4.43–46／印刷32／PDF39将defined改为declared）。只有一个函数出现的标识符应优先收窄到块作用域；静态存储期不是保留文件作用域的充分理由，主文的调用计数示例本身就使用函数内static。地址被异步消费者借用时也须分别核对对象存储期与标识符可见范围，不能把二者混同。

55077f87四profile分析中的standard-ecu有46个报告对象，不按TU／程序重复计数。十个无独立MemMap数据段属性的私有对象已保留原类型／限定符、静态或TLS存储期并移入唯一消费函数：Target的native_owner／native_thread、protocol_write／read、next_output_ticket及三个output索引，Det_Host的reporting_development，以及HostBatch输入control的batch。Target仍以同一原子发布身份，队列索引及ticket不随调用重置；Det仍每个原生线程首次进入后保持递归边界；batch仍是H控制线程借用的进程期对象，执行／observer回收协议不变。除这些声明位置，三个文件的其他非注释C token不变。

另一个已收窄对象是Linux端口dispatcher_thread：在xPortStartScheduler原创建点定义const HANDLE，保留创建、注册等待和永久等待的原顺序。只是句柄值的局部副本，不把actor、映射或回调上下文的所有权改为自动存储期；后续有序补丁上下文和manifest一并同步。Windows不选择这两份Linux补丁。

其余32个自有／生成对象目前保留真实Advisory违反及映射理由：Can和Com各一个发布状态、八个Ecu_RuntimeConfig配置对象、Rte的rte_allocated、HostBusSM的publication、Backend六个状态／入口记录、Error的TLS guard、Interrupt状态、Mailbox四个函数私有状态、HostLinux句柄池及StackLinux七个altstack／fault／终止文本对象。它们现有独立初始化／清零／常量／TLS段属性与CODE区分开；直接把带相同OS_MEMMAP等宏的声明移到CODE函数内会取得不同的活动属性。继续收窄需要另证块内数据映射设计及交付段身份，不能删除数据映射或在活动CODE区内嵌套START／STOP来消警报。这是现有映射下保留布局的工程理由，不表示AUTOSAR要求这些C标识符必须处于文件作用域，也不宣称其他等价映射设计不可能。

采用内核另三个对象保留明确用途：tasks.c说明xDelayedTaskList1／2的文件级静态符号供kernel-aware debugger定位；uxTopUsedPriority为OpenOCD提供优先级界限，具名C引用用于保留该对象。列表指针逃逸本身并不禁止块内static，调试符号兼容才是这里保留的理由；本仓库没有据此证明OpenOCD实际运行或赋予采用代码普遍豁免。上述11处整改、32处映射理由及三个调试对象是该46处的有界处置；新字节检查与其他未报告构造另核，原始诊断及not_assessed不改写。
