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
当前记录待独立复核，完成后才实施 MR12；不据本表声称源码符合或改写原始分析。

| 规则 | 精确范围 | 映射及消费／控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.9 | `runtime/bsw/src/Can.c` 中 `host_tx_completed_through` | VAR_CLEARED_UNSPECIFIED；Can_InternalHostCompleteTransmit 保留已观察的完成 token，Init 不回退。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/bsw/src/Com.c` 中 `publishing` | VAR_CLEARED_UNSPECIFIED；Com_Publish 的在途通知边界，Init 不清除。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/host/src/Det_Host.c` 中 `reporting_development` | VAR_CLEARED_8／TLS；Det_HostEnterDevelopmentReport 的每线程递归状态。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/host/src/Ecu_HostBusSM.c` 中 `bussm_publication` | VAR_CLEARED_UNSPECIFIED；Ecu_HostBusSM_Publish 的待发布／在途状态，BUSSM 区保护。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `resource_owner` | VAR_CLEARED_32；Os_BackendResource 在 port 锁下维护；ARTI view 也指向该数组。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `trace_length` | VAR_CLEARED_32；Os_TargetTrace 的调用方串行化长度，配合既有 trace 缓冲。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `rebase_entries` | VAR_CLEARED_64；rebase_orders 的有界重排暂存，元素类型和容量保持。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `saved_priorities` | VAR_CLEARED_32；Os_BackendResource 按 actor／资源保存 Task 优先级。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Backend.c` 中 `saved_interrupt_ceilings` | VAR_CLEARED_32；Os_BackendResource 按资源层级保存并恢复 ISR ceiling。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Error.c` 中 `error_active` | VAR_CLEARED_TLS_32；Os_ErrorResult 的 actor 递归保护；不处置参数地址的 R18.6。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Interrupt.c` 中 `states` | VAR_CLEARED_TLS_64；owner_state 返回逻辑调用方的 actor 自有深度对象。 | 维护者 原用户决定，ADR 0023 |
| MISRA C:2012 Rule 8.9 | `runtime/os/src/Os_Mailbox.c` 中 `last_ticket` | VAR_CLEARED_64；Os_TargetPostInput 的单生产者 ticket；测试配置另有只读 accessor。 | 维护者 原用户决定，ADR 0023 |
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
