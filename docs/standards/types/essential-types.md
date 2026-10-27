# Essential type与表达式

本篇承接[标准类型与平台类型](../types.md)，按固定C99目标记录具体表达式及检查边界。

## Boolean与character

`Platform_Types.h` 的 `boolean`、TRUE和FALSE分别按固定R24-11 `SWS_Platform_00026`（页19）、`00054`／`00055`／`00056`（页29–30，含原表及续页）核对：本实现用_Bool，宏保留规定的true／false映射及ifndef。MISRA 2013主文附录D.6“Boolean constants”（页199／PDF207）明确给予stdbool.h的true／false essentially Boolean类型。因此，赋给boolean或与boolean比较的这些宏展开，不能仅因底层0／1为int就认定R10.3或R10.4真实违反；其他整数、原生BOOL参数和返回值须按各自类型另外核查。固定addon在嵌套宏的识别限制不改变原始报告，不以改写规定的宏、删除include或强制转换用户代码解决。

整数状态接口与布尔局部状态分别处理：`Os_ArtiServiceEnter`、`Os_BackendInterruptEnabled`、`Os_InterruptAllows`和中断服务许可保留既有sint32的0／1结果，并显式用条件表达式选择整数值；`Os_ErrorResult`的ISR判断和中断`change`的running局部量使用boolean及直接布尔条件。依据R10.3（主文页86–87／PDF94–95）、R10.4（页88／PDF96）；TC2的case标签勘误及AMD2语言扩充不改变这些C99赋值。没有用Boolean到整数的强制转换取代类型选择。

INVALID_TASK按OSEK OS 2.2.3 §13.2.4（页54）的TaskType常量义务显式保持TaskType；INVALID_ISR是本实现对`SWS_Os_00782`（页156，implementation_specific）的类型选择，同样保留ISRType。两个哨兵的值仍为255，ABI中的TaskType／ISRType均未改宽；C99消费者同时检查实际宏表达式宽度和值，防止整数提升后的UINT8_MAX被误当成有目标类型的常量。该选择不新增可接受ID或改变GetTaskID／GetISRID的无活动对象语义。

本轮沿实际调用闭包继续处置整数边界：OS的hook／mailbox／time／ARTI许可、原生wait／close／event结果及ECU owner／timing接口用条件表达式明确选择整数0／1；调用处在逻辑操作数或`?:`条件中显式比较。probe与HostBridge的私有require只消费比较结果，直接采用boolean。原接口宽度、错误码和truth值未变。此处是R10.1／10.3／10.4的类型整改，不是把Boolean强制转换为整数或改变状态含义。

Linux私有`Os_HostLinux.h`不再定义TRUE／FALSE，避免它在Platform_Types之前被包含时抢先占用标准宏；实际宏映射仍由Platform_Types按`SWS_Platform_00054`／`00055`／`00056`提供。原生BOOL是int，不是AUTOSAR boolean：CreateEvent／Mutex、WaitForMultipleObjects、alertable wait、DuplicateHandle及线程priority boost的整数参数使用0／1，Linux的BOOL结果／局部量和Windows采用端口的同类调用一致。`os_native_errno.c`在导入标准头之前编译私有声明，拒绝私有头再次定义标准布尔宏；随后继续执行真实设施及失败／回收场景。

`Os_TargetWaitTick` 的 waiting 和受控 POSIX 端口 `Os_PortInterruptSourceControl` 的 clear 均为 boolean，分别消费 TRUE／FALSE 的赋值与比较。最小对照在固定 Cppcheck 默认模型中没有 R10.3／R10.4 报告；加入真实分析输入的 `-Dtrue=1`／`-Dfalse=0` 后，同一源码出现两条 R10.3 和一条 R10.4。因此裁决依据是原文与实际宏展开，不能把默认模型的零报告外推为生产分析通过。

`complete_activation` 的 `configASSERT` 参数用条件运算符在 TRUE 与 `eTaskGetState(...) == eSuspended` 之间选择；前者是附录 D.6 的 Boolean 常量，后者是附录 D.7 的 essentially Boolean 比较结果，R10.4 不要求把其中一支改成整数。内核查询仍在断言参数内，仅在 count 为零时执行。保留 enum 返回类型的最小断言对照，在默认模型下没有 R10.4，加入生产 true／false 定义后报告一条；把 TRUE 改为有符号整数参数的真实类别冲突在两种模型下均报告。当前源码的原始报告保留，不因上述裁决计作扫描通过。

`Os_PortTaskThread` 给 role 选择 `'T'`、`'B'`、`'I'` 的嵌套条件表达式，两个层级的结果均为 essentially character。依据主文附录 D.6／D.7（页199–200／PDF207–208）及 TC1 的括号运算符勘误，外层 `:` 两侧的 essential type 相同；报告位置不是函数指针比较。固定 addon 把嵌套结果退回标准 int 的类别，最小字符对照仍报告 R10.4；把内层两个分支改为整数的真实类别冲突也报告。R10.4（Required，页87–88／PDF95–96）、AMD2.52 及 AMD3.31–32 没有要求把这些字符标签改成整数，也未使用字符算术或实／复浮点例外。

已对`8bd09194`与`platform-retention-results`四份完成工件逐位置核对整组R10.3／R10.4报告：standard-ecu、user-application各129／84，component-pipeline为129／90，no-diagnostic为129／89，共863条摘要诊断、按程序归属为1722次。516条赋值的目标与源均为Boolean；347条操作数报告中343条是Boolean同类表达式，4条是上述character条件式。逐列区分同一行的计数器、enum和指针比较，没有仅凭TRUE／FALSE文本归类。真实声明包括CanTp的due／send、Com的receive／Mode和状态数组、Dcm工作／actor字段、RTE生命周期／收包状态、HostBusSM及OS局部量；Platform的boolean为_Bool，TRUE／FALSE来自stdbool的Boolean常量，逻辑／比较／同类条件结果按附录D.6／D.7保留Boolean。未使用signed→unsigned常量等例外放行。

固定addon未完整保留数值化Boolean宏及嵌套条件式的essential类型；上述位置据实际声明／表达式判为误报。独立只读复核核对18份直接交付源码与冻结Git字节、各自RTE生成来源、补丁端口的Boolean参数及完整seal／补丁来源；未重跑作者对照。完整位置、列与程序索引留在PR #371，原始四份2970／2970／3078／3044条诊断及221个not_assessed保持，不将863条的裁决扩大为R10.3／R10.4全部源码或配置符合。

当前unsigned-masks-final四份完成工件的R10.3／R10.4分别为138／88、138／88、138／94、138／93，共915条摘要、1826次程序归属。相对上述旧工件，新增的九个赋值与四个比较位置来自Backend、Time及生成Os_Arti的valid检查：声明均为boolean，赋值TRUE／FALSE、比较FALSE；Time的两个拒绝分支分别计位置。其余变化是私有名字／循环行和显式函数地址的源码键变化，角色条件式外层char cast不改变分支的character类型。作者逐差异连接实际声明，仍据D.6／D.7判这些位置为Boolean／character同类误报；旧863条的独立复核不冒充本次915条独立执行，原始报告及coverage拒绝均保留。

## 原生位掩码

R10.1（Required，主文印刷82–84／PDF90–92）的位运算要求unsigned操作数；有符号非负常量例外仅适用于移位右侧。当前OS把信号、映射、eventfd、poll、futex、unwind及原子状态的位操作放在无符号域，随后按原生接口转换；调用、标志值与ABI不变。清位常量先按uint32取反，再转LONG；TC1将~加入复合运算符，但整数常量表达式仍非复合表达式。TIME及ARTI原子常量去除Linux上产生64位long的L后缀，避免隐式压窄到32位LONG（R10.3／D.6）。固定addon漏查signed位操作及一元~，零报告不证明满足；Windows实际运行另取证。

当前standard-ecu完成结果另外保留三处清位常量cast的R10.8报告。固定misra_10_8没有先排除常量表达式；实际~(uint32)1u没有对象读取或调用，适用§8.10.3的常量边界，不因不同signed类别就成为非恒定复合表达式cast。原始2819条诊断及比较拒绝保留，不靠改写为裸-2常量或增加baseline消除报告。

## 字符算术与显式转换

R10.2为Required（主文印刷85／PDF93）；TC2.19把另一整数操作数的rank限定为不高于int。HostBatch的digit先将输入字符与十进制／十六进制字符常量分别转int再作差；Os_LinuxNative的map_address在字符范围检查后以char减char得到标准int，再转unsigned。Os_HostTextField／Hex从固定digits数组选取字符，不进行char加uint64算术。这些是实际表达式的不同类型路径，不把所有数字字符处理都当作同一例外。

R10.5为Advisory（主文印刷89–90／PDF97–98；TC1、AMD2.53–55及AMD3.33）。CanTp接收FF路径的(boolean)TRUE是Boolean到同类Boolean，并非整数／enum到Boolean的常量例外；TRUE的来源仍为stdbool。原生bitset的算术类型转换不提供指针、浮点或enum转换的通用允许条件。所查生产转换目标没有新增浮点求值；其他SDK定义及所有可能的配置不由这几个位置销项。

## 复合表达式与赋值结果

R10.6／R10.7均为Required（主文印刷90–92／PDF98–100；AMD2追加C11，AMD3.34澄清实／复浮点的共同运算）。前者覆盖参数和返回等赋值语义；后者限制窄复合表达式再与更宽操作数运算。TC1 §8.10.3把~加入复合运算符，但常量表达式仍不属于复合表达式，不能把所有含运算符的文本统一判断。

实际宽度安排分别为：HostBatch::number在乘法前已有uint64 result，并在减法／加法前把next转uint64；map_address的uintptr_t result在每轮乘16之前已有指针宽度，digit仅作为单值扩展；COM先把字节转uint32再移位打包；Dcm_ReadData的length及Dcm_ReadSize返回值均为Dcm_MsgLenType；Linux monotonic_ms在秒／纳秒换算前选uint64_t；Target复制长度先把元素数转size_t再乘sizeof。上述不是先作窄运算再靠结果cast扩大。futex的unsigned operation和有符号原生参数之间另有命名的中间值，保持系统调用ABI；其原始PRIVATE宏的signed位运算已整改。这里只确认这些实际计算与转换边界，不据若干样例关闭两条规则全部位置。

R13.4为Advisory（主文印刷112–113／PDF120–121），包括未求值表达式里的赋值结果。所读生产函数中的赋值用于独立语句或for初始化／迭代，不由条件、返回、索引或外层赋值消费结果；逻辑判断使用已经取得的变量。采用内核实际使用的listSET_LIST_ITEM_OWNER／VALUE、list插入／移除、queue锁及task选择宏分别检查完整展开，赋值仍为独立语句，不能仅靠宏调用末尾有分号判断。未选协程、TLS、多核及任意SDK宏的展开另负责任；词法候选检查和零诊断不等于整条规则通过。

## 重复条件与有用副作用

`48907b3b`的type-owner标准ECU有40条`duplicateCondition`，按物理位置为Ecu_RuntimeConfig.c的8条及Os_ArtiServices.c的32条。已逐位置核对前后两项条件：前者都是按值传入的events与固定mask比较，后者都是函数内const trace与0比较；两种guard的地址均不逃逸。RunCycle按计划顺序调用不同实体；ARTI分别捕获服务结果与发布完成事件，不能把相同条件下的不同调用判成重复调用而删除。

该报告指出可以合并相邻同条件块的源码冗余，不证明服务执行两次或运行时结果错误。R2.2对有用操作的判断仍须落到具体调用及其结果，不能只凭这一通用style诊断将这些调用视为死代码；本次也不将报告称为检查器误报。保留逐调用guard的当前表达与真实报告，不新增MISRA批准、baseline或suppression；以后合并时仍须保持实体顺序、无条件HostFlush位置以及ARTI的捕获→发布顺序。此判断仅覆盖上述40处，不代表其他重复条件或全部R2.2已核完。

## 内核无符号 tick 回绕的具体报告

complete-guards-final-results的standard-ecu在kernel/tasks.c:4836报告knownConditionTrueFalse，认为xConstTickCount等于零恒假。实际所选portmacro.h将TickType_t定义为uint32_t；tasks.c:4830的xTickCount与加数均在32位unsigned域，UINT32_MAX加一得到零，后续条件可真可假。主文R14.3（Required，印刷119／PDF127；AMD2追加C11）不允许不变控制条件，但这里不是不变条件，不借用无限循环或do/while例外；TC2.27–28修改的do/while例外也不适用。

现有sc1-timing正常入口另编译controlled_time.c的hardware-kernel-wrap消费者：StartupHook用既有测试seed将真实内核tick置于UINT32_MAX减二，随后通过生产tick处理与xTaskIncrementTick推进。Os_TargetWaitTick返回的completion.kernel_tick实际由xTaskGetTickCount读取；独立预期依次为4294967294、4294967295、0，与epoch65535／65536／65537和软件counter1／2／3一起精确比较。type-prefix-os-native本次已有成功收据，不再为此重跑；其host-timer/kernel/tasks.c与上述分析副本摘要同为4ccccb0d…49edf4d。

这是既有seed后的真实回绕观察，不是实际等待2的32次方个tick，也不是SC1软件Counter最大值15的回绕测试。生产状态推进没有规定小于UINT32_MAX的总tick上限；该位置不能据数学整数范围忽略C99无符号模运算。这里只裁决这一条核心报告，不修改原始passed／error／assessment或将其他恒定条件批量判误报；采用内核的两个延迟链break仍另负R15.4真实Advisory责任。

## SDK 整数字面量的实际展开

R7.1（Required，主文印刷59–60／PDF67–68）只对单个数字0给出八进制例外。complete-guards-final标准ECU的Native TU（序号48）在AUTSAR_VALUE探针之前，实际open("/proc/self/maps", O_RDONLY)展开为00；bits/fcntl-linux.h:43的定义确为O_RDONLY 00。因此这里不能按值为零销项，也不能将主文例外扩大为任意零写法。

同一TU的非ISO fcntl.h处理进入bits/fcntl2.h:48，__OPEN_NEEDS_MODE展开O_CREAT及__O_TMPFILE／__O_DIRECTORY，留下0100一次、020000000与0200000各两次。原生参数宏收据为_FORTIFY_SOURCE=3、__USE_FORTIFY_LEVEL=3；源码仍是正常O1／fexceptions／pthread参数，不能把fortify函数体当成未进入的源代码。既有四份带来源标记的sdk-inline-closure给出相同构造，HostLinux的三个eventfd枚举另见OS篇，其他两个样本TU没有八进制候选。这些具体位置保留真实Required缺口；不以改写数字、关闭fortify、换入口或间接include当作整改，也不扩张已接受的三个include范围。

R7.2另覆盖#if／#elif控制表达式及预处理后整数常量（主文印刷60–61／PDF68–69，AMD2.46及语言表）。按既有Linux LP64宽度区分十进制与十六／八进制的候选类型，四份.i的832／455／186／620个实际整数token均未发现缺U而具有unsigned类型的常量，所有数字token均能归入整数形式。这里没有把赋值目标是unsigned误当成literal类型，也没有把R7.1的八进制违反并为R7.2违反；该局部结论不包含已消去的条件控制表达式、其他TU或Windows，不能据此关闭整条R7.2。
