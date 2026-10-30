# 显式映射的单函数对象

本篇承接[声明用途](declarations.md#单函数对象与存储期)，逐对象记录 ADR 0023 冲突。
R8.9 为 Advisory／System，适用 C99（主文印刷71／PDF79；AMD2追加C11，
AMD4.43–46 将 defined 改为 declared）。静态或 TLS 存储期本身不要求文件可见性；
地址逃逸、Init 保留或测试方便都不是本条的规范例外。

## 共同约束与替代

固定 R24-11 MemoryMapping 的 SWS_MemMap_00023（PDF37）禁止函数体内包含映射头，
并明确要求需要特殊映射的函数静态变量移到文件作用域。下表对象均在现有模块的独立
VAR／CONST／TLS START/include…STOP/include 区内，与消费函数的 CODE 段区分。
SWS_MemMap_00003 的声明／定义各自包装同时适用。

块内自动对象改变存储期；把当前属性宏直接搬入消费函数会使用活动 CODE 属性；
在函数内重新 START/include 违反 00023。删除段、写死 GCC section 或复制原生属性
不保持现有可配置 MemMap 交付合同。因此保留文件作用域及真实 R8.9 违反。
这不是所有 file-static 状态的类别豁免，未映射或可正常收窄的对象仍须修复。

风险为同 TU 的意外访问；对象继续 static，原有原子／TLS／独占区／单生产者约束及
容量保持。既有 MemMap 配对／段属性、C99 合同和 OS 资源／ErrorHook／mailbox／stack、
Can／Com／host 消费者承担验证，实际执行及源码身份集中 PR。
段类别、属性来源、存储期、owner、消费者或目标改变时逐对象重评。

## 精确对象记录

授权来自[原用户决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)，
按 [ADR 0023](../../adr/0023-autosar-required-conflict-disposition.md)执行，不是实现者新增批准。
下表逐项列出当前映射及消费闭包；共同不可兼得依据均为上述 00023。
独立复核及各 profile／源码键身份集中 [PR #389](https://github.com/g1331/autsaro/pull/389)。
最初22个对象的映射必要性及声明关联已在 PR 的集中独立复核中确认；三处段类别与条件 setter 描述更正后实施 MR12，不据此声称源码符合或改写原始分析。

| 规则 | 精确范围 | 映射及消费／控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.9 | `runtime/bsw/src/Can.c` 中 `host_tx_completed_through` | VAR_CLEARED_UNSPECIFIED；Can_InternalHostCompleteTransmit 保留已观察的完成 token，Init 不回退。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/bsw/src/Com.c` 中 `publishing` | VAR_CLEARED_UNSPECIFIED；Com_Publish 的在途通知边界，Init 不清除。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/host/src/Det_Host.c` 中 `reporting_development` | VAR_CLEARED_8／TLS；Det_HostEnterDevelopmentReport 的每线程递归状态。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/host/src/Ecu_HostBusSM.c` 中 `bussm_publication` | VAR_CLEARED_UNSPECIFIED；Ecu_HostBusSM_Publish 的待发布／在途状态，BUSSM 区保护。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `resource_owner` | VAR_CLEARED_32；Os_BackendResource 在 port 锁下维护；ARTI view 也指向该数组。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `trace_length` | VAR_CLEARED_32；Os_TargetTrace 的调用方串行化长度，配合既有 trace 缓冲。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `rebase_entries` | VAR_CLEARED_PTR；rebase_orders 的有界重排暂存，元素类型和容量保持。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `saved_priorities` | VAR_CLEARED_64；Os_BackendResource 按 actor／资源保存 Task 优先级。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `saved_interrupt_ceilings` | VAR_CLEARED_32；Os_BackendResource 按资源层级保存并恢复 ISR ceiling。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Error.c` 中 `error_active` | VAR_CLEARED_TLS_32；Os_ErrorResult 的 actor 递归保护；不处置参数地址的 R18.6。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Interrupt.c` 中 `states` | VAR_CLEARED_TLS_32；owner_state 返回逻辑调用方的 actor 自有深度对象。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Mailbox.c` 中 `last_ticket` | VAR_CLEARED_64；Os_TargetPostInput 的单生产者 ticket；OS_EVENT_TESTS=STD_ON 另由 Os_TestInputLastTicket 条件 setter 写入；生产配置仍为单生产者。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Mailbox.c` 中 `write_index` | VAR_CLEARED_32；Os_TargetPostInput 的单生产者发布索引。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Mailbox.c` 中 `producer` | VAR_CLEARED_32；Os_BridgeContext 原子发布唯一生产者身份。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Mailbox.c` 中 `read_index` | VAR_CLEARED_32；Os_TargetTakeInput 的单消费者读取索引。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `altstacks` | VAR_CLEARED_UNSPECIFIED；Os_StackRegister 的 actor 备用信号栈，容量和对齐保持。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fault_claimed` | VAR_CLEARED_32；publish_fault 的原子 first-fault winner。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fatal_claimed` | VAR_CLEARED_32；Os_StackFatalExit 的原子 terminal-report winner。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fault_park` | VAR_CLEARED_32；fault_park_forever 的常零 futex 地址，不作 Init 写入。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fatal_prefix` | CONST_8；Os_StackFatalExit 的不可变报告前缀字节。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fatal_middle` | CONST_8；Os_StackFatalExit 的不可变报告分隔字节。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `fatal_suffix` | CONST_8；Os_StackFatalExit 的不可变报告后缀字节。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fault_claimed` | VAR_CLEARED_32；publish_fault 的原子 first-fault winner；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fatal_claimed` | VAR_CLEARED_32；Os_StackFatalExit 的原子 terminal-report winner；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fatal_text` | VAR_CLEARED_8；Os_StackFatalExit 的 winner 独占256字节格式缓冲；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fatal_prefix` | CONST_8；Os_StackFatalExit 的不可变报告前缀；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fatal_middle` | CONST_8；Os_StackFatalExit 的不可变异常字段分隔；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Stack.c` 中 `fatal_suffix` | CONST_8；Os_StackFatalExit 的不可变报告尾部；仅 Windows x64，保留独立于消费函数 CODE 的完整映射。 | 维护者 原用户决定，ADR 0023 |

## 历史源码与收据边界

以下保留原冻结身份下的整改收据，不把其中数量或旧函数消费者作为当前结论。

R8.9为Advisory、System（主文印刷71／PDF79；AMD2.27追加C11，AMD4.43–46／印刷32／PDF39将defined改为declared）。只有一个函数出现的标识符应优先收窄到块作用域；静态存储期不是保留文件作用域的充分理由，主文的调用计数示例本身就使用函数内static。地址被异步消费者借用时也须分别核对对象存储期与标识符可见范围，不能把二者混同。

55077f87四profile分析中的standard-ecu有46个报告对象，不按TU／程序重复计数。十个无独立MemMap数据段属性的私有对象已保留原类型／限定符、静态或TLS存储期并移入唯一消费函数：Target的native_owner／native_thread、protocol_write／read、next_output_ticket及三个output索引，Det_Host的reporting_development，以及HostBatch输入control的batch。Target仍以同一原子发布身份，队列索引及ticket不随调用重置；Det仍每个原生线程首次进入后保持递归边界；batch仍是H控制线程借用的进程期对象，执行／observer回收协议不变。除这些声明位置，三个文件的其他非注释C token不变。

另一个已收窄对象是Linux端口dispatcher_thread：在xPortStartScheduler原创建点定义const HANDLE，保留创建、注册等待和永久等待的原顺序。只是句柄值的局部副本，不把actor、映射或回调上下文的所有权改为自动存储期；后续有序补丁上下文和manifest一并同步。Windows不选择这两份Linux补丁。

上述十个对象的记录固定于当时的源码；后续逐处映射核查将Det的reporting_development恢复为文件作用域的显式TLS数据段，见[Det私有状态](../det.md#私有开发报告状态的映射)。该处R8.9真实Advisory保留重新成立，不把早期局部化收据改成当前状态，也不重写原工件的46项计数。

其余32个自有／生成对象目前保留真实Advisory违反及映射理由：Can和Com各一个发布状态、八个Ecu_RuntimeConfig配置对象、Rte的rte_allocated、HostBusSM的publication、Backend六个状态／入口记录、Error的TLS guard、Interrupt状态、Mailbox四个函数私有状态、HostLinux句柄池及StackLinux七个altstack／fault／终止文本对象。它们现有数据段属性与CODE区分开；直接把带相同OS_MEMMAP等宏的声明移到CODE函数内会取得不同的活动属性。固定SWS_MemMap_00023（R24-11，PDF37）禁止函数体内包含映射头，并明确说明特殊映射的函数静态变量须移到文件作用域。该义务限制沿现有START／STOP机制收窄这些对象；不能删除映射或在函数内嵌套请求消警报。其他机制仍需另证数据段身份、配置及编译器兼容性，不把GCC局部属性支持当作现成等价替代；逐处ADR0023／00115记录仍另核。

采用内核另三个对象保留明确用途：tasks.c说明xDelayedTaskList1／2的文件级静态符号供kernel-aware debugger定位；uxTopUsedPriority为OpenOCD提供优先级界限，具名C引用用于保留该对象。列表指针逃逸本身并不禁止块内static，调试符号兼容才是这里保留的理由；本仓库没有据此证明OpenOCD实际运行或赋予采用代码普遍豁免。上述11处整改、32处映射理由及三个调试对象是该46处的有界处置；新字节检查与其他未报告构造另核，原始诊断及not_assessed不改写。

## 对象生命周期与标准线程的人工评估

沿用[共同配置及冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)。
当前C99源码使用pthread/Win32/FreeRTOS及GNU原子，不使用C11线程/原子对象；
逐项不适用不豁免真实平台上的并发、锁顺序、所有权和失败清理。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| R17.9 | Mandatory | System／C11 | not_applicable | _Noreturn函数不得返回调用者（AMD3.36，17–18）。[无返回说明符边界](../os/platform.md#c11-无返回说明符的适用边界)及当前两组交付无_Noreturn/stdnoreturn.h；GNU noreturn及真实终止路径另核，不以语言不适用掩盖返回。 |
| R17.10 | Required | TU／C11 | not_applicable | _Noreturn函数返回类型必须void（AMD3.36，18）。同一有限源码没有该说明符，故无待裁决的返回类型；不是允许任意非void致命函数返回。 |
| R17.11 | Advisory | System／C11 | not_applicable | 不返回函数宜声明_Noreturn，main有明确例外（AMD3.36，19）。当前C99没有该C11设施，未借main例外覆盖普通函数；平台noreturn保持真实属性和消费者。 |
| R18.9 | Required | TU／C99 | satisfied | 临时生命周期数组不可衰变为指针；只允许立即下标且不可产生可修改左值（AMD3.39，21–22）。[聚合返回闭包](../os/implementation.md#非左值聚合与临时数组)中的Dcm_NextPublication、Arti_SaveAddressCapture、Os_ArtiCallerSave和selected_task结果无数组子对象；CONTEXT先保存至具名对象再复制，不借此批准ErrorHook普通自动对象地址外逃。 |
| R18.10 | Mandatory | TU／C99 | not_applicable | 不声明指向可变修改数组类型的对象/参数，普通一维数组形参重写不等于pointer-to-array（AMD4.31，17；并合成AMD4.25–30对R18.8的区分）。[数组声明边界](../os/configuration.md#c99-数组与回调类型)的OS容量、512栈、65536备用栈及sizeof/offsetof条件数组都是常量表达式，当前无该可变修改指针。 |
| R21.25 | Required | TU／C11 | not_applicable | 标准原子同步仅用seq_cst，所有_explicit/fence接口也须此内存序（AMD4.32，18–19）。[原子对象边界](../types.md#c11-原子对象的适用边界)没有标准atomic/memory_order用途；GNU__atomic的acquire/release不冒称满足本条，而按平台契约证明。 |
| R21.26 | Required | System／C11 | not_applicable | mtx_timedlock只作用于mtx_timed或mtx_timed与mtx_recursive组合的mutex（AMD4.33，19–20）。[标准线程边界](../os/platform.md#c11-标准线程规则的适用边界)无该函数/对象；原生有界等待的错误/类型语义另核。 |
| R22.11 | Required | System／C11 | not_applicable | 已join或detach的C11线程不能再次join/detach（AMD4.34，20–21）。当前无thrd_*对象；原生observer并发join/detach的0→2→1状态及失败回滚见[动态设施](../os/native-facilities/storage-interrupts.md)，不能由本条不适用跳过。 |
| R22.12 | Mandatory | System／C11 | not_applicable | C11线程/同步/TSS对象仅由指定标准库函数访问（AMD4.34，21–22）。当前无thrd_t/mtx_t/cnd_t/tss_t；pthread句柄或Win32 HANDLE不伪装成这些类型。 |
| R22.13 | Required | TU／C11 | not_applicable | thrd_t/mtx_t/cnd_t/tss_t不可有自动或线程存储期（AMD4.34，22–23）。同一源码没有四类C11对象；原生静态槽、TLS及callback借用仍守各自真实存储期。 |
| R22.14 | Mandatory | System／C11 | not_applicable | mutex/condition先经mtx_init/cnd_init初始化，mutex类型限原文四个组合（AMD4.34，23–24）。当前无C11初始化对象；不是对未初始化原生事件/锁的豁免。 |
| R22.15 | Required | System／C11 | not_applicable | 访问资源的线程全部终止后才销毁同步对象/TSS，或不销毁（AMD4.34，24–26）。当前无C11销毁入口；本机park/detach不算终止，按D4.13实际生命周期判断。 |
| R22.16 | Required | System／C11 | not_applicable | C11 mutex锁定后至线程退出的所有可达路径须由同一线程显式解锁（AMD4.34，26–27）。同一源码无标准mutex；实际独占区正常/失败出口沿[独占区协议](../../architecture/runtime.md#独占区协议)约束。 |
| R22.17 | Required | System／C11 | not_applicable | 只有先锁定mutex的线程可解锁或以其调用cnd_wait/cnd_timedwait（AMD4.34，27–28）。同一源码没有这些函数；POSIX owner身份及重入风险独立保留。 |
| R22.18 | Required | System／C11 | not_applicable | 非递归mutex不能由已持有它的线程再次锁定（AMD4.34，28–29）。同一源码无C11 mutex；FreeRTOS critical/模块锁的真实嵌套规则不因此消失。 |
| R22.19 | Required | System／C11 | not_applicable | 一个C11 condition variable至多关联一个mutex（AMD4.34，29）。同一源码没有cnd_*；pthread条件变量按原生对象身份和锁协议核查，不作C11对象满足声明。 |
| R22.20 | Mandatory | System／C11 | not_applicable | TSS指针先经tss_create创建才访问（AMD4.34，29–30）。同一源码没有tss_t/tss_*；glibc TLS分配及Windows线程存储仍按采用责任、D4.12和R22.1处理。 |
<!-- misra-manual-end -->
