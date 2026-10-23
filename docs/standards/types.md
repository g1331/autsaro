# 标准类型、平台类型与通信栈类型

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |

## C99 essential type 与实际常量

`Platform_Types.h` 的 `boolean`、TRUE和FALSE分别按固定R24-11 `SWS_Platform_00026`（页19）、`00054`／`00055`／`00056`（页29–30，含原表及续页）核对：本实现用_Bool，宏保留规定的true／false映射及ifndef。MISRA 2013主文附录D.6“Boolean constants”（页199／PDF207）明确给予stdbool.h的true／false essentially Boolean类型。因此，赋给boolean或与boolean比较的这些宏展开，不能仅因底层0／1为int就认定R10.3或R10.4真实违反；其他整数、原生BOOL参数和返回值须按各自类型另外核查。固定addon在嵌套宏的识别限制不改变原始报告，不以改写规定的宏、删除include或强制转换用户代码解决。

整数状态接口与布尔局部状态分别处理：`Os_ArtiServiceEnter`、`Os_BackendInterruptEnabled`、`Os_InterruptAllows`和中断服务许可保留既有sint32的0／1结果，并显式用条件表达式选择整数值；`Os_ErrorResult`的ISR判断和中断`change`的running局部量使用boolean及直接布尔条件。依据R10.3（主文页86–87／PDF94–95）、R10.4（页88／PDF96）；TC2的case标签勘误及AMD2语言扩充不改变这些C99赋值。没有用Boolean到整数的强制转换取代类型选择。

INVALID_TASK按OSEK OS 2.2.3 §13.2.4（页54）的TaskType常量义务显式保持TaskType；INVALID_ISR是本实现对`SWS_Os_00782`（页156，implementation_specific）的类型选择，同样保留ISRType。两个哨兵的值仍为255，ABI中的TaskType／ISRType均未改宽；C99消费者同时检查实际宏表达式宽度和值，防止整数提升后的UINT8_MAX被误当成有目标类型的常量。该选择不新增可接受ID或改变GetTaskID／GetISRID的无活动对象语义。

本轮沿实际调用闭包继续处置整数边界：OS的hook／mailbox／time／ARTI许可、原生wait／close／event结果及ECU owner／timing接口用条件表达式明确选择整数0／1；调用处在逻辑操作数或`?:`条件中显式比较。probe与HostBridge的私有require只消费比较结果，直接采用boolean。原接口宽度、错误码和truth值未变。此处是R10.1／10.3／10.4的类型整改，不是把Boolean强制转换为整数或改变状态含义。

Linux私有`Os_HostLinux.h`不再定义TRUE／FALSE，避免它在Platform_Types之前被包含时抢先占用标准宏；实际宏映射仍由Platform_Types按`SWS_Platform_00054`／`00055`／`00056`提供。原生BOOL是int，不是AUTOSAR boolean：CreateEvent／Mutex、WaitForMultipleObjects、alertable wait、DuplicateHandle及线程priority boost的整数参数使用0／1，Linux的BOOL结果／局部量和Windows采用端口的同类调用一致。`os_native_errno.c`在导入标准头之前编译私有声明，拒绝私有头再次定义标准布尔宏；随后继续执行真实设施及失败／回收场景。

`Os_TargetWaitTick` 的 waiting 和受控 POSIX 端口 `Os_PortInterruptSourceControl` 的 clear 均为 boolean，分别消费 TRUE／FALSE 的赋值与比较。最小对照在固定 Cppcheck 默认模型中没有 R10.3／R10.4 报告；加入真实分析输入的 `-Dtrue=1`／`-Dfalse=0` 后，同一源码出现两条 R10.3 和一条 R10.4。因此裁决依据是原文与实际宏展开，不能把默认模型的零报告外推为生产分析通过。

`Os_PortTaskThread` 给 role 选择 `'T'`、`'B'`、`'I'` 的嵌套条件表达式，两个层级的结果均为 essentially character。依据主文附录 D.6／D.7（页199–200／PDF207–208）及 TC1 的括号运算符勘误，外层 `:` 两侧的 essential type 相同；报告位置不是函数指针比较。固定 addon 把嵌套结果退回标准 int 的类别，最小字符对照仍报告 R10.4；把内层两个分支改为整数的真实类别冲突也报告。R10.4（Required，页87–88／PDF95–96）、AMD2.52 及 AMD3.31–32 没有要求把这些字符标签改成整数，也未使用字符算术或实／复浮点例外。

## 源码诊断的语法边界

R7.3（Required，主文页61–62／PDF69–70）禁止字面量后缀中的小写 l。所选 Linux 样本报告中的 cell／all 是变量或参数标识符，不是字面量；固定 addon 的正则未先要求数字 token，能同时匹配这些名字与真正的 `1l`。原始报告保留，不以重命名合法标识符代替裁决；此结论不覆盖其他字面量。

R12.3（Advisory，主文页106／PDF114）针对逗号运算符。所选 Linux 样本报告的 OS 字段／局部变量及采用内核局部变量均为声明中的 declarator 分隔符，不求值逗号表达式；固定 addon 对拆分声明标记直接报告。最小声明对照有报告，真实逗号运算符置于 return 表达式的对照却没有，故同时存在误报与漏报边界。两项规则在 AMD2 中增加 C11 适用性，不改变这些 C99 语法条件；完整位置、原始输出及对照收据见 PR #371。

`Ecu_ExecutionTransmit` 的 sink 为 `Can_TxSinkType` 函数指针，`(void)sink` 只显式丢弃此参数；实际发送沿 Can 的 flush → Ecu_ExecutionTransmit → Ecu_TargetEnqueueTransmit 进入受控队列。R11.1（Required，主文页93–94／PDF101–102）的例外2明确允许函数指针转 void，此处没有转成 void 指针、整数或其他函数指针类型；AMD2.58 只修订例外3的说明，不撤销例外2。固定 addon 对丢弃函数指针与改成不兼容函数指针的对照均报告；只对前者按既有原文例外裁决，不扩大为通用指针转换许可。

## C11 原子对象的适用边界

AMD4 新增的 R9.7（Mandatory，AMD4.6，印刷页10／PDF17）、R11.10（Required，AMD4.15，页12／PDF19）、R12.6（Required，AMD4.16，页13／PDF20）及 R21.25（Required，AMD4.32，页18–19／PDF25–26）均适用于 C11。已核对的双平台四样本及当前四个 Linux 样本，交付 C／头文件与采用内核均没有 `_Atomic`、标准 `atomic_*`／`memory_order*` 标识符或 `stdatomic.h` 入口；逐文件词法收据与冻结源码身份留在 PR #371，不外推系统头的预处理闭包。

R9.7 没有需要按该条初始化的 C11 原子对象；R11.10 没有施加 `_Atomic` 的 void 不完整类型；R12.6 没有被直接访问成员的原子结构体／联合体；R21.25 没有使用其规定内存序的 C11 标准库同步操作，故在上述源码范围分别不适用。这不是 Mandatory 豁免，也不把 GNU 原子内建视为 C11 标准库函数。Linux 实际使用的 `__atomic_*` 操作、发布顺序与 lock-free 前提仍按[平台扩展及并发边界](os/README.md#c99-平台扩展的使用边界)核查；这些操作存在的事实没有从收据中排除。

## 泛型选择的适用边界

AMD3.51 的 R23.1–R23.8 均只适用于 C11 泛型选择（AMD3 印刷页27–37／PDF35–45）。已核对的双平台四样本及当前四个 C99 生成样本，其交付 C／头文件和采用内核没有 `_Generic`，也没有 `tgmath.h` 泛型宏入口；这是实际构造的缺席依据，不仅是编译参数的语言标签。冻结源码身份及逐文件核对收据留在 PR #371。

R23.1（Advisory）的泛型宏展开位置与 R23.2（Required）的非宏控制表达式副作用条件均不存在，故在上述源码范围不适用。R23.3（Advisory）没有可检查的非 default association，R23.4（Required）没有 association 类型列表，分别不适用。R23.5（Advisory）没有基于指针类型的泛型选择，R23.6（Required）没有需要比较 essential／standard type 的控制表达式，分别不适用；未借用 R23.6 的整数常量例外。R23.7（Advisory）没有泛型宏实参的求值次数，R23.8（Required）没有 default association 的位置，分别不适用；未借用 R23.7 的常量结果例外。

这八项的适用性结论只针对上述源码；新增泛型选择或引入会展开它的库宏时重新核查。C99 中已有的 essential type、宏参数求值、指针转换和持久副作用仍按各自适用规则评估，原始 checker 的 `not_assessed` 目录状态不由这段人工结论改写。
