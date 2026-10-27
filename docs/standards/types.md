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

R7.3为Required，当前C99适用（2013主文印刷61／PDF69；AMD2仅追加C11，AMD1／3／4及TC1／2未改变本处字面量后缀对象）。四份已完成 `direct-render-mapping-final-results` 各20条报告逐一按文件、行、列取token核对，实际全部是标识符：`call`四处、`cell`十四处、`all`两处，不是数值literal，因而这20处是误报；不是对任意含小写l名字新增规则例外。

位置属于冻结Os_Backend.c的1413:28／57、1415:28／35；Os_HostServices.c的6:71、8:39、11:75、14:32、17:82、21:39；Linux Os_HostLinux.c的119:58、122:42、128:58／84、677:45、686:19；Os_LinuxNative.c的26:57、31:41、34:58、37:49。它们分别是宏形参／替换中的call、原子与futex接口的cell，以及多对象等待的all；四份分析源码在这些对象上与58fb105a字节一致。每份20个摘要位置都归属host-batch／probe，展开为40次程序报告，不把两种计数混用。

固定addon的misra_7_3对rawTokens使用`(0[xX])?[0-9a-fA-FpP.]+[Uu]*l+[Uu]*`整词正则，却没有先要求token是数值literal；call／cell／all的前缀恰好落在其字符集合内，故末尾l被误认成后缀。没有为这些名字改源码、添加suppression或改检查器。此结论只覆盖这20个报告，其他literal、SDK及采用代码仍按实际构造核查；原raw诊断、passed=false／error=null和221项not_assessed均保留。

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
