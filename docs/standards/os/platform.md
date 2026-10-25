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

Windows补丁`0018-windows-thread-setup-errors.patch`按D4.7／R17.7检查Task线程的affinity、boost、priority以及dispatcher的boost、affinity结果；失败沿现有断言或pdFAIL路径停止启动，不能进入Ready。`vPortCloseRunningThread`的priority调用同样检查，当前`INCLUDE_vTaskDelete=0`没有该删除调用路径，不声称它已被运行验证。依据[affinity](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadaffinitymask)、[boost](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriorityboost)与[priority](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriority)的零返回失败契约。Windows原生回归通过临时端口包装逐项拒绝五个启动调用，并与去掉0018的变异及正常启动对照；执行结果以实际Windows CI为准。

补丁`0019-windows-native-handoff-errors.patch`继续处置相同D4.7／R17.7义务：Task critical／yield、host中断发布、dispatcher及handler安装共同使用四个端口私有检查函数，验证mutex获取／释放、event发布／复位及无限yield等待；不改变锁或操作顺序。dispatcher的多对象WAIT_TIMEOUT继续等待，只有WAIT_OBJECT_0才能使用受保护状态；失败或abandoned结果停止OS，不能靠后续GetThreadContext成功冒充已取得mutex。现有控制线程承担停止，不新增状态／锁／分配。Windows回归保留真实正常／超时对照，逐类注入失败并与移除0019的变异比较；失败后同一actor继续调用原生端口时有独立终止哨兵，避免后续无关错误碰巧返回相同错误码。删除路径未启用的close／wait调用只作源码检查，不据此宣称运行验证。

Linux actor的mapping成员按实际用途保存为unsigned char字节区间：guard偏移、ASan fiber底端及一次性栈入口直接在同一映射内计算，不再从void指针反复转换。`Os_HostCurrentStackRange`仍将真实映射地址转为uintptr_t供栈界限／ARTI消费者比较，不隐藏为符合：原R11.6（Required，主文页99／PDF107，TC2.22修正整数到void指针的描述）已随存储类型消除；mmap返回到字节指针的R11.5与地址到整数的R11.4是保留的Advisory违反（页97–98／PDF105–106，TC1限定对象指针、AMD2.60扩充stdint说明）。mmap保证页对齐，偏移和长度来自同一次映射，uintptr_t不截短地址，转换结果只用于数值边界检查，不转换回指针解引用；原生stack／ARTI和errno回收消费者验证这些约束。依据§6.2.3记录建议性违反及理由，不申请新的Required豁免。

`Os_PortRestartLanding`是保留Task-loop帧内的汇编续行点，不是按C原型调用的函数。该符号现在只在本对象内可见；C侧通过真正遵循调用ABI的`Os_PortRestartAddress`取得native PC：Linux为uintptr_t，Windows为RtlUnwindEx要求的void指针表示。叶函数在原`.os_code`内用RIP相对LEA返回RAX，保留其他寄存器及配对call/return，不需要C函数指针转整数或void指针。依据R11.1（Required，主文页93–94／PDF101–102；AMD2.58只修订隐式转换例外说明），不使用该条的新例外；实际native扩展仍按上表R1.2／D4.2／D4.3评估。原生restart验证10000次同一物理栈重入、拒绝与外来异常，并分别检查全局C入口和本地续行点的代码段身份；错误PC变异必须被实际restart成功判据拒绝。该边界仍依赖受控x64 ABI，Windows执行及硬件SHSTK按实际结果记录。

Linux 备用信号栈原匿名 union 仅借 long double 成员取得对齐，实际消费者为 `Os_StackRegister → Os_LinuxAltStack`。R19.2（Advisory、C99，主文页154–155／PDF162–163；AMD2 只增加 C11 适用性及行为索引）直接约束 union 关键字，不因没有交叉成员读取而不适用。现改为仅含 long double 存储数组的 struct，每个 actor 仍取得原来的 65536 字节和原生对齐；C 代码不读取浮点值，不新增平台属性或转换。真实源的编译布局对照检查容量、对齐与槽数，降低对齐和缩短容量的反例分别拒绝；ARTI 消费者另检查发布地址的实际对齐。该整改不改变注册、信号及故障关闭路径。

`Os_ErrorParametersType` 同样改为各服务参数各占独立字段的 struct。服务包装按名初始化，`Os_ErrorResult` 复制参数和服务标识，getter 保持“匹配的 ErrorHook 内”这一前置条件；`Os_ErrorContextSave/Restore` 连同 phase 保存及恢复，C 消费者不依赖原来的重叠表示。所选 GCC x64 的对象从24变272字节，现有 ErrorHook 各访问开关、嵌套拒绝及 ISR 消费者检查实际字段；丢失参数和损坏服务标识的编译变异分别被现有完整协议判据拒绝，进程 exit0 不能替代该判据。

采用内核的 `QueueDefinition.u` 在共同受控补丁中也改为独立字段，保留 `xQueue`／`xSemaphore` 名称及全部原有分支。所选配置为静态分配开启、动态分配和 mutex 关闭，真实 FIFO／peek／满空／reset 路径消费 `xQueue`，不需要另一组字段的存储表示；未通过启用其他配置来替代所选配置的验证。该目标的 StaticQueue_t 从192变208字节，u从16变32字节；静态缓冲仍直接使用同一完整 QueueDefinition 类型。补丁摘要及双平台派生内核身份随源重新计算，原上游文件保持固定身份。两处布局均需重新生成交付物并整体重编；此逐处 R19.2 整改不外推系统头、未选变体或整个采用内核符合性。

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

## errno 的错误来源与诊断裁决

AMD1的R22.8–R22.10均为Required、适用C99（印刷13–16／PDF21–24；AMD2增加C11）。R22.8除列出的标准函数外，明确涵盖其他通过errno返回错误信息的函数；R22.9在返回值足以识别错误时允许不另与零比较。TC2.38纠正First Revision纸本的R22.9标题，不要求调用后把errno清零。项目不能把任意“可能改errno”的函数当成可靠错误来源，也不能把POSIX函数一律排除。

`eea9f25d`四份Linux完整工件各保留11个R22.10位置，四个相关源码与`2e274641`字节一致。逐处核对如下：`Os_HostSetEvent`之后的write、`read_event`的两处read判定、`wait_event`的poll、有限Sleep的nanosleep、`write_bytes`及`fatal_write`的write、HostBatch私有`read_byte`的read、`Os_LinuxMappedStack`的read，以及SIGPIPE路径的write_bytes／sigtimedwait。read／write／poll失败返回-1，nanosleep失败返回-1，sigtimedwait失败返回-1，均以errno报告错误；已有本机Linux man-pages6.7的各RETURN VALUE／ERRORS给出该契约，不能由Cppcheck函数名单替代。

这些errno读取均由失败返回值保护，相关平台调用前清零；`read_event`的EAGAIN检查前没有后续调用。`Os_LinuxMappedStack`只有count>0才调用行解析器，该分支由短路表达式避开errno读取；count<0路径没有这些解析调用。`write_bytes`在同一线程返回时保留最后write的errno，失败可为EPIPE或零进展；外层仅将真实EPIPE用于消费本次产生的SIGPIPE，零进展时errno仍为调用前的零。该helper自身就是将底层错误信息与0/1返回值一起交付的errno-setting-function，不是随意读取早前无关错误。完整异步上下文／provider语义仍按各自证据核查。

固定addon的`is_errno_setting_function`只有硬编码标准函数名，未包含上述平台函数／helper；`misra_22_10`又仅沿token记忆最近调用，并在右花括号清空，没有路径分析。因而这11个具体位置不能按其报告认定真实R22.10违反；源码、错误契约及失败分支支持限定误报裁决。名单还存在缺逗号导致相邻名字拼接的问题，不能据该checker缺报告确认R22.8／R22.9完整。保留原始报告和工具身份，不改addon、模型、基线或assessment；此结论不覆盖其他errno调用、Windows或全部三条规则。
