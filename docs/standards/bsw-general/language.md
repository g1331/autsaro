# BSW源码语言边界

本篇承接[BSW General](README.md)，记录所选C99交付源码的词法与初始化判断；逐条规范处置和原始分析状态仍分别核算。

## 注释与字面量的实际词法边界

R3.1（Required，主文印刷45／PDF53）在C99下禁止注释内的新注释起始序列；AMD4.39–40的例外1允许scheme://path形式的URI，例外2仍允许//出现在真正的//行注释中。FreeRTOS许可URI适用例外，Doxygen块内的C示例//不适用；@code／@verbatim不改变C词法。三个实际包含的采用头task.h／queue.h／timers.h分别有247／174／218处此类位置。受控0021补丁仅将对应23／15／10个Doxygen块转换为C99的///形式，保留说明、示例、标签、物理行数及全部非注释token，上游原件与许可URI不变。该正常修复不使用新偏离，原始checker未报告不作为满足依据。

R3.2为Required，本仓库C99配置适用（印刷46／PDF54；AMD2.27追加C11，AMD2.31删除说明性Note）。上述48块没有反斜杠换行，转换后的行注释也没有该构造。实际宏及Os_Hooks／commented_defines的续行仍位于预处理指令或完整块注释中，不能仅凭反斜杠换行判R3.2违反。新注释形式不改变此前的宏拼接语义。

R4.1（Required，印刷46–47／PDF54–55；AMD2.32未放宽终结条件）要求数值转义在另一个转义或当前literal结束处终结，不能仅靠后续非数字字符停止。已核冻结自有C／头、模板、四份Linux生成语料及双目标补丁内核中的实际数值转义均为字符literal末端的\0。Task名字虽经JSON序列化形成C字符串，正常入口先按grammar／rules将SHORT-NAME限定为ASCII identifier，不会把引号、反斜杠或控制字符送入该生成点。R4.2（Advisory，印刷47／PDF55）按替换发生在注释和条件处理前的顺序核对九种trigraph，已核原始文本未发现；生成注释没有通用自由文本转义层，当前实际调用方为固定说明、受约束名字或类型化数值，不扩张到任意用户应用或未来输入。

上述构造结论、源身份、生成入口与Windows语料补核集中PR #371；正常生成／编译／实际分析和独立复核按各自head记录。它们不改变原始passed/error/not_assessed及严格比较拒绝，也不构成全部221项或完整平台符合性。

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

R15.6为Required（主文印刷127–128／PDF135–136），只有else紧跟if的特例，不豁免函数式宏内部的选择体。自有FreeRTOSConfig.h的configASSERT在if之后原为单条Os_BackendAssert调用，现只为该调用增加花括号，保留外层do／while FALSE和参数的一次求值。旧／新实际宏的局部严格C99编译控制均观察成功调用一次、失败条件调用一次及原失败出口；除增加一对复合语句括号外，其余宏token相同。没有新增断言开关或生产状态。

R16.2、R16.5–R16.7均为Required（主文印刷131、134–136／PDF139、142–144，AMD2追加C11）。已读九个自有switch的标签均直接属于其switch复合语句，default在末尾且至少两个clause；控制类型分别为状态／发布／ARTI枚举、操作／服务ID及动作整数，没有Boolean控制。生成Ecu_BswMAction的模式是ComM_ModeType整数，同样有三种模式及末尾default；采用内核两个通知switch使用eNotifyAction枚举。统计格式化与协程模板的关闭条件另行保留，不能从未选择语法取得当前程序符合证明。这些是已列switch的布局及类型判断，不销项整个项目的可达性或所有新增配置。
