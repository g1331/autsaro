# 标准类型、平台类型与通信栈类型

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |

公开类型的 R2.3 处置提案见[标准类型声明](types/public-types.md)，其独立复核状态与源码注释阶段分开记录。

## C99 essential type 与实际常量

Boolean／character诊断、原生位掩码及复合表达式的具体判断见[essential type与表达式](types/essential-types.md)。

## 源码诊断的语法边界

R7.3／R12.3 的历史报告裁决与选定源码的人工证据见[下文](#r73-的标识符误报)。局部误报、源码修正和规则评估分别成立，不能用报告消失替代后者。

`Ecu_ExecutionTransmit` 的 sink 为 `Can_TxSinkType` 函数指针，`(void)sink` 只显式丢弃此参数；实际发送沿 Can 的 flush → Ecu_ExecutionTransmit → Ecu_TargetEnqueueTransmit 进入受控队列。R11.1（Required，主文页93–94／PDF101–102）的例外2明确允许函数指针转 void，此处没有转成 void 指针、整数或其他函数指针类型；AMD2.58 只修订例外3的说明，不撤销例外2。固定 addon 对丢弃函数指针与改成不兼容函数指针的对照均报告；只对前者按既有原文例外裁决，不扩大为通用指针转换许可。

当前standard-ecu的Det.c:16一条R8.2报告也须按真实声明判断：Det_ValidHooks的hooks／count均具名，Det.h中的Det_ErrorHookType四个参数为ModuleId、InstanceId、ApiId、ErrorId，没有省略的参数类型或名字。主文R8.2（Required，印刷64–66／PDF72–74）及适用修订不要求把已具名的函数指针typedef重新内联进形参。本处按源码判为误报，不与生成runnable的真实nameless role冲突、SDK的无名参数或整个R8.2混为同一结论；原始报告保留。

## 数组初始化与重复指定

R9.3／R9.4的具体初始化闭包见[初始化层级](bsw-general/language.md#初始化层级与实际求值)。补充原文边界：主文印刷78–80／PDF86–88中，R9.3明确允许`{0}`、仅由指定初始化器组成的稀疏数组和字符串初始化；R9.4按AMD4.47–50（印刷33／PDF40）还须检查指定位置之后的位置初始化是否重新覆盖先前子对象，不能只比较成员名文本。两条均为Required、当前C99适用，AMD2.27追加C11。

`f1986d7d`四份os-aliased-service工程复核保留同一结论：priorities容量为32、唯一指定索引为30，符合稀疏例外；tasks／counters／alarms及路由等数组由列表推导长度。相应自有源码没有新增重复指定或回退后的位置覆盖；采用tasks.c仍按实际单核及关闭运行时统计的配置判断。不外推SDK初始化宏、多核或任意用户应用，原始人工状态保持。

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

## R7.3 的标识符误报

PR #371 的四份 `direct-render-mapping-final-results` 各20个位置为标识符：call四处、cell十四处、all两处，分别属于资源宏、原子／futex及多对象等待；两个程序共40次报告。固定addon未先要求数字token，误认名字末尾的l；仅裁决这些位置。后续[参数名修正](bsw-general/language-tokens.md#原始-token-分类的具体误报)及 #420 的声明修正保留既有语义；历史误报裁决仍成立。固定R12.3还漏报过return中的真实逗号运算符，不能从报告减少推定通过。

### R7.3／R12.3：四 profile 的有界人工证据

冻结基线为 #420 合并的 `8bdb1c7f4190764fa27f49a54c5779ccbfe371fb`，范围为 standard-ecu、user-application、component-pipeline、no-diagnostic 的 C99 HostBatch／probe。PDU ID／长度依次为16／16、16／16、8／32、8／8位；末项无应用DID／诊断路由，仍纳入Dcm TU。

持久原件的七份SHA256SUMS均已核：2013主文R7.3为Required（印刷61–62／PDF69–70），对象为数值literal后缀的小写l；R12.3为Advisory（印刷106／PDF114），对象为逗号运算符，包含for初值／增量及嵌套实参中的表达式。AMD2.27仅追加C11适用性，AMD1／3／4与TC1／2未改变这两条的对象、类别或例外；当前仍为C99。

证据复用 Linux decl58 四份完整扫描及 [#420 精确head运行](https://github.com/g1331/autsaro/actions/runs/41549133111) 的 Windows 四份完整分析与输入工件。逐份核对seal、非内核副本、target／程序及当前originalBswSources的C／头／补丁字节，顺序重建内核并逐字节比较。Linux的92项runtimeSources摘要与基线一致；精确身份及核查收据归 #58 本阶段PR。

责任沿[ADR 0025](../adr/0025-misra-layered-scope.md)区分：originalBswSources映射自有runtime；其余生成／应用C由实际producer负责；FreeRTOS V11.3.1的`054e14f3397023aa83813a65aa065fc4597d481b`原件与生产patch分开，以scope.patchedLines判断改动行，不把整个kernel算作采用原件。SDK依[原头来源](os/sdk.md#sdk-原头来源类别)绑定实际包、头摘要、TU及活动宏；路径或来源分类本身不是满足。

R7.3：两平台四份交付C／头（含未激活分支、补丁后内核）逐token核对，没有小写l数值后缀或追加小写l的token拼接；call／cell／all、注释和字符串不算literal。Linux四份实际GCC展开也无此后缀。Windows每份展开有八处：Backend三处CONTEXT_CONTROL、Time一处WAIT_TIMEOUT、项目patch后的MSVC-MingW port各一处WAIT_TIMEOUT／CONTEXT_CONTROL、HostBatch一处ERROR_BROKEN_PIPE，以及采用queue.c的一处SIZE_MAX。前七处由`__MSABI_LONG(x)=x ## l`产生0x1l／258l／109l；末处沿SIZE_MAX→_UI64_MAX产生0xffffffffffffffffull。源码未手写后缀不代表展开满足。

Windows头包为mingw-w64-x86_64-headers `14.0.0.r569.g34eeafb7d-1`，编译器GCC `16.1.0-5`；活动定义分别见winnt.h:1795、winerror.h:356／248、limits.h:76及_mingw_mac.h。主文§6.7（印刷15–16／PDF23–24）区分ISO标准库实现与调用方义务。SIZE_MAX是ISO C99标准接口，其内部后缀按此边界处理；不扩及非ISO Windows接口。上述七处非ISO展开的R7.3责任仍由 #58 承接，尚未完成满足／合法处置，不新增偏离。

R12.3：Linux四份实际GCC展开的56／56／58／58个TU由Clang18.1.3完整解析，无逗号运算符。Windows的55／55／57／57个TU另按行标记保留项目／采用／patch源码及使用点宏展开，C语法解析无错误；GNU属性另核为段、对齐、调用约定及noreturn／used标记，无逗号运算符。每份仅六处真实逗号表达式，均是生成Os_Arti.c的assert在标准assert.h:46展开的`(_assert(...),0)`，按§6.7归标准库内部语法；条件仍为计数非零或0。未发现该活动语法中的其他逗号运算符。声明成员、函数调用、数组初始化和宏参数均按实际语法区分，不从checker零报告推定。

消费者为资源宏、原子／等待、栈／故障、资源计数及native_starts；采用tasks／list／queue、所选port与生成配置／RTE／ARTI进入两个程序。对照识别小写后缀、return／for逗号运算符，排除大写后缀与分隔符。

Windows完整SDK内部、未展开宏的其他实例及供应方实现仍未评估。Clang的GCC16头错误不作阴性证据。新增源码／宏用途、配置、补丁、provider或target时重评。#381／#382的诊断分类及#385的66项记录不重做；155项checker-covered与raw221项仍not_assessed，passed=false／error、completeCompliance=false、retention键／形状及strict refusal不由本文改写，不声称总体符合性。

## 函数指针的显式丢弃

R11.1为Required、当前C99适用（2013主文印刷93–94／PDF101–102）。Exception 2明确允许函数指针转换成void，不能把void与void指针混同。AMD2追加C11，AMD2.58修改Exception 3的隐式转换说明，没有撤销Exception 2；其余相关修订未增加这里的限制。

四份 `direct-render-mapping-final-results` 各有一处R11.1，均为 `generated/src/Ecu_Execution.c:7:5` 的 `(void)sink`，归属host-batch／probe。实际 `Can_TxSinkType` 为返回EcuStatus、接受id／dlc／八字节const数据指针的函数指针；目标类型确为void，没有星号或转成另一函数原型。因此该具体报告适用Exception 2，是误报。这里不调用sink、不保存变换后的地址，实际ECU发送继续进入Ecu_TargetEnqueueTransmit；不由这项判断证明其他callback、参数用途或地址表示符合。

ECU producer从 `runtime/ecu/src/Ecu_Execution.c/.h` 的保留模板交付这些文件，四份分析副本及Can_CfgTypes.h均与实际模板／头字节相等；不假称Emitter输出具有直接原件映射。固定addon的misra_11_1在目标不再是函数指针时报告，该分支未先排除void，解释了这条报告。未改检查器、源码、suppression或baseline；原raw、strict refusal及221项not_assessed保留，不扩为整条R11.1或任意指针转换通过。

## 未使用声明与真实消费者

公共声明、条件定义及真实使用的判断见[声明用途](types/declarations.md#未使用声明与真实消费者)。

## C99过时形式与名称有效长度

R1.5为Required，本仓库C99适用（AMD3.14，印刷7／PDF15；AMD3.60重置的Appendix F，印刷41／PDF49）。C99对应#1–5、#7–9；表中的ATOMIC_VAR_INIT和零长度realloc属于C18，不能混入此判断。所核四个Linux工程的实际声明、生成入口和采用补丁闭包没有旧式空参数／K&R定义、标准storage-class位置倒置、bool/true/false重定义或gets／受限ungetc调用；POSIX原件中的prvFatalError已由所选补丁整段删除，不拿未补丁输入判交付违反。此源码结论不涵盖任意外部应用。

Appendix F #2／C99 §6.11.3另要求核对外部名字的实现有效长度，不能由名字较短或GCC版本替代。[GCC13.3实现说明](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc/Identifiers-implementation.html)将外部名字长度交给linker。当前解析的GNU ld为binutils2.42-4ubuntu2.10，字节与官方deb一致；对应发行源码的bfd_link_hash_lookup进入bfd_hash_lookup，按完整字符串计算长度、复制len+1并用strcmp比较，没有小于255字符的前缀限制。已核发行补丁未改变这两个函数；ELF名字使用字符串表，当前构建没有名字截断选项或编译器搜索路径覆盖。该证据支持这一Linux工具组合，不是所有linker的属性。Windows2.47-4的实现材料及实际linker选择边界见[OS](os/implementation.md#windows-外部名字与所选工具)；没有运行名字边界实验。完整身份与原文证据集中PR #371。

callback-parameter旧7处R2.4均为匿名类型误报；新增xSTATIC_TIMER为真实未用tag，现由0015省去，保留StaticTimer_t及布局（Breaking说明同步）。原报告保留，typedef用途另按R2.3判断。

## 链接可见性与实际引用

外部接口、私有helper及检查器的TU去重限制见[链接可见性](types/declarations.md#链接可见性与实际引用)。

## 单函数对象与存储期

块作用域、静态／TLS存储期与独立MemMap段的关系见[单函数对象](types/declarations.md#单函数对象与存储期)。

## 原生ABI类型与编译约束

Os_HostTypes.h集中原有atomic／handle／thread callback别名，Os_PortTypes.h集中原有PC与向量回调别名；类型头不承载对象／函数映射。LONG在Windows仍为32位long、Linux仍为int32_t；64位存储、HANDLE／DWORD／LPTHREAD_START_ROUTINE、Windows void*／Linux uintptr_t PC以及uint32(void)向量回调均保留。Schedule的私有别名直接使用原有Os_ArtiScheduleStateType，不改tables布局或工具view。

原四个Os_Atomic guard及ArtiAtomic32IsFourBytes／Os_HostLongMustBe32BitsType的char-array typedef名字没有消费者，约束作用却必须保留。现在匿名枚举的显式值使用sizeof(char[原谓词?1:-1])：受支持目标的界为常量1，失败目标因负数组界在编译期拒绝；不生成状态对象、VLA或运行时分支，Linux always-lock-free拒绝仍独立存在。六个宽度／对齐谓词未放宽，旧实现guard类型名的源码移除已记CHANGELOG。基础存储／回调别名及服务二进制ABI不变。

正常编译控制分别改变32／64位宽度与对齐，拒绝须定位到对应guard表达式；将同一guard的检查表达式替为常量1的私有副本必须编译成功，避免把其他检查的失败算作目标证据。sizeof中的Os_Atomic64Type仍是真实类型用途，不能因旧addon遗漏而一同删除。上述只处置类型归属和编译约束，GNU扩展、SDK、provider及实际Windows执行继续单列。

`48907b3b`的type-owner标准ECU完成结果另有三条新增R17.3，位于交付Os_HostTypes.h:23的两次及:30的一次`__alignof__`。同一批原R2.3由60减至56、R5.9由17减至0；R17.3则由71增至74，raw合计2862，不把净减少当作没有新报告。原比较工件为os-aliased-service，具体位置仍按各自字节身份保留。

主文R17.3（Mandatory，印刷138／PDF146）仅适用C90，当前实际TU为C99；三处是编译器对类型的对齐查询，不是遗漏函数原型。这三个GNU运算在旧guard中已经存在，改换常量表达式上下文没有新增一次运行时调用。固定addon的misra_17_3按未关联function／valueType的名字加左括号报告，没有完整建模该扩展。实际C99编译及对应约束拒绝另作证据；不手写内建声明、不新增baseline／suppression，原报告、语言适用性、GNU扩展责任和严格比较拒绝分别保留。

维护者接受的平台／OS宏形式见[精确预处理记录](types/retained-platform.md)，不扩大所选配置的运行能力。
