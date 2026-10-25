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
- [受控主机平台与 MISRA 边界](platform.md)：平台扩展、原生错误处理、三处有界保留及 C11 设施的适用性。
- [原生设施与资源生命周期](native-facilities.md)：上下文／布局、实际provider、展开初始化及线程所有权。

## 核算

`requirements SWS_OS` 只剩中断向量表段（`SWS_Os_00336`）与 OS 声明的 MemMap 包装（`SWS_Os_00815`）未处置，它们随 SchM／MemMap 责任链在 [#191](https://github.com/g1331/autsaro/issues/191) 处置；SchM 与 BSW 通用义务（`SWS_BSWGeneral --module Os`）同样由 #191 处置。ShutdownHook 返回后的死循环（`SWS_Os_00425`）由目标 OS 端口承担，记为差距 [#200](https://github.com/g1331/autsaro/issues/200)。SC1 的条款按能力本身处置，覆盖等级要求的最小容量（`SWS_Os_00241`），不缩减为参考配置实际使用的对象。

## 已知差距

- 引用的 OSEK 版本 ISO 17356-3 全文未取得，严格 ISO 条款由 [#57](https://github.com/g1331/autsaro/issues/57) 跟踪；固定 FreeRTOS 内核（[ADR 0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)）的上游代码诊断标记为 `adopted` 但不排除（[ADR 0008](../../adr/0008-misra-baseline-and-no-deviation.md)），由 [#58](https://github.com/g1331/autsaro/issues/58) 跟踪。
- Windows 实际交接运行只在 CI 的 Windows 套件中执行。

## SDK 声明与常量

当前SDK仍有独立的R8.2缺口：项目私有Os_HostLinux.h直接包含pthread.h，其pthread_create声明的start_routine内层void指针参数未命名；Os_PortRestart.c直接包含GCC13的unwind.h，_Unwind_Trace_Fn、_Unwind_Personality_Fn及_Unwind_Backtrace声明也保留无名参数。主文的函数指针示例明确要求命名；TC2.10只增加交叉引用，TC2.12只澄清R8.3不比较无名参数，没有取消R8.2义务。AMD2追加C11，AMD3.63补充引用，均不提供当前例外。

主文§6.7对ISO标准头内容及处理标准头时包含的文件另有边界；这里两条直接入口分别是POSIX和GNU接口，不能把整个pthread／unwind SDK归入ISO标准库豁免。删除显式包含而保留间接链、手写替代声明或修改本机SDK，均不能证明交付闭包已整改。保留对应SDK声明需新的有界项目取舍或供应方合规材料；当前只记录真实缺口，不套ADR0024、不写MR12，也不将该取舍等同MISRA合法偏离。此处列的是已核具体声明，不是整个SDK的完整违规清单。

R8.14同为Required（主文印刷75／PDF83；AMD2追加C11）。同一pthread.h的pthread_create、pthread_attr_getstack等声明使用__restrict限定参数；这是当前GNU C99下的restrict类型限定，不因拼写有下划线而消失。标准库memcpy的原文示例不提供整个POSIX头豁免。这是独立于R8.2的已确认声明缺口，既有两头文件命名取舍提案尚未覆盖此规则；不扩张原提案、不擅自移除SDK限定或填写批准记录，其他实际SDK构造仍须继续核查。

R7.1为Required（主文印刷59–60／PDF67–68；单个0例外，AMD2.27追加C11）。Os_HostLinux.c经sys/eventfd.h实际包含bits/eventfd.h；其EFD_SEMAPHORE=00000001、EFD_CLOEXEC=02000000、EFD_NONBLOCK=00004000使用八进制，后两项直接用于事件创建。前者虽未选择，枚举声明仍进入TU。stdint.h处理先结束，再包含这些枚举，不能据主文§6.7把整个POSIX头豁免。当前libc6-dev为2.39-0ubuntu8.9，两个已核pthread／eventfd头与该固定源码字节一致；精确身份、替代和[有界SDK取舍补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6397172894)留PR。当前未接受，不填批准表／MR12，不套ADR0023／0024，也不据此声称其余SDK已评估。

R8.10也是独立Required义务（主文印刷71／PDF79，AMD2追加C11）：当前pthread.h:624–630的__pthread_cleanup_routine和:1339–1343的pthread_equal均为生效的extern inline定义。前者由GNU与__EXCEPTIONS选择，后者由__USE_EXTERN_INLINES选择；正常GCC13.3 C99／O1／fexceptions宏收据确认两条条件。没有项目调用不消除已生效定义的义务，自有六个Interlocked辅助函数则均为static inline。固定libc6-dev 2.39-0ubuntu8.9头的身份、包含链及替代排除见[具体SDK补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6402928996)。仅关闭优化不能移除前者，改异常／inline语义会影响既有unwind及供应方接口；当前保留提案仍待决定，不扩ADR0024、不填批准表或MR12。

## futex 返回信息

D4.7为Required（主文印刷30／PDF38，AMD2.14追加C11）；错误信息应尽快有意义地检查，只有证明函数不可能报错才适用免检条件。Linux private futex使用对齐且有存储期保证的32位字，但对应6.8.0-147.147发行源码的get_futex_key／futex_wake仍提供负错误返回；源码、发行补丁及论证留[PR调查](https://github.com/g1331/autsaro/pull/371#issuecomment-6398922631)。没有据此声称当前内核已启用故障注入，也不凭正常参数替全部结果作免检证明。

Os_HostLinux.c的私有futex_wait在终止停车、恢复停车、SuspendThread及停止ACK的共同边界区分0、-EAGAIN、-EINTR、-ETIMEDOUT；这些状态沿原子状态重新读取与原有10ms片段继续。其他结果沿既有原生致命出口退出1。Os_LinuxFutexWake在共享原生边界检查负结果，非负唤醒数保持；原始类型化Os_LinuxFutexWait仍返回kernel状态，未把错误改成成功。

Os_StackLinux.c仅在已发布fault后进入永久停车；其等待遇到不可恢复错误时直接以E_OS_STACKFAULT终止，不能返回原故障帧，不假报controllers_exhausted。没有新增输出阻塞点、期限、重试或共享状态，公开签名与handle布局不变。测试的真实零waiter唤醒、值不匹配和超时与私有返回状态注入分开记账；各检查撤销必须由精确阶段拒绝。此处不覆盖其他系统调用、provider完整异步语义或全部D4.7。

## 自有配置与保留名字

R21.2为Required（主文印刷166–167／PDF174–175，名字集合见R21.1印刷165／PDF173；TC1更正标题及示例，AMD2追加C11）。旧Os_Schedule局部量／参数time和自有Os_ConfigType.time复用标准库标识符；没有因其不是时间函数调用而销项。局部名及配置成员均改为timing，Os_Time和生命周期消费者同步；生成模板的具名初始化、成员类型／顺序、计时与调度语义不变。旧具名成员访问需更新并重新生成／构建，不能用宏别名恢复保留名。这是自有接口整改，不扩大ADR0024或批准SDK保留。

## 实现与供应方边界

实现定义、编译／需求追溯、临时聚合、Windows外部名字、线程TLS分配、声明宏及SDK保留名字见[实现与供应方边界](implementation.md)。各条的适用条件、原始诊断与待决定状态分别保留。

## 对象地址与回调的转换

R11.4、R11.5均为Advisory，当前C99适用（主文印刷97–98／PDF105–106；TC1限定R11.4为对象指针，AMD2.60及语言表扩展不取消义务）。当前switch-termination的standard-ecu原摘要中六处R11.4、五处R11.5均为真实转换，不按uintptr_t可容纳指针或void回转合法判误报；保留具体用途和原始诊断，不构成整条符合或新偏离批准。

R11.4分别来自Os_ArtiAlarmBaseReturn记录完整地址／32位trace token、Os_HostThreadLayout导出映射范围、Os_StackRecordBuffer记录逻辑缓冲范围，以及采用tasks.c的栈顶对齐表达式两次转换及对齐断言。前几处用于地址身份、范围或输出，没有把截断的trace token恢复成指针解引用；采用栈顶仍须按实际StackType_t缓冲、深度与8字节对齐前提判断。当前x64的uintptr_t、size_t及64位地址记录由实际ABI／编译收据支持，不能外推任意整数、地址或Windows未取证供应方。

R11.5的HostBridge watch恢复实际传入的静态Ecu_HostWatch地址；native_task_start恢复native_starts中的同型元素；Linux actor_start恢复reserve_handle返回的真实句柄对象。这些回调的生产者和消费者类型相配，借用和退休仍按原生生命周期篇处理，不由一次强制转换证明无竞态。mmap返回的void地址转为mapping字节视图，失败先检查MAP_FAILED后再使用；其动态设施仍保留D4.12和供应方责任。

第五处是采用内核prvTaskStatusArrayWriter从void恢复TaskStatusArrayWriterContext_t，并非TCB转换。uxTaskGetSystemState先写入xContext的数组指针及uxIndex=0，再在挂起调度期间同步遍历／调用，返回后才离开该自动对象作用域；这处未发现保存context供以后调用。调用方容量、真实遍历和输出对象仍各负其责，不能从该转换推导所有外部callback安全。C99对象指针经void回转的语言依据为N1256 §6.3.2.3p1（印刷47／PDF59），对象—整数转换另属实现定义；两者均不消除上述Advisory违反。

MAP_FAILED另触发独立R11.6 Required义务（主文印刷99／PDF107；TC2.22澄清整数转void指针属实现定义，未取消规则）。当前Os_HostCreateActor实际失败比较展开固定sys/mman.h的((void *) -1)，不适用零常量例外，也不是R11.5的对象指针恢复。失败仍先识别该哨兵再释放槽位返回NULL；改看NULL／errno、搬动声明或改用同样哨兵的mmap64均非等价整改。针对这个精确SDK构造的有界保留仍待项目决定，不扩ADR0024、不填批准表／MR12；具体身份、替代与风险留PR #371。
