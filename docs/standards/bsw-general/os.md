# BSW General：Os

本篇属于 [BSW General](README.md)，按 OS 单独处置全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。OS 自身的条款见 [OS](../os/README.md)，其他模块的标记不替本模块处置。

OS 是 SC1、单核的 AUTOSAR OS，内核采用固定版本的 FreeRTOS 并打上生产补丁，主机端口在 Linux 与 Windows 的 x64 进程中运行。SWS_OS 用 `StatusType` 与 ErrorHook 报告错误，以 `StartOS`／`ShutdownOS` 管理生命周期，不定义开发错误、版本 API 或安全事件；R24-11 的 OS ECUC 也没有对应的开关。下文的“原文”指 SWS_OS 与 OS ECUC 定义。`Arti.h`／`Arti.c` 是 SWS_ARTI §7 规定的 ARTI 工具绑定文件，不属于 OS 模块文件。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 模块缩写与实现前缀为 Os；实现与头文件为 `Os.c`、`Os_*.c`、`Os.h`、`Os_*.h`，RTE 类型头为 `Rte_Os_Type.h`；主机后端为 `host/linux/Os_HostLinux.c` 等；没有仅大小写不同的文件名。`Os.h` 只导出 SWS_OS 的服务、类型与 hook 声明，集成接口在 `Os_Target.h`、`Os_Time.h`、`Os_Mailbox.h` 与 `Os_IntegrationHooks.h`。 已核对（源码）[Os]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | 各 `Os_*.c` 直接或经 `Os_Target.h`、私有头 `Os_Backend.h` 间接包含实现头 `Os.h`，`Os_MemMap.h` 只由 `Os.h` 在声明区段包含；内部声明在 `src/` 下的私有头；OS 不使用 SchM、不向 Dem 报告；调用的 hook 与集成钩子都在 OS 自己的头中声明。外部链接的变量都定义在实现源中并以 `Os_` 开头，只读的描述表带 `const`。 已核对（源码）[Os]：SWS_BSW_00004、00005、00007、00008、00010、00060、00129–00131 |
| 中断 | OS 依赖处理器，不是平台无关模块。Category 2 ISR 由 `ISR()` 声明，经 `Os_Interrupt.c` 与 `Os_Vector.c` 中的向量表分派（与其余实现分开），Category 1 由 `Os_TargetConfig.category1_isrs` 选择；从 ISR 到 Task 的转换由 OS 服务完成。主机端口没有硬件中断入口，也就没有 interrupt frame；分派只做有界的优先级与状态更新，应用 ISR 体由集成方提供。主机进程不访问硬件寄存器。 已核对（源码）[Os]：SWS_BSW_00016–00019、00021、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只有 pre-compile 配置：生成的 `Ecu_Config.c` 定义只读 `Ecu_OsConfig`，经 `Os_TargetPrepare(const Os_TargetConfig *)` 传入；`Os_Cfg.h` 与 `Os_Counter.h` 给出预编译开关与对象 ID；没有 link-time／post-build 配置文件。 已核对（生成样本）[Os]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184 |
| 实现头 | `Os.h` 自包含，经 `Os_Types.h` 引入 `Std_Types.h`，声明全部 API 服务；OS 没有主函数；除按段多次包含的 `Os_MemMap.h` 外，每个头都有唯一的 include guard。 已核对（源码）[Os]：SWS_BSW_00020、00024、00048、00110、00210、00249 |
| AUTOSAR 服务 | OS 提供 SERVICE-KIND 为 OPERATING-SYSTEM 的计数器服务：`Os.h` 包含 Application Types Header `Rte_Os_Type.h`；生成的 `Rte_OsService.c` 使用 RTE 接口并包含 `Rte_Os.h`，服务端口与接口由 `os_service.rs` 生成的服务组件描述；OS 不提供经 RTE 路由的回调。 已核对（源码与生成样本）[Os]：SWS_BSW_00023、00025、00040、00180 |
| RTE 与应用接口 | OS 源码没有 `Rte_` 调用，OS 头不包含 `Rte_Os.h` 或应用头。 已核对（源码）[Os]：SWS_BSW_00069、00218 |
| Hook | OS 没有 BSW 意义上的 callout；ErrorHook、StartupHook 等是 SWS_OS 规定的 hook 例程，声明在 `Os.h`／`Os_Hooks.h`，按 SWS_Os_00815 由 `OS_START_SEC_CODE`／`OS_STOP_SEC_CODE` 包裹（memory-mapping suite 验证）。 已核对（原文与源码）[Os]：SWS_BSW_00135、00136、00254 |
| 编码与命名 | 枚举值与宏除 SWS_OS／OSEK 规定的标准名（`TASK`、`SUSPENDED`、`OSServiceId_*` 等）外以 `OS_` 开头且全大写，错误值为标准的 `E_OS_*`；`boolean` 只赋值、返回 TRUE／FALSE 并与之比较；Linux 与 Windows 的栈探测是两种平台实现，不是重复代码。 已核对（源码）[Os]：SWS_BSW_00124、00125、00127、00142 |
| 类型、关键字与开关的残留 | 非主机代码的整数已改用 AUTOSAR 类型，`Os.c` 等模块源经 `Os_MemMap.h` 的 `OS_CODE`、`OS_THREAD_LOCAL` 与私有头 `Os_Vector.h` 的 `OS_VECTOR_TABLE_SECTION` 使用编译器关键字；仍有字符数据的 `char`、计数用的 `size_t`、预编译开关取 0／1 与按定义判断、配置结构缺少 `Type` 后缀，`Os_Stack.c`、主机后端与 ARTI 绑定 `Arti.c` 直接使用 `__thread`／`__asm__`，`CounterType` 等标准接口类型定义在 `Os_Types.h` 而不只在 RTE 类型头中，模块源也不直接包含 `Os_MemMap.h`，见 #203。 差距 #203[Os]：SWS_BSW_00006、00120、00121、00123、00029、00146、00147 |
| 指针参数 | 输入指针（如 `Os_TargetPrepare` 的配置）带 `const`，其余指针参数是输出或输入输出。 已核对（公开头）[Os]：SWS_BSW_00186 |
| C99 与调度依赖 | 公开头由独立的 C99 消费者编译（public-types suite）；OS 不调用任何主函数；SWS_BSW_00257 的表约束 OS 服务的使用者，OS 是这些服务的提供者。 已核对（构建与源码）[Os]：SWS_BSW_00133、00138、00234、00257 |
| 独占区 | OS 不声明也不进入 SchM 独占区，内核数据由端口自己的互斥协议保护。 已核对（源码）[Os]：SWS_BSW_00134 |
| 主函数 | OS 没有调度函数。 已核对（源码）[Os]：SWS_BSW_00037、00153、00154、00156 |
| 初始化与关闭 | OS 没有 `Os_Init`／`Os_DeInit`，生命周期按 SWS_OS：`Os_TargetPrepare` 只接受一次（再次调用返回 `E_OS_STATE`），`StartOS` 只启动一次，`ShutdownOS` 先置关闭状态再调用 ShutdownHook，此后服务不再执行。 已核对（原文与源码）[Os]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，由生成的主程序与 `Ecu_TargetPrepare` 承担调用 `StartOS`／`ShutdownOS` 的角色。 不适用（归属：EcuM／生命周期角色由生成启动器承担）[Os]：SWS_BSW_00150、00152 |
| 回调 | OS 不向其他模块提供回调函数；它调用的 hook 与集成钩子都返回 void 或简单状态，由集成方实现。 已核对（源码）[Os]：SWS_BSW_00172、00173 |
| 配置方式 | 用 ECUC 工具生成可读配置源，只接纳 VARIANT-PRE-COMPILE；参数名沿用 OS ECUC 定义（`OsStatus`、`OsUseGetServiceId` 等）。 已核对（配置模型）[Os]：SWS_BSW_00116、00126、00157、00183、00237 |
| 多分区 | 单核单分区：`TotalNumberOfCores` 为 1，`GetCoreID` 返回 `OS_CORE_ID_0`。 已核对（配置模型与源码）[Os]：SWS_BSW_00190–00192 |
| 错误分类 | OS 的错误全部是 SWS_OS 规定的 `StatusType` 值，没有实现专有错误。 已核对（原文与源码）[Os]：SWS_BSW_00073、00144、00209 |
| 开发错误 | SWS_OS 不定义开发错误，OS ECUC 没有 `OsDevErrorDetect`；OS 不包含 Det.h，参数检查结果经 `StatusType` 与 ErrorHook 报告（`OsStatus` 选择 STANDARD／EXTENDED）。 已核对（原文与源码）[Os]：SWS_BSW_00042、00045、00049、00050、00201、00203、00212、00243、00255 |
| 运行时错误 | SWS_OS 不定义经 Det 报告的运行时错误。 已核对（原文与源码）[Os]：SWS_BSW_00219、00222 |
| 生产错误 | SWS_OS 没有 production error、extended production error 或 DemInitMonitor 回调；没有 Dem 事件。 已核对（原文）[Os]：SWS_BSW_00046、00066、00143、00204–00208 |
| 安全事件 | SWS_OS 没有定义安全事件，OS ECUC 没有对应开关，也不调用 IdsM。 已核对（原文与 ECUC 容器）[Os]：SWS_BSW_00244、00250–00253、00258 |
| 版本 API | SWS_OS 不定义 `Os_GetVersionInfo`，OS ECUC 没有 `OsVersionInfoApi`；OS 不提供版本 API，与默认不可用一致。 已核对（原文与源码）[Os]：SWS_BSW_00051、00052、00064、00164、00236 |
| MetaData | OS 不引用全局 PDU。 不适用（归属：生产与转发 PDU 的 Com、PduR、CanIf 等模块；OS 不引用全局 PDU）[Os]：SWS_BSW_00239–00242 |
| Post-build variation | 主线只生成一个预编译配置集。 不适用（归属：配置工具只生成单一预编译配置集；变体选择由 EcuM 角色承担，主线没有 EcuM）[Os]：SWS_BSW_00226–00228 |
| 发布版本信息 | 差距 #59[Os]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA | 差距 #58[Os]：SWS_BSW_00115 |
| 完整 BSWMD／模块文档 | 交付的 BSWMD 只描述 OS 的两个计数器服务入口，没有其余服务、ISR 与独占区说明；没有随实现交付、写明处理器与工具链的模块文档。 差距 #159[Os]：SWS_BSW_00001、00002、00003、00038、00041、00054、00061 |
| BSWMD 模式组 | OS 的 BSWMD 没有 providedModeGroup。 已核对（BSWMD 生成）[Os]：SWS_BSW_00238 |
| Symbolic Name Value | 生成代码经 `Os_Cfg.h`／`Os_Counter.h` 使用 `OS_TASK_ID_*`、`OS_COUNTER_ID_*`，但集成头 `Ecu_TargetConfig.h` 中的任务 ID 以字面量写出。 差距 #160[Os]：SWS_BSW_00200 |
