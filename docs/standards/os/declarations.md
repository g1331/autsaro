# OS声明与SDK接口

本篇承接[OS标准说明](README.md)，区分自有声明整改、按实际配置选择的内核接口，以及仍待决定的SDK义务。

## 自有接缝的声明一致性

R8.3为Required（主文印刷66–67／PDF74–75，AMD2追加C11）；具名参数及其类型名字／限定符须在同一函数的重复声明中一致。TC2.11–13（印刷5／PDF11）的无名参数例外不允许两组具名参数不同，主文width_t／height_t例子也不允许仅凭相同底层类型销项。Linux的pxPortInitialiseStack现沿portable.h使用pxTopOfStack／pxCode／pvParameters；vPortCloseRunningThread原型沿定义使用pending_yield。共同idle存储回调沿task.h使用ppxIdleTaskTCBBuffer／ppxIdleTaskStackBuffer／puxIdleTaskStackSize。参数顺序、类型与静态存储对象保持。

两个端口的Os_PortInterruptSourceEnabled／Control沿Os_Backend.h统一为sint32返回，interrupt／operation为uint32，clear仍为boolean；Os_IntegrationTimingAuthorized的ECU定义和独立OS夹具同样沿Os_IntegrationHooks.h使用sint32。所选目标上这些别名与原int／unsigned／uint32_t的实际基本类型相同，修改不改变ABI或返回值；它处置的是名字一致性。Windows与Linux完整有序补丁链另行核对，所有后续签名上下文与manifest身份同步，上游原件不改。当前Windows的新编译／执行仍须实际取证，不能从名字映射推出provider行为。

源码属性以真实声明头为预期，对恢复旧名字／旧类型拼写的副本作拒绝；实际编译和原生消费者独立核对。RTE的无名参数不因与具名定义并置而违反R8.3，R8.2及其SWS责任仍另核；函数指针声明也不要求与目标函数使用相同参数名。const TickType_t与TickType_t const只是相同限定的不同顺序，互斥平台分支、不同Task宏实例和关闭的协程实现不能混成同一生效实体。这里没有宣称全部对象、SDK或R8.3已闭合。


## 条件接口与声明owner

R8.5／R8.6均为Required、System范围（主文印刷69–70／PDF77–78；AMD2追加C11，AMD4.42补充R8.15交叉引用）。R8.5只约束非定义声明，不能把实现体的定义另计为重复声明。OS_TIME_TESTS分支曾在Os_Backend.h重复声明xInsideInterrupt；该头已通过FreeRTOS.h取得当前端口portmacro.h的声明，故删除私有重复行，不改对象类型或定义。两个目标的正常编译继续从各自唯一选中的端口头取得名字。

R8.6不能由“未调用”或链接成功直接销项。当前实际C99头展开和定义条件核对发现27处仍生效但没有所选定义的接口：portable.h的10个heap声明、task.h的12个接口和queue.h的5个接口。0025让声明与实际定义使用相同的配置条件：动态分配、mutex／recursive mutex、timer受限等待、Delay／Delete，以及scheduler-state组合条件。xQueueCreateMutexStatic保留既有static-allocation外层条件，再检查mutex条件；没有删掉已启用实现、补假定义或增加兼容别名。所有声明的类型、参数及非预处理C token保持，上游原件不改。

条件声明整改后，首次标准ECU分析还新增两条R2.3及两条R2.4：HeapRegion_t／HeapStats_t及其HeapRegion／xHeapStats tag不再有已选用途。引用闭包只剩同头中关闭的heap接口，故0025把这两个完整typedef／tag定义也纳入相同动态分配条件；启用分支的字段与类型名字不变。两条均为Advisory，不能仅用关闭功能解释仍然生效的无用途定义；对应2828条原始结果和严格配置拒绝仍按整改前身份保留，不反写为成功。

正常静态对象消费者仍使用真实TCB／Queue布局、Task栈和FIFO；同一构建命令导出的编译参数另检查七个已启用名字可见，以及27个关闭函数名字、两个heap类型各自触发undeclared诊断。只检查共享编译退出码不够：逐个恢复声明的副本还须被对应名字的观察拒绝。该观察证明当前头文件可见性，不声称关闭功能已实现，也不替代函数体及运行时生命周期的验证。Windows实际运行按新head证据另记。

Linux仍实际使用mmap、pthread和TLS，屏蔽heap声明不消除D4.12／R21.3／R22.1的责任。nm只用于交叉核对：有定义但未在符号表单列的九个私有函数不属于这27处；未定义符号表本身也不能证明所有provider或整个R8.6满足。SDK、外部运行库、其他声明和未选配置仍须按各自真实来源核查。

## SDK 声明与常量

当前SDK仍有独立的R8.2缺口：项目私有Os_HostLinux.h直接包含pthread.h，其pthread_create声明的start_routine内层void指针参数未命名；Os_PortRestart.c直接包含GCC13的unwind.h，_Unwind_Trace_Fn、_Unwind_Personality_Fn及_Unwind_Backtrace声明也保留无名参数。主文的函数指针示例明确要求命名；TC2.10只增加交叉引用，TC2.12只澄清R8.3不比较无名参数，没有取消R8.2义务。AMD2追加C11，AMD3.63补充引用，均不提供当前例外。

主文§6.7对ISO标准头内容及处理标准头时包含的文件另有边界；这里两条直接入口分别是POSIX和GNU接口，不能把整个pthread／unwind SDK归入ISO标准库豁免。删除显式包含而保留间接链、手写替代声明或修改本机SDK，均不能证明交付闭包已整改。保留对应SDK声明需新的有界项目取舍或供应方合规材料；当前只记录真实缺口，不套ADR0024、不写MR12，也不将该取舍等同MISRA合法偏离。此处列的是已核具体声明，不是整个SDK的完整违规清单。

R8.14同为Required（主文印刷75／PDF83；AMD2追加C11）。同一pthread.h的pthread_create、pthread_attr_getstack等声明使用__restrict限定参数；这是当前GNU C99下的restrict类型限定，不因拼写有下划线而消失。标准库memcpy的原文示例不提供整个POSIX头豁免。这是独立于R8.2的已确认声明缺口，既有两头文件命名取舍提案尚未覆盖此规则；不扩张原提案、不擅自移除SDK限定或填写批准记录，其他实际SDK构造仍须继续核查。

R7.1为Required（主文印刷59–60／PDF67–68；单个0例外，AMD2.27追加C11）。Os_HostLinux.c经sys/eventfd.h实际包含bits/eventfd.h；其EFD_SEMAPHORE=00000001、EFD_CLOEXEC=02000000、EFD_NONBLOCK=00004000使用八进制，后两项直接用于事件创建。前者虽未选择，枚举声明仍进入TU。stdint.h处理先结束，再包含这些枚举，不能据主文§6.7把整个POSIX头豁免。当前libc6-dev为2.39-0ubuntu8.9，两个已核pthread／eventfd头与该固定源码字节一致；精确身份、替代和[有界SDK取舍补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6397172894)留PR。当前未接受，不填批准表／MR12，不套ADR0023／0024，也不据此声称其余SDK已评估。

R8.10也是独立Required义务（主文印刷71／PDF79，AMD2追加C11）：当前pthread.h:624–630的__pthread_cleanup_routine和:1339–1343的pthread_equal均为生效的extern inline定义。前者由GNU与__EXCEPTIONS选择，后者由__USE_EXTERN_INLINES选择；正常GCC13.3 C99／O1／fexceptions宏收据确认两条条件。没有项目调用不消除已生效定义的义务，自有六个Interlocked辅助函数则均为static inline。固定libc6-dev 2.39-0ubuntu8.9头的身份、包含链及替代排除见[具体SDK补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6402928996)。仅关闭优化不能移除前者，改异常／inline语义会影响既有unwind及供应方接口；当前保留提案仍待决定，不扩ADR0024、不填批准表或MR12。

按四个真实TU原参数预处理后，还确认bits/poll2.h的poll、bits/unistd.h的read／getcwd／confstr／getgroups／ttyname_r／getlogin_r，以及bits/fcntl2.h的open为当前fortify选择下的外部inline，共八个新增R8.10位置；不是前述两个pthread函数提案的自动扩张。按实际包含栈，ISO标准头及处理它们时包含文件的实现内联仍守主文§6.7边界。原_FORTIFY_SOURCE=3、GNU inline及异常选择保持，不通过关闭硬化使这些定义消失。

R17.1同为Required、C99适用：AMD3.65–66禁止stdarg.h包含与其设施，TC2.8的变参声明示例直接标记不满足该规则。当前非ISO SDK有fcntl、open、makecontext、execle／execl／execlp、__open_alias的八个变参声明／定义位置（open在公开声明与fortify定义各一处）；生产只在Os_LinuxMappedStack中以固定O_RDONLY读取/proc/self/maps，不将其他声明记为执行。具体头字节、八个新增inline函数、变参位置、真实包含栈及替代排除见[合并SDK补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6403690524)。保留提案仍待决定，现成TU预处理不是新head完整分析／provider语义或合法偏离证明。

