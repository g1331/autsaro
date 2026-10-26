# BSW源码语言边界

本篇承接[BSW General](README.md)，记录所选C99交付源码的词法与初始化判断；逐条规范处置和原始分析状态仍分别核算。

## 注释与字面量的实际词法边界

R3.1（Required，主文印刷45／PDF53）在C99下禁止注释内的新注释起始序列；AMD4.39–40的例外1允许scheme://path形式的URI，例外2仍允许//出现在真正的//行注释中。FreeRTOS许可URI适用例外，Doxygen块内的C示例//不适用；@code／@verbatim不改变C词法。三个实际包含的采用头task.h／queue.h／timers.h分别有247／174／218处此类位置。受控0021补丁仅将对应23／15／10个Doxygen块转换为C99的///形式，保留说明、示例、标签、物理行数及全部非注释token，上游原件与许可URI不变。该正常修复不使用新偏离，原始checker未报告不作为满足依据。

R3.2为Required，本仓库C99配置适用（印刷46／PDF54；AMD2.27追加C11，AMD2.31删除说明性Note）。上述48块没有反斜杠换行，转换后的行注释也没有该构造。实际宏及Os_Hooks／commented_defines的续行仍位于预处理指令或完整块注释中，不能仅凭反斜杠换行判R3.2违反。新注释形式不改变此前的宏拼接语义。

R4.1（Required，印刷46–47／PDF54–55；AMD2.32未放宽终结条件）要求数值转义在另一个转义或当前literal结束处终结，不能仅靠后续非数字字符停止。已核冻结自有C／头、模板、四份Linux生成语料及双目标补丁内核中的实际数值转义均为字符literal末端的\0。Task名字虽经JSON序列化形成C字符串，正常入口先按grammar／rules将SHORT-NAME限定为ASCII identifier，不会把引号、反斜杠或控制字符送入该生成点。R4.2（Advisory，印刷47／PDF55）按替换发生在注释和条件处理前的顺序核对九种trigraph，已核原始文本未发现；生成注释没有通用自由文本转义层，当前实际调用方为固定说明、受约束名字或类型化数值，不扩张到任意用户应用或未来输入。

上述构造结论、源身份、生成入口与Windows语料补核集中PR #371；正常生成／编译／实际分析和独立复核按各自head记录。它们不改变原始passed/error/not_assessed及严格比较拒绝，也不构成全部221项或完整平台符合性。

D4.4为Advisory（主文印刷27／PDF35），约束把实现注释掉；采用头的Doxygen示例不是停用实现。生成器的commented_defines仍输出生效define及说明，不能凭函数名判为注释代码。此判断不覆盖任意用户注释。

## 初始化层级与实际求值

R9.1为Mandatory，R9.2–R9.5、R13.1为Required，当前C99适用（主文印刷75–81、108／PDF83–89、116）。AMD4.47–48明确禁止designator／位置项重复初始化同一子对象；AMD4.56只将C11 atomic初始化交给R9.7，AMD4.58为R9.2补充R9.6引用。R9.1仍按每个元素／成员及实际成功／失败路径判断，不由聚合布局或零诊断销项。

四份switch-emitter工程中，Can载荷、CanIf错误槽、Os_Error来源与Os_Time等待句柄均给足数组元素；running、increments、向量表及内核计数数组使用允许的{0}。xYieldPendings的pdFALSE元素数对应实际单核。生成tasks[]内部.id是结构成员designator；实际数组designator仅为.priorities内部的[OS_INPUT_INTERRUPT]，容量由Os_IsrConfigType显式声明，单次指定、其余零初始化，未混入覆盖它的位置项。函数地址／OS_TASK_ENTRY名字及sizeof(storage)不执行回调或改变存储，等待列表读取普通HANDLE。此结论仅到上述初始化及宏闭包，不外推SDK、多核或任意用户应用；完整逐处依据留PR #371。

## 采用内核的两处值域报告

当前standard-ecu完成报告中的arrayIndexOutOfBounds指向prvCreateIdleTasks，把configIDLE_TASK_NAME的"IDLE"当成会读取下标14。实际循环从0递增，每次复制后立即检查NUL并break；该五字节literal在下标4停止，目的cIdleName容量为16，后续终结写仍在下标4。configIDLE_TASK_NAME未被目标覆盖；这条具体路径没有报告所称的越界，不借用上游coverity注释作为批准或证明。改名字宏、长度或循环顺序时应重核，不能按此结论豁免任意配置。

另一条knownConditionTrueFalse称xTaskIncrementTick的xConstTickCount==0恒假。所选configTICK_TYPE_WIDTH_IN_BITS为32，实际端口TickType_t是uint32_t；volatile xTickCount加一再转换到同一无符号类型，UINT32_MAX后的零是定义明确的回绕值。R14.3为Required（主文印刷119–120／PDF127–128，AMD2追加C11）；此比较在完整类型域中可真可假，不因短时运行没有触及回绕就成为不变条件。它不是R12.4约束的整数常量表达式。该源码判断保留原始报告，不声称已有真实provider长时运行或该规则全部符合。

## 条件宏与同名标识符

R5.5为Required、Single Translation Unit（主文印刷53／PDF61；AMD2.39及语言表扩展C11）。先前定义后又undef的宏仍须核对，不能仅凭最终宏清单缺席销项。当前switch-termination的standard-ecu完成摘要有47处：45个函数定义的同名宏仅位于mpu_wrappers.h的portUSING_MPU_WRAPPERS==1分支，实际值为0；另外两处pxCurrentTCB声明／定义的同名宏仅在tasks.c多核else中，实际configNUMBER_OF_CORES=1。沿46个不同名字遍历该工程全部C／头的定义入口，没有其他同名#define；既有实际TU宏收据亦无这些定义。这里证明的是定义从未生效，而不是被撤销后视为无冲突。

因此这47处可按所选程序的真实条件排除同名宏冲突；不据此修改原始诊断、假称整个R5.5通过，或扩大到MPU／多核配置。固定addon的misra_5_5从directives收集宏名后直接与variables／scopes比较；需要回到实际定义条件，不能以函数体左花括号的报告位置推断冲突名字，也不把初版候选检索误识别的if／while当成函数声明。

## sizeof的对象与数组形参

R12.5为Mandatory（AMD1印刷3／PDF11，AMD2追加C11），禁止直接对声明为数组形式的函数参数取sizeof；不是禁止sizeof参数所指的某个真实元素。R13.6按TC2.26已由Mandatory改为Required（主文印刷114–115／PDF122–123；TC2印刷9／PDF15），即使通常不求值，也不能在sizeof操作数中放潜在副作用；函数调用本身即在此限制内。

当前四份switch-reachability工程逐函数核对的数组形式参数包括Can四个data入口、Can_Host两个data入口、Ecu_ExecutionTransmit、Target两个帧入口、host-batch的read_line，以及采用内核prvSearchForNameWithinSingleList；component-pipeline／no-diagnostic另有应用PipelineTest_Copy的result。没有对这些数组形参本身取sizeof；Can复制用局部payload的实际八字节对象，read_line使用显式容量。sizeof(frames[0])取得单个帧类型的大小，不是把指针大小当数组容量；该判断也不单独证明count合法。

Os_LinuxNative的sizeof(char[条件?1:-1])使用offsetof／非VLA类型sizeof构成编译期布局约束；真实宏收据中offsetof为__builtin_offsetof，不是调用具有潜在副作用的函数。sizeof(destination->native.__fpregs_mem)也不读取其FP内容。词法候选中把==拆成两个等号、或把offsetof括号当调用，会误判这些位置，不能当作违反依据。采用stack_macros.h中含sizeof的宏使用局部固定数组，且当前stack-check选择已排除该分支；没有拿它当通用实参宏。此结论限定于已读源码、类型／宏闭包和当前C99配置，不扩大到SDK或新增VLA／用户应用。

0015移除xSTATIC_TIMER后，当前standard-ecu仍报告八处R2.4，不能据总数说修复未发生或诊断已清零。八处实际声明分别是三个自有匿名enum、FreeRTOS.h三个匿名struct、task.h的匿名struct／enum；Timer位置的CTU记录现以StaticTimer_t充当tag名，源码没有该tag。R2.4的对象是tag，不能把typedef名称当成它。StaticTimer_t的R2.3报告则另行保留：当前timers.h入口已由configUSE_TIMERS=0排除，不能沿旧未打补丁输入的无条件include，声称其静态timer原型正在使用该typedef。保留公开类型／配置变体的技术理由与实际用途仍须分别核对，不借匿名tag裁决一并销项。

## 选择体与clause布局

R14.2为Required（主文印刷118／PDF126；TC1印刷8／PDF14修正第一clause，AMD2.27／66／67追加C11及语言措辞）。for第二clause不得有持久副作用；循环体修改的额外控制对象须为essentially Boolean，比较结果是Boolean不使原状态码或响应字节成为Boolean。Os资源校验的两处status、CanIf领取的more、Dcm读取的response[0]、Linux等待的waiting／result和maps解析的result，因此展开为初始化、原条件while、原body及尾部增量。两个平台各有两处栈枚举条件读取原子／volatile计数，也同样改用while，保持查询次数和顺序。

这11处没有continue，提前return保持原位；不增加状态、锁、期限或重试。只读且body不修改的isrs／配置数量不混入此组；FreeRTOS默认configCONTROL_INFINITE_LOOP()为空，实际for(;;)适用例外。完整条件／body／增量token核对、实际消费者与原始零报告限制见[PR调查](https://github.com/g1331/autsaro/pull/371#issuecomment-6409180937)，不外推其他循环或SDK。

R15.6为Required（主文印刷127–128／PDF135–136），只有else紧跟if的特例，不豁免函数式宏内部的选择体。自有FreeRTOSConfig.h的configASSERT在if之后原为单条Os_BackendAssert调用，现只为该调用增加花括号，保留外层do／while FALSE和参数的一次求值。旧／新实际宏的局部严格C99编译控制均观察成功调用一次、失败条件调用一次及原失败出口；除增加一对复合语句括号外，其余宏token相同。没有新增断言开关或生产状态。

R16.2、R16.5–R16.7均为Required（主文印刷131、134–136／PDF139、142–144，AMD2追加C11）。已读九个自有switch的标签均直接属于其switch复合语句，default在末尾且至少两个clause；控制类型分别为状态／发布／ARTI枚举、操作／服务ID及动作整数，没有Boolean控制。生成Ecu_BswMAction的模式是ComM_ModeType整数，同样有三种模式及末尾default；采用内核两个通知switch使用eNotifyAction枚举。统计格式化与协程模板的关闭条件另行保留，不能从未选择语法取得当前程序符合证明。这些是已列switch的布局及类型判断，不销项整个项目的可达性或所有新增配置。

## 枚举值、数组声明及跳转

R8.11为Advisory、R8.12为Required（主文印刷72–73／PDF80–81，AMD2追加C11）。所选四份交付物有相同的33种枚举体：有的值全部显式给定，其余是首项零（显式或隐式）后递增；已核体内没有给隐式项重置编号、溢出int或重复值的构造。R8.12针对同一枚举列表的隐式值，不要求不同枚举共享一套全局数值。该结论不涵盖SDK枚举、未读应用或未来生成输入。

自有外部数组声明为Arti_Events、Os_ArtiTasks、Os_ArtiTaskStacks、Os_ArtiTaskContexts和Os_InterruptVectorTable，头中分别给出ARTI_EVENT_CAPACITY、OS_MAX_TASKS或OS_INTERRUPT_VECTOR_COUNT。定义使用同一容量来源；CDS的Rte_Inst对象是指针而非不完整数组，不能仅按名字混入此组。这里核对声明容量及对应定义，不由它推导所有索引或发布过程安全。

R2.6、R15.1为Advisory，R15.2／R15.3为Required（主文印刷44、122–123／PDF52、130–131；AMD2追加C11）。所核交付C／头及宏中没有C goto或具名语句标签；汇编字符串中的数字标签、switch的case／default不是这类标签，因此没有依靠早退或跨块goto绕过初始化的实际跳转。这里只按构造缺席裁决，不从中推出全部控制流或R9.1已满足。自有定义亦没有变参declarator或直接stdarg设施，但实际open调用的SDK变参义务已在OS篇单列，不能用自有标识符缺席批量关闭R17.1。

## 致命出口之后的返回

R2.1为Required（主文印刷39–41／PDF47–49，AMD2追加C11），其检查还包括函数出口，不限于switch clause。Backend的native_task_start和bootstrap曾在已声明noreturn的Os_BackendShutdown后保留return；control还在report_and_exit及Sleep(INFINITE)之后保留返回。这四句均没有可达的正常路径，现删除。report_and_exit全部路径最终调用ExitProcess，私有定义用既有GNU noreturn属性表达这一事实；没有新增公共签名、错误值、输出或退出期限。属性只是控制流契约，不能代替对函数体和调用者的核查。

Linux的无限Sleep沿现有pause循环，即使信号使pause返回也继续；Windows的[原生Sleep合同](https://learn.microsoft.com/windows/win32/api/synchapi/nf-synchapi-sleep)规定INFINITE不超时，并另以SleepEx提供alertable等待。删除后仍保持原有永久停放，不把函数末尾返回当作平台故障后备。源码属性反例分别恢复四句时必须拒绝；普通原生消费者与严格编译另核对实际运行，不把不可达代码删除称为产生了新的行为差异，也不据此关闭全部R2.1／R17.4。

## 包含与条件指令

R20.3／R20.8／R20.9／R20.13／R20.14均为Required，当前C99适用（主文印刷156–157、160–161、163–165／PDF164–165、168–169、171–173；AMD2追加C11）。TC2.35只更正R20.14示例的末行文件名。atomic-handoff-final-results四份实际补丁后源码中的包含均为完整尖括号或引号形式；每个文件的条件分支自行闭合，没有跨文件连接的if／endif。重复MemMap包含的条件体也在头内闭合，不能把反复包含误判成跨文件条件；R20.1／R20.5的真实义务仍分别保留。

标准ECU／用户应用、组件管线、无诊断三种语料组的if／elif合计分别为1290、1324、1299处。所有已核表达式的顶层为比较、逻辑或defined的布尔形式，没有以非零任意整数充当布尔结果。共同条件的155个数值标识符及160个别名闭包进一步核对到项目定义和实际编译器宏收据：定义为数值、单个别名或整体括起的表达式，没有宏替换把未括起的低优先级算符泄漏到外层。此处按实际展开边界判断，不由条件恰好选择了一个分支推定R20.8。

未定义的OS测试选项及portARMV8M_MINOR_VERSION先受defined保护；主文R20.9的逻辑与示例明确允许这种只在已定义时才求值的形式。旧16位tick选项位于已由32位配置排除的兼容路径，旧DelayUntil别名仅在对应ifdef内生效；AVR32与picolibc标识符分别受未选择的工具链／运行库入口控制。不能把所有未定义名字按预处理器补零后就写成满足R20.9，也不能把最终宏清单单独当作每个时刻的定义证明。

保留的上游兼容文件kernel/compat/include/StackMacros.h含非C99的#warning。kernel_path为避免大小写折叠碰撞将旧别名移至compat路径，真实tasks.c使用小写stack_macros.h，所选TU的include path不含compat，四语料也没有进入该旧别名的包含边。因此它不是这些实际TU中的R20.13违反，但该兼容文件本身不能称为C99指令已整改；将来启用它必须另作处置。上述检查处理了续行和注释，不把注释示例中的#字样当作指令；SDK指令与未取证配置不在这里销项。

标准库名字的索引同样需要对照原文：固定addon的C99名字表把mode列在stdio.h项下，但N1256 §7.19.5.3中的mode是fopen形参名，不是R21.1／R21.2引用的标准库文件作用域标识符。当前标准ECU两个实际程序的CTU声明候选中18个同名局部／参数位置不因此成为真实违反；这不是把此前floor参数、time／signal成员的整改反向撤销，也不是整个R21.2或SDK名字空间已满足。该索引只用于定位，实际作用域与标准声明才是依据。

R2.2同为Required（主文印刷41–42／PDF49–50；AMD4.37更新headline），但不能把所有冗余值都按赋值语句裁决。TC1的R2.2 Note（印刷2／PDF8）明确初始化不是赋值操作，不作为dead-code候选；native_task_state的SUSPENDED初始化因而不能仅凭正常case随后写入就判违反。TC2.3–4（印刷3／PDF9）另将结果确实使用的cast排除在dead-code之外；这不豁免其R10／R11类型与限定符义务。显式(void)也只处理有意不用值，不证明底层错误已按D4.7检查。此处区分具体操作种类，没有据此关闭其余可执行但无行为效果的调用、运算或赋值。

Counter的R7.5还可沿实际生成来源核查，不只依赖样本枚举。ScheduleContract的maximum／ticks_per_base／minimum_cycle／tick_ms均为u32；ecu.rs::configure_os先把tick_ms转成u64，再乘1000或1000000，最大结果4294967295000000仍小于UINT64_MAX。三项Counter配置值及换算因子均以Rust十进制整数格式化放入UINT64_C，没有后缀、符号或任意C片段。正常schedule验证又将maximum限定为不超过65535，其余三项为1；1ms下，完整UINT32_MAX ticks的纳秒乘积4294967295000000亦可表示。固定UINT32_C(0)实参满足32位范围，已修的scheduler资源1u不再调用小整数宏。此结论覆盖这些emitter来源的R7.5实参与该换算范围，不声称所有表达式或未来输入均已完成R10／R12／R13评估。

R11.9为Required（主文印刷101–102／PDF109–110，AMD2追加C11）。TC1明确增加aggregate／union的{0}初始化例外（印刷7／PDF13）；Os_HostInstallSignals与Os_StackPrepare的sigaction，以及actor_start的stack_t快照均适用，不能把其中首个指针成员当成新违反而改写SDK布局。实际指针赋值／比较中的整数零仍须来自NULL，(void *)0另按原文允许形式判断；该聚合例外不豁免R11.3／R11.5转换、限定符、资源错误或后续对象生命周期。这里仅裁决所列初始化器，没有从checker零报告推导整条R11.9满足。

R16.6为Required（主文印刷135／PDF143，AMD2追加C11），计算的是switch-clause而非标签数；多个case／default共用一段并只在末尾break，仍是一段。当前四份declaration-consistency分析副本各有13个物理switch：九个共同运行时、一个实际生成BswM动作、内核两个通知及一个排除的统计格式化switch。直接核对case体后，前十二个生效构造各有至少四个独立clause；统计分支仍由configUSE_STATS_FORMATTING_FUNCTIONS=0排除。协程crSTART模板没有进入所选TU，不能用其case数量替代已启用协程的证明。这里支持这些具体switch的R16.6判断；未知生成配置、宏外部调用和SDK不由标签计数批量销项。

R11.2／R11.7也不能仅按指针token判定（均Required；主文印刷95及99–100／PDF103及107–108，AMD2追加C11）。前者明确排除void指针，并由TC1要求按所指未限定类型判断；native上下文接缝中的void视图不因此成为不完整结构转换。FP复制涉及的struct _fpstate及ucontext在相应TU中已有完整定义，实际sizeof约束也依赖其完整性。后者的non-integer arithmetic还明确包括essentially Boolean／character／enum，不能缩成只查浮点。该接缝复制FP表示而非把指针转换成浮点数值；对象地址转uintptr_t仍按R11.4核查，void指针表示的转换另承担R11.6责任。这里没有替其他opaque对象或SDK转换作整体判定。

## 原生位掩码

信号、futex、原子状态及计数常量的[实际类型边界](../types/essential-types.md#原生位掩码)另记；不以固定checker的零报告替代判断。
