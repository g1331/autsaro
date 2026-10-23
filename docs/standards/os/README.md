# OS

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_OS` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR／OSEK 符合或认证。

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3（`os223.pdf`）。编号全集取自固定 SpecificationsARXML，`ECUC_Os_` 与 `ECUC_Arti_` 条目也在其中。

## 已选配置

单核 SC1，固定一个后端（FreeRTOS 内核加本项目的 OS 适配层）。ECU 只有一个所属任务、一个软件计数器（1 ms／tick）、一个名为 OSDEFAULTAPPMODE 的应用模式；周期触发来自 RELATIVE 自动启动的闹钟或重复、不同步的调度表，动作只有对所属任务的 SetEvent；资源只有可选的虚拟 RES_SCHEDULER。Error／Pre／Post／Startup／Shutdown hook、栈检测与 ARTI 始终启用。

OS-Application、IOC、多核、spinlock、时间／内存保护、ProtectionHook、外设区域与调度表同步不属于 SC1，在生成前于源对象位置拒绝，相应条款按配置不适用。运行时还实现了更一般的 SC1 能力（多任务与多计数器、ActivateTask／callback／IncrementCounter 闹钟动作、绝对起点、单次与多点调度表、配置资源与 LINKED 资源、ISR），并有原生测试；生成器只接受上述参考形态，其余取值同样在源位置拒绝，依赖它们的条款按配置不适用，每个参数都有 `Rejects:` 测试。放开生成器由 [#198](https://github.com/g1331/autsaro/issues/198) 跟踪。生成的 OS 只包含调度实际使用的对象，未使用的 OsAlarm、OsEvent、OsAppMode 或调度表同样拒绝，不会静默丢弃。

## 处置与证据

- [SC1 内核](kernel.md)：原生套件之外经审阅确认的内核条目、规范之间的取舍与依赖生成器所拒配置的条目。
- [ECUC 配置与生成](configuration.md)：全部 `ECUC_Os_` 条目、生成器接受的形态与拒绝方式、OS 生成章节。
- [SC1 以外的能力](extensions.md)：多核、OS-Application、保护、IOC、外设、调度表同步等由配置拒绝证明不适用的条目。
- [ARTI](arti.md)：ARTI 描述文件、hook 宏与 `ECUC_Arti_` 条目。

## 核算

`requirements SWS_OS` 只剩中断向量表段（`SWS_Os_00336`）与 OS 声明的 MemMap 包装（`SWS_Os_00815`）未处置，它们随 SchM／MemMap 责任链在 [#191](https://github.com/g1331/autsaro/issues/191) 处置；SchM 与 BSW 通用义务（`SWS_BSWGeneral --module Os`）同样由 #191 处置。ShutdownHook 返回后的死循环（`SWS_Os_00425`）由目标 OS 端口承担，记为差距 [#200](https://github.com/g1331/autsaro/issues/200)。SC1 的条款按能力本身处置，覆盖等级要求的最小容量（`SWS_Os_00241`），不缩减为参考配置实际使用的对象。

## 已知差距

- 引用的 OSEK 版本 ISO 17356-3 全文未取得，严格 ISO 条款由 [#57](https://github.com/g1331/autsaro/issues/57) 跟踪；固定 FreeRTOS 内核（[ADR 0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)）的上游代码诊断标记为 `adopted` 但不排除（[ADR 0008](../../adr/0008-misra-baseline-and-no-deviation.md)），由 [#58](https://github.com/g1331/autsaro/issues/58) 跟踪。
- Windows 实际交接运行只在 CI 的 Windows 套件中执行。

## C99 平台扩展的使用边界

以下是所选GCC x64端口对R1.2的设计理由，不把建议性违反写成已批准偏离。MISRA 2013主文R1.1（页37／PDF45）允许目标编译器支持的扩展；R1.2（Advisory，页38／PDF46）要求说明保留扩展的理由及有效使用的保证。编译器身份、参数与预定义宏随实际交付记录；不从C99语言标签推导平台ABI或扩展正确。

| 构造与实际消费者 | 必要性、风险与有效使用检查 |
| --- | --- |
| `__attribute__((section, aligned))`：`Os_MemMap.h`；`used`：`Os_PortRestart` personality／SEH handler；`noreturn`：关闭、断言与restart入口 | 段映射、向量表对齐及汇编引用的处理函数不能由普通C存储声明表达；错误属性可能导致丢失代码或错误优化。原生消费者检查ELF／PE的段、符号及向量表，真实故障／restart路径检查不返回行为；不把属性本身当作行为证明。 |
| `__thread`、`__alignof__`：`Os_Host.h`、`Os_MemMap.h`与Linux端口 | TLS保存每个原生actor的上下文，不能换成共享全局状态；对齐检查约束Interlocked存储。编译期宽度／对齐断言及原生并发、嵌套中断、栈身份测试检查实际消费者。TLS不产生同步保证。 |
| `__atomic_*`：Linux Interlocked、句柄、停车与故障发布 | C99没有标准原子接口；这里需要跨pthread及信号handler的发布顺序。保持明确的内存序、自然对齐及always-lock-free约束；构造失败归还同样使用原子写入。并发成功／失败对照及ThreadSanitizer回归验证实际路径，不能证明所有调度无竞争。 |
| `__asm__`：`OS_HOST_STACK_POINTER`、`Os_LinuxNative`、`Os_PortRestart` | 分别读取真实RSP、调用Linux UAPI、建立可被平台unwinder识别的Task入口；寄存器、clobber、栈对齐及unwind元数据依赖目标ABI。宏／函数隔离符合D4.3（Required，主文页26／PDF34）；用途与C接口说明按D4.2（Advisory，同页）记录于[运行时架构](../../architecture/runtime.md#激活重启e4-host-restart)。真实Task、栈／ARTI、restart及sanitizer消费者验证这些边界，SHSTK未启用时保留not_assessed。 |

这些理由不豁免D4.12的动态分配、R21.5的signal设施或R21.10的time设施，也不替代采用内核的其他逐项评估。

## 原生设施的源码处置

Windows补丁`0018-windows-thread-setup-errors.patch`按D4.7／R17.7检查Task线程的affinity、boost、priority以及dispatcher的boost、affinity结果；失败沿现有断言或pdFAIL路径停止启动，不能进入Ready。`vPortCloseRunningThread`的priority调用同样检查，当前`INCLUDE_vTaskDelete=0`没有该删除调用路径，不声称它已被运行验证。依据[affinity](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadaffinitymask)、[boost](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriorityboost)与[priority](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriority)的零返回失败契约。Windows原生回归通过临时端口包装逐项拒绝五个启动调用，并与去掉0018的变异及正常启动对照；执行结果以实际Windows CI为准。

补丁`0019-windows-native-handoff-errors.patch`继续处置相同D4.7／R17.7义务：Task critical／yield、host中断发布、dispatcher及handler安装共同使用四个端口私有检查函数，验证mutex获取／释放、event发布／复位及无限yield等待；不改变锁或操作顺序。dispatcher的多对象WAIT_TIMEOUT继续等待，只有WAIT_OBJECT_0才能使用受保护状态；失败或abandoned结果停止OS，不能靠后续GetThreadContext成功冒充已取得mutex。现有控制线程承担停止，不新增状态／锁／分配。Windows回归保留真实正常／超时对照，逐类注入失败并与移除0019的变异比较；失败后同一actor继续调用原生端口时有独立终止哨兵，避免后续无关错误碰巧返回相同错误码。删除路径未启用的close／wait调用只作源码检查，不据此宣称运行验证。

Linux actor的mapping成员按实际用途保存为unsigned char字节区间：guard偏移、ASan fiber底端及一次性栈入口直接在同一映射内计算，不再从void指针反复转换。`Os_HostCurrentStackRange`仍将真实映射地址转为uintptr_t供栈界限／ARTI消费者比较，不隐藏为符合：原R11.6（Required，主文页99／PDF107，TC2.22修正整数到void指针的描述）已随存储类型消除；mmap返回到字节指针的R11.5与地址到整数的R11.4是保留的Advisory违反（页97–98／PDF105–106，TC1限定对象指针、AMD2.60扩充stdint说明）。mmap保证页对齐，偏移和长度来自同一次映射，uintptr_t不截短地址，转换结果只用于数值边界检查，不转换回指针解引用；原生stack／ARTI和errno回收消费者验证这些约束。依据§6.2.3记录建议性违反及理由，不申请新的Required豁免。

`Os_PortRestartLanding`是保留Task-loop帧内的汇编续行点，不是按C原型调用的函数。该符号现在只在本对象内可见；C侧通过真正遵循调用ABI的`Os_PortRestartAddress`取得native PC：Linux为uintptr_t，Windows为RtlUnwindEx要求的void指针表示。叶函数在原`.os_code`内用RIP相对LEA返回RAX，保留其他寄存器及配对call/return，不需要C函数指针转整数或void指针。依据R11.1（Required，主文页93–94／PDF101–102；AMD2.58只修订隐式转换例外说明），不使用该条的新例外；实际native扩展仍按上表R1.2／D4.2／D4.3评估。原生restart验证10000次同一物理栈重入、拒绝与外来异常，并分别检查全局C入口和本地续行点的代码段身份；错误PC变异必须被实际restart成功判据拒绝。该边界仍依赖受控x64 ABI，Windows执行及硬件SHSTK按实际结果记录。

## C11 无返回说明符的适用边界

AMD3.36 新增的 R17.9（Mandatory）、R17.10（Required）及 R17.11（Advisory）均只适用于 C11，原文分别在 AMD3 印刷页17–18／PDF25–26、页18／PDF26、页19／PDF27；AMD3.74 的适用性表（页45／PDF53）与 AMD4 后续修订未改变这些条件。R17.9／R17.10 约束以 `_Noreturn` 声明的函数的返回路径／返回类型；R17.11 建议为不返回的函数使用该说明符，并明确排除 main。

固定双平台八份分析工程及 `5243b3ef` 四个 Linux 封存工程的交付 C／头文件（含采用内核）均没有 `_Noreturn` 或 `stdnoreturn.h`；逐文件身份及词法收据见 PR #371。因此，三条 C11 义务在上述 C99 源码范围分别不适用，不借用 main 例外，也不豁免 Mandatory 违反。保留的 GNU `__attribute__((noreturn))` 用于 Assert、Shutdown、原生 ExitProcess 及 Task-loop／restart；它不是 C11 说明符，仍按 R1.2 的扩展依据及各终止消费者的真实控制流核查。原始 POSIX port 的同类属性与所选补丁的移除也保留在来源记录中，不能把未打补丁的输入内核当成最终分析源码。该词法收据不证明系统头展开或链接库内部实现。

## C11 标准线程规则的适用边界

AMD4.34 的 R22.11–R22.20 均适用于 C11 标准线程设施（AMD4 印刷页20–30／PDF27–37）。已核对的双平台四样本及当前四个 C99 生成样本，其交付 C／头文件和采用内核没有 `threads.h`，也没有 `thrd_*`、`mtx_*`、`cnd_*`、`tss_*` 类型或函数。实际 OS 消费的是 pthread／Win32／FreeRTOS 接口；不因函数用途相近而把它们写成 C11 标准对象。冻结源码身份及逐文件核对收据留在 PR #371。

R22.11（Required）没有 C11 的 join／detach 对象；R22.12（Mandatory）没有受其标准函数访问限制的四类 C11 对象，R22.13（Required）没有须核对存储期的 `thrd_t`／`mtx_t`／`cnd_t`／`tss_t`，分别不适用。R22.14（Mandatory）没有由 `mtx_init`／`cnd_init` 初始化的对象，R22.15（Required）没有由 `mtx_destroy`／`cnd_destroy`／`tss_delete` 销毁的资源，分别不适用。

R22.16、R22.17、R22.18、R22.19（均为 Required）分别没有 C11 mutex 的显式同线程解锁、持锁等待、非递归重入及 condition-variable／mutex 关联对象，故在上述源码范围分别不适用。R22.20（Mandatory）没有由 `tss_create` 创建的线程特定存储标识，故不适用。这些是规定对象缺席的判断，不是对 Mandatory 违反的豁免；引入相应类型或接口时重新核查。

R21.26（Required，AMD4.33，印刷页19–20／PDF26–27）同样只适用于 C11，约束 `mtx_timedlock` 的 mutex 类型必须为 mtx_timed 或 mtx_timed 与 mtx_recursive 的组合。上述源码没有该标准函数调用，故不适用；POSIX／Win32 的有界等待不能因此免除其实际锁类型、超时与失败契约核查。

pthread／Win32 的获取／释放、持有者、失败清理与停止交错仍须按实际平台契约及 D4.1、D4.7、R22.1 等适用义务核查。C11 条款不适用不能代替这些核查，也不解除 D4.12 或采用代码责任；原始分析及其 `not_assessed` 状态保持。
