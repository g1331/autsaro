# OS：受控主机平台与 MISRA 边界

本篇属于 [OS](README.md)，记录实际平台扩展、原生设施及相关 MISRA 适用性；项目保留、技术判断与未取证事项分别说明。

## C99 平台扩展的使用边界

以下是所选GCC x64端口对R1.2的设计理由，不把建议性违反写成已批准偏离。MISRA 2013主文R1.1（页37／PDF45）允许目标编译器支持的扩展；R1.2（Advisory，页38／PDF46）要求说明保留扩展的理由及有效使用的保证。编译器身份、参数与预定义宏随实际交付记录；不从C99语言标签推导平台ABI或扩展正确。

| 构造与实际消费者 | 必要性、风险与有效使用检查 |
| --- | --- |
| `__attribute__((section, aligned))`：`Os_MemMap.h`；`used`：`Os_PortRestart` personality／SEH handler；`noreturn`：关闭、断言与restart入口 | 段映射、向量表对齐及汇编引用的处理函数不能由普通C存储声明表达；错误属性可能导致丢失代码或错误优化。原生消费者检查ELF／PE的段、符号及向量表，真实故障／restart路径检查不返回行为；不把属性本身当作行为证明。 |
| `__thread`、`__alignof__`：`Os_Host.h`、`Os_MemMap.h`与Linux端口 | TLS保存每个原生actor的上下文，不能换成共享全局状态；对齐检查约束Interlocked存储。编译期宽度／对齐断言及原生并发、嵌套中断、栈身份测试检查实际消费者。TLS不产生同步保证。 |
| `__atomic_*`：Linux Interlocked、句柄、停车与故障发布 | C99没有标准原子接口；这里需要跨pthread及信号handler的发布顺序。保持明确的内存序、自然对齐及always-lock-free约束；构造失败归还同样使用原子写入。并发成功／失败对照及ThreadSanitizer回归验证实际路径，不能证明所有调度无竞争。 |
| `__asm__`：`OS_HOST_STACK_POINTER`、`Os_LinuxNative`、`Os_PortRestart` | 分别读取真实RSP、调用Linux UAPI、建立可被平台unwinder识别的Task入口；寄存器、clobber、栈对齐及unwind元数据依赖目标ABI。宏／函数隔离符合D4.3（Required，主文页26／PDF34）；用途与C接口说明按D4.2（Advisory，同页）记录于[运行时架构](../../architecture/runtime.md#激活重启e4-host-restart)。真实Task、栈／ARTI、restart及sanitizer消费者验证这些边界，SHSTK未启用时保留not_assessed。 |

这些理由不豁免D4.12的动态分配、R21.5的signal设施或R21.10的time设施，也不替代采用内核的其他逐项评估。

受控POSIX补丁的`next_interrupt`及Windows补丁0012的`prvNextOsInterrupt`参数原名均为`floor`，虽只用作unsigned优先级起点，仍复用标准数学函数名字。R21.2（Required、C99，主文印刷166–167／PDF174–175）引用R21.1的标准库名字集合；TC1修正标题／示例，未提供局部参数例外。两处均改为`minimum_priority`，调用、类型、选择顺序与ABI不变；此前Linux九项补丁组合的31文件对照仅有参数及其引用两个token变化，Windows按其完整补丁链另核，不把旧Linux执行结果扩张到Windows。该行属项目补丁责任，不能借adopted标签或固定checker未遍历参数声明而免除；它也不是数学调用，不改变D4.15的判断。

## 三处有界 Linux 平台保留

[ADR 0024](../../adr/0024-bounded-linux-platform-retention.md)记录[人类接受的项目决定](https://github.com/g1331/autsaro/pull/371#issuecomment-6392165780)，仅限当前 Linux C99／glibc-pthread x86-64 三处和其中列明的四 profile／两个程序及相同原生实现。它独立于 ADR 0023，既不是 MISRA 合法偏离或符合性批准，也不解除 `SWS_BSW_00115`。SDK 间接包含链及其他根路径继续可见，provider／ABI／flags／handler／合同变化重评。

三处均为 Required、适用 C99 的真实违反：主文 R21.5／R21.10（印刷168／170，PDF176／178）、AMD2.82及AMD3.68／69／70仍禁止相应包含。独立只读复核固定 `97da3702e605a4cb9b923d10964a60cfff256065`，逐处原文、调用闭包、替代及风险控制均无新增技术发现；没有把项目取舍当作技术结论，也未重跑作者测试。完整进入调查、替代分析及证据见[PR具体提案](https://github.com/g1331/autsaro/pull/371#issuecomment-6391387787)。

| 规则 | 精确位置 | 技术理由、风险与控制 | 项目决定来源 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 21.5 | `runtime/os/src/host/linux/Os_HostLinux.c` 中 `#include <signal.h>` | 保留异步上下文捕获、代次ACK及终止；mask与pending SIGPIPE须保持。SWS_Os_00001／OSEK抢占语义不指定此头；不能改为合作式暂停或全局忽略SIGPIPE。 | 维护者 人类，2026-10-25；ADR0024有界项目保留，非MISRA符合性批准 |
| MISRA C:2012 Rule 21.10 | `runtime/os/src/host/linux/Os_HostLinux.c` 中 `#include <time.h>` | 保留单调截止、futex间隔、EINTR余量及零时SIGPIPE等待；pthread间接链仍在。SWS_Os_00399／00286的软件Counter不能替代本机监督时钟。 | 维护者 人类，2026-10-25；ADR0024有界项目保留，非MISRA符合性批准 |
| MISRA C:2012 Rule 21.5 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `#include <signal.h>` | 保留真实guard地址／SP、备用栈、故障发布与actor停车。SWS_Os_00067／00068不指定此头；终止报告保留预分配缓冲及stderr回退，不推导输出完成期限。 | 维护者 人类，2026-10-25；ADR0024有界项目保留，非MISRA符合性批准 |

三处技术复核状态均为完成，范围如上。后续[有限Sleep及背压验证](https://github.com/g1331/autsaro/pull/371#issuecomment-6392720670)补上`Sleep(1250)`的受控非零余量序列及四种变异；`Sleep(0)`历史证据不扩大，项目重试协议与真实provider异步语义分开。SDK／最终链接provider与完整异步语义证据缺口仍由 #58 承接。注释／记录只精确关联保留理由，不改变原始诊断、`passed`／`error`／`not_assessed`／比较拒绝或其他221项评估状态。

## 原生设施的源码处置

见[原生设施与资源生命周期](native-facilities/README.md)：原生错误处理、上下文／布局、实际provider、初始化和所有权。

## 入口及跟踪宏的实际展开

AMD3.37新增R17.12（Advisory，印刷19–20／PDF27–28），适用当前C99，要求函数标识符用于调用或显式取地址。自有中断绑定／比较、线程及信号入口、锁初始化和HostBatch回调现用&；生成配置的通信／诊断／模式／Det回调及Task入口同样在值的位置显式取地址，NULL_PTR及数据数组引用不改。OS_TASK_ENTRY公共宏仍返回原函数designator，只在生成的调用方加&，不破坏已有显式取地址的用法。N1256 §6.3.2.1p4与§6.5.3.2p3支持这两种函数地址形式的相同类型与值；这里没有转换函数类型、修改签名、调用目标或ABI。

采用内核的状态遍历回调及两端口的中断绑定／比较也按相同实际角色处理；共同0022只改tasks.c和Windows端口，Linux选择其中tasks.c，其端口地址写法沿原Linux补丁修正并同步后续补丁上下文。完整组合保留原有换行字节，最终源差异只为已核函数值前的&；原件、类型与调用顺序保持。宏参数中的函数声明名字、函数指针对象本身和汇编字符串内的符号不是这些函数值，不机械添加&。生成观察使用正常完整入口的实际配置源逐槽检查，能区分取地址与调用／NULL；不把词法候选集合当作完整绑定分析或全部R17.12符合。

R20.7为Required（主文159–160／PDF167–168），约束宏参数展开后形成的表达式，不要求给声明符加括号；AMD2增加C11，未改变当前C99判断。四份comment-final-results各有五条原始报告：Os.h的ALARMCALLBACK参数形成函数声明／定义名字，受控POSIX portmacro.h的portTASK_FUNCTION_PROTO／portTASK_FUNCTION参数形成函数名和形参声明，都不是缺括号的表达式。其余两条来自picolibc-freertos.h的configINIT_TLS_BLOCK；实际FreeRTOS.h默认configUSE_PICOLIBC_TLS为0并排除唯一包含入口，不能把未进入当前程序的宏体当作已生效展开，也不外推启用picolibc时满足本条。四份工件的Os.h／Arti.h均与本次冻结源码逐字节相同；端口宏按实际补丁后定义核对，不用原始POSIX定义替代。

相同四份工件各有五条R20.10报告，均为真实Advisory违反（主文162／PDF170）：TASK、OS_TASK_ENTRY、ISR、OS_ISR_ENTRY以##连接既定入口前缀与配置标识符；ARTI_TRACE以#将context／class_name／instance／event分别保存为静态字符串。保留理由是入口定义与绑定使用同一名字约定，以及记录调用点的字面身份；改成普通函数不能自行取得这些预处理名字，手写每个绑定或重复字符串会增加两套身份来源。这是当前技术理由，不声称AUTOSAR强制选择##／#，也不提升为新的批准或整条规则符合。

上述每个入口拼接只形成一个合法C标识符，两个声明中的拼接互不依赖求值次序；ARTI的四次字符串化独立形成四个实参，两个payload实参各有括号且只出现一次。R20.11（Required，主文162／PDF170）禁止的#参数后紧邻##在这些替换列表中不存在；R20.12（Required，163／PDF171）所约束的同一参数混用也不存在：入口参数只用于拼接，四个身份参数只用于字符串化，两个payload只按表达式展开。此判断仅覆盖这些实际定义与调用角色，不借它判定其他宏全部满足。

## 采用 Queue 的前端游标

R18.1 为 Required、适用于 C99（主文印刷页143–144／PDF151–152），约束指针减法及 `-=` 的结果位于同一数组内，另允许末尾后一位；没有先越过头部再恢复的例外。AMD2.75／76（页16／PDF23）澄清非数组对象并删除旧理由句；TC2.33／34（页11／PDF17）将对象的字符表示视图按 `sizeof` 作为数组处理，两者都不允许负方向越界。

所选静态 Queue 的 `prvCopyDataToQueue` 前端／overwrite 路径原先先减 itemSize，再判断是否小于头部。合法单元素队列的读游标初值即为头部，因此会先形成对象之前的指针，即使随后不解引用并恢复也不满足 R18.1。共同补丁 `0020-bounded-queue-front.patch` 改为先判断是否等于头部：是则直接选择最后一个 item，否则才减 itemSize。零 item-size 的计数队列仍走原分支，复制、消息数、唤醒和 overwrite 的计数调整不变；未删减前端插入能力。

静态内核消费者检查单／双元素队列、重复前插／读取回绕、overwrite、peek、满空、reset 及零 item-size。测试副本在减法求值前按真实存储头部检查域；它拒绝恢复旧运算的有效输入，而不是等越界指针被解引用后才判断。删除正常移动、选错回绕目标的已编译变异另由实际队列顺序拒绝。该观察器只存在于测试副本，不给产品加入钩子，也不声称 sanitizer 能检测所有未解引用的越界指针。

两目标选择、资产及补丁摘要同步，原上游文件身份未变；派生内核由构建重新计算，交付须重新生成并重编，公开签名和 Queue 布局不变。此处保留按 item 步长运算的 R18.4 Advisory 违反（主文页147／PDF155）：理由是沿用现有环形存储表示，以先选回绕的局部修复保持 API、容量和状态模型；并非用该建议性理由豁免 R18.1，也不覆盖采用内核其他指针运算。

## C11 无返回说明符的适用边界

AMD3.36 新增的 R17.9（Mandatory）、R17.10（Required）及 R17.11（Advisory）均只适用于 C11，原文分别在 AMD3 印刷页17–18／PDF25–26、页18／PDF26、页19／PDF27；AMD3.74 的适用性表（页45／PDF53）与 AMD4 后续修订未改变这些条件。R17.9／R17.10 约束以 `_Noreturn` 声明的函数的返回路径／返回类型；R17.11 建议为不返回的函数使用该说明符，并明确排除 main。

固定双平台八份分析工程及 `5243b3ef` 四个 Linux 封存工程的交付 C／头文件（含采用内核）均没有 `_Noreturn` 或 `stdnoreturn.h`；逐文件身份及词法收据见 PR #371。因此，三条 C11 义务在上述 C99 源码范围分别不适用，不借用 main 例外，也不豁免 Mandatory 违反。保留的 GNU `__attribute__((noreturn))` 用于 Assert、Shutdown、原生 ExitProcess 及 Task-loop／restart；它不是 C11 说明符，仍按 R1.2 的扩展依据及各终止消费者的真实控制流核查。原始 POSIX port 的同类属性与所选补丁的移除也保留在来源记录中，不能把未打补丁的输入内核当成最终分析源码。该词法收据不证明系统头展开或链接库内部实现。

## C11 标准线程规则的适用边界

AMD4.34 的 R22.11–R22.20 均适用于 C11 标准线程设施（AMD4 印刷页20–30／PDF27–37）。已核对的双平台四样本及当前四个 C99 生成样本，其交付 C／头文件和采用内核没有 `threads.h`，也没有 `thrd_*`、`mtx_*`、`cnd_*`、`tss_*` 类型或函数。实际 OS 消费的是 pthread／Win32／FreeRTOS 接口；不因函数用途相近而把它们写成 C11 标准对象。冻结源码身份及逐文件核对收据留在 PR #371。

R22.11（Required）没有 C11 的 join／detach 对象；R22.12（Mandatory）没有受其标准函数访问限制的四类 C11 对象，R22.13（Required）没有须核对存储期的 `thrd_t`／`mtx_t`／`cnd_t`／`tss_t`，分别不适用。R22.14（Mandatory）没有由 `mtx_init`／`cnd_init` 初始化的对象，R22.15（Required）没有由 `mtx_destroy`／`cnd_destroy`／`tss_delete` 销毁的资源，分别不适用。

R22.16、R22.17、R22.18、R22.19（均为 Required）分别没有 C11 mutex 的显式同线程解锁、持锁等待、非递归重入及 condition-variable／mutex 关联对象，故在上述源码范围分别不适用。R22.20（Mandatory）没有由 `tss_create` 创建的线程特定存储标识，故不适用。这些是规定对象缺席的判断，不是对 Mandatory 违反的豁免；引入相应类型或接口时重新核查。

R21.26（Required，AMD4.33，印刷页19–20／PDF26–27）同样只适用于 C11，约束 `mtx_timedlock` 的 mutex 类型必须为 mtx_timed 或 mtx_timed 与 mtx_recursive 的组合。上述源码没有该标准函数调用，故不适用；POSIX／Win32 的有界等待不能因此免除其实际锁类型、超时与失败契约核查。

pthread／Win32 的获取／释放、持有者、失败清理与停止交错仍须按实际平台契约及 D4.1、D4.7、R22.1 等适用义务核查。C11 条款不适用不能代替这些核查，也不解除 D4.12 或采用代码责任；原始分析及其 `not_assessed` 状态保持。

AMD4.2的D5.1／D5.2／D5.3均为Required、仅适用C11（分别印刷2–4、4–5、5–6／PDF9–11、11–12、12–13），在当前C99下分别不适用。D5.3的正文限制创建阶段，不是“有固定上界就符合”；当前HostBatch的observer在每次COMMIT时创建，不能把它描写成全部线程只在程序启动时创建。这里按原文语言范围判定，不豁免C99扩展下的数据竞争、锁序、回调重入、资源获取／失败回滚和监督线程生命周期；这些仍由D4.1、D4.7、D4.12、D4.13及实际平台契约逐处核查。

## 参数副本与函数入口

R17.8为Advisory（主文印刷142／PDF150；AMD2追加C11），当前C99下适用。append_hex以独立cursor推进八个十六进制字符，offset保留入口值；共同0023在prvInitialiseNewTask、vTaskPrioritySet和prvTaskCheckFreeStackSpace中分别使用同型局部副本。优先级断言、特权位去除、上限裁剪、事件链表键和填充值扫描仍按原顺序消费同一数值；结构成员、函数参数名称／类型、调用方及ABI不变。Linux与Windows均选择该补丁的tasks.c，原始上游文件不改，补丁后两目标tasks.c字节相同。此修复不声称四个报告之外不存在参数修改。

## errno 的错误来源与诊断裁决

AMD1的R22.8–R22.10均为Required、适用C99（印刷13–16／PDF21–24；AMD2增加C11）。R22.8除列出的标准函数外，明确涵盖其他通过errno返回错误信息的函数；R22.9在返回值足以识别错误时允许不另与零比较。TC2.38纠正First Revision纸本的R22.9标题，不要求调用后把errno清零。项目不能把任意“可能改errno”的函数当成可靠错误来源，也不能把POSIX函数一律排除。

`eea9f25d`四份Linux完整工件各保留11个R22.10位置，四个相关源码与`2e274641`字节一致。逐处核对如下：`Os_HostSetEvent`之后的write、`read_event`的两处read判定、`wait_event`的poll、有限Sleep的nanosleep、`write_bytes`及`fatal_write`的write、HostBatch私有`read_byte`的read、`Os_LinuxMappedStack`的read，以及SIGPIPE路径的write_bytes／sigtimedwait。read／write／poll失败返回-1，nanosleep失败返回-1，sigtimedwait失败返回-1，均以errno报告错误；已有本机Linux man-pages6.7的各RETURN VALUE／ERRORS给出该契约，不能由Cppcheck函数名单替代。

这些errno读取均由失败返回值保护，相关平台调用前清零；`read_event`的EAGAIN检查前没有后续调用。`Os_LinuxMappedStack`只有count>0才调用行解析器，该分支由短路表达式避开errno读取；count<0路径没有这些解析调用。`write_bytes`在同一线程返回时保留最后write的errno，失败可为EPIPE或零进展；外层仅将真实EPIPE用于消费本次产生的SIGPIPE，零进展时errno仍为调用前的零。该helper自身就是将底层错误信息与0/1返回值一起交付的errno-setting-function，不是随意读取早前无关错误。完整异步上下文／provider语义仍按各自证据核查。

固定glibc2.39-0ubuntu8.9的signal/sigempty.c与signal/sigfillset.c在NULL参数路径设置EINVAL；这些非ISO接口不能因POSIX未要求错误就被当成绝不设置errno。Os_HostInstallSignals的两个集合调用、wait_mutex的集合初始化和Os_StackPrepare的guard集合初始化现各自先清零；原有HostWriteBytes／StackFatalExit已清零。所有调用都传入同一栈帧内有效sigset_t对象，实际provider对这些实参返回0，支持D4.7的免检理由，而R22.8仍按函数错误契约保留调用前清零。既有errno夹具新增真实sigemptyset／sigfillset包装，在成功返回后留下EAGAIN并记录输入errno与调用序号，沿正常初始化、mutex及回滚入口观察，不把清零等同于完整异步安全证明。

固定addon的`is_errno_setting_function`只有硬编码标准函数名，未包含上述平台函数／helper；`misra_22_10`又仅沿token记忆最近调用，并在右花括号清空，没有路径分析。因而这11个具体位置不能按其报告认定真实R22.10违反；源码、错误契约及失败分支支持限定误报裁决。名单还存在缺逗号导致相邻名字拼接的问题，不能据该checker缺报告确认R22.8／R22.9完整。保留原始报告和工具身份，不改addon、模型、基线或assessment；此结论不覆盖其他errno调用、Windows或全部三条规则。

R12.1为Advisory（主文印刷103–104／PDF111–112；AMD2.62更新表前三行），当前C99适用。比较与逻辑运算、条件式与算术运算按既有优先级显式分组；覆盖所读Linux／Windows位置，另核对未报告的故障注入条件及本机布局常量。只增加完整子表达式的括号，保留求值顺序、短路边界、读写次数和ABI；不把非括号token相同单独当作解析树等价证明。configASSERT的字符串化文本与源码行号可随格式变化，仍表达原断言条件；不新增错误种类、时限或资源动作。实际编译／消费者和新分析分开取证，旧诊断不改写。
