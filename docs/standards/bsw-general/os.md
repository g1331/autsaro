# BSW General：Os

本篇属于 [BSW General](README.md)，按 OS 单独处置全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。OS 自身的条款见 [OS](../os/README.md)，其他模块的标记不替本模块处置。

OS 是 SC1、单核的 AUTOSAR OS，内核采用固定版本的 FreeRTOS 并打上生产补丁，主机端口在 Linux 与 Windows 的 x64 进程中运行。SWS_OS 用 `StatusType` 与 ErrorHook 报告错误，以 `StartOS`／`ShutdownOS` 管理生命周期，不定义开发错误、版本 API 或安全事件；R24-11 的 OS ECUC 也没有对应的开关。下文的“原文”指 SWS_OS 与 OS ECUC 定义。`Arti.h`／`Arti.c` 是 SWS_ARTI §7 规定的 ARTI 工具绑定文件，不属于 OS 模块文件。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 模块缩写与实现前缀为 Os；实现与头文件为 `Os.c`、`Os_*.c`、`Os.h`、`Os_*.h`，RTE 类型头为 `Rte_Os_Type.h`；主机后端为 `host/linux/Os_HostLinux.c` 等；没有仅大小写不同的文件名。`Os.h` 只导出 SWS_OS 的服务、类型与 hook 声明，集成接口在 `Os_Target.h`、`Os_Time.h`、`Os_Mailbox.h` 与 `Os_IntegrationHooks.h`。 已核对（源码）[Os]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | 各 `Os_*.c` 直接或经 `Os_Target.h`、私有头 `Os_Backend.h` 间接包含实现头 `Os.h`，`Os_MemMap.h` 目前由 `Os.h` 包裹声明，实现源的直接映射见下文未完成项；内部声明在 `src/` 下的私有头；OS 不使用 SchM、不向 Dem 报告；调用的 hook 与集成钩子都在 OS 自己的头中声明。外部链接的变量都定义在实现源中并以 `Os_` 开头，只读的描述表带 `const`。 已核对（源码）[Os]：SWS_BSW_00004、00005、00007、00008、00010、00060、00129–00131 |
| 中断 | OS 依赖处理器，不是平台无关模块。Category 2 ISR 由 `ISR()` 声明，经 `Os_Interrupt.c` 与 `Os_Vector.c` 中的向量表分派（与其余实现分开），Category 1 由 `Os_TargetConfigType.category1_isrs` 选择；从 ISR 到 Task 的转换由 OS 服务完成。主机端口没有硬件中断入口，也就没有 interrupt frame；分派只做有界的优先级与状态更新，应用 ISR 体由集成方提供。主机进程不访问硬件寄存器。 已核对（源码）[Os]：SWS_BSW_00016–00019、00021、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只有 pre-compile 配置：生成的 `Ecu_Config.c` 定义只读 `Ecu_OsConfig`，经 `Os_TargetPrepare(const Os_TargetConfigType *)` 传入；`Os_Cfg.h` 与 `Os_Counter.h` 给出预编译开关与对象 ID；没有 link-time／post-build 配置文件。 已核对（生成样本）[Os]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184 |
| 实现头 | `Os.h` 自包含，经 `Os_Types.h` 引入 `Std_Types.h`，声明全部 API 服务；OS 没有主函数；除按段多次包含的 `Os_MemMap.h` 外，每个头都有唯一的 include guard。 已核对（源码）[Os]：SWS_BSW_00020、00024、00048、00110、00210、00249 |
| AUTOSAR 服务 | OS 提供 SERVICE-KIND 为 OPERATING-SYSTEM 的计数器服务：`Os.h` 包含 Application Types Header `Rte_Os_Type.h`；生成的 `Rte_OsService.c` 使用 RTE 接口并包含 `Rte_Os.h`，服务端口与接口由 `os_service.rs` 生成的服务组件描述；OS 不提供经 RTE 路由的回调。 已核对（源码与生成样本）[Os]：SWS_BSW_00023、00025、00040、00180 |
| RTE 与应用接口 | OS 源码没有 `Rte_` 调用，OS 头不包含 `Rte_Os.h` 或应用头。 已核对（源码）[Os]：SWS_BSW_00069、00218 |
| Hook | OS 没有 BSW 意义上的 callout；ErrorHook、StartupHook 等是 SWS_OS 规定的 hook 例程，声明在 `Os.h`／`Os_Hooks.h`，按 SWS_Os_00815 由 `OS_START_SEC_CODE`／`OS_STOP_SEC_CODE` 包裹（memory-mapping suite 验证）。 已核对（原文与源码）[Os]：SWS_BSW_00135、00136、00254 |
| 编码与命名 | 枚举值与宏除 SWS_OS／OSEK 规定的标准名（`TASK`、`SUSPENDED`、`OSServiceId_*` 等）外以 `OS_` 开头且全大写，错误值为标准的 `E_OS_*`；`boolean` 只赋值、返回 TRUE／FALSE 并与之比较；Linux 与 Windows 的栈探测是两种平台实现，不是重复代码。 已核对（源码）[Os]：SWS_BSW_00124、00125、00127、00142 |
| 模块数值与字符 | 配置八个计数、对应遍历及 ScheduleTable 的 ARTI 索引使用 uint32，容量检查在 Os_TargetPrepare／Os_TimeValidate／Os_ScheduleValidate 先于遍历；CounterType 仍是完整 uint32 ID，TickType 仍是 uint64 tick。任务名字节串与 trace 用 uint8；FreeRTOS 名称及 hosted printf 的 char 转换仅在调用边界，断言文件参数为不解引用的 const void *。MemorySizeType 使用 x64 uint64，ARTI 地址保留 uint64 全宽；原生 API 的 LONG、uintptr_t、size_t 与 char 仍按平台／工具签名使用，不据此作 MISRA 豁免。已核对（Os_Target.h、Os_Time.h、Os_Types.h、Os_Backend.c、Os_Arti.c；public-types 的独立宽度约束及 lifecycle 的逐计数拒绝向量）[Os]：SWS_BSW_00120 |
| 模块自定义类型 | 配置、快照、模块内部类型与直接 host 适配类型使用 Os_<Tn>Type；SWS_OS／OSEK 的 TaskType、CounterType、TickType、StatusType 等保留标准名。两个生产补丁的原生栈类型引用同步，原始 FreeRTOS 身份不变。已核对（公开／私有 OS 头、实际 Ecu_Config.c 生成模板与内核直接消费者）[Os]：SWS_BSW_00146 |
| 预编译选择 | OS_STATUS_EXTENDED、OS_USE_GET_SERVICE_ID、OS_USE_PARAMETER_ACCESS 与测试功能开关使用 STD_ON／STD_OFF，并与值比较；非法配置检查也用符号值。平台选择、头 guard、MemMap 段协议及 callable observer 按各自职责保留。已核对（Os_Cfg.h、模块各编译分支、status-modes 的独立组合及关闭测试路径）[Os]：SWS_BSW_00029、00123 |
| 平台关键字映射 | OS_CODE、OS_MEMMAP 与 OS_THREAD_LOCAL 保留实际段／TLS 作用，向量属性归入 Os_MemMap.h 的数据段映射。native TLS 与 x64 栈指针汇编集中于 Os_Host.h，模块不直接写专有关键字；static inline 是 C99。已核对（Os_MemMap.h、Os_Host.h 及其实际消费者）[Os]：SWS_BSW_00121 |
| 实现源内存映射 | OS 的公开／私有声明、实现源、原生适配、ARTI 与生成配置按 CODE、CONST、CLEARED／INIT、TLS／VECTOR 实际类别成对映射，配对内不含其他预处理。Os_MemMap.h 显式施加段属性；静态状态保持单一生命周期，向量表保留 .os_vec，描述引用同一映射头。已核对（Os_MemMap.h、模块声明／定义、Ecu_Config.c 真实 emitter；memory-mapping 的源码分类反例、链接段与原生行为检查）[Os]：SWS_BSW_00006 |
| 服务共享类型 | CounterType 与 TimeInMicrosecondsType 由 RTE 的 Rte_Type.h 唯一定义，Rte_Os_Type.h 包含该头满足 SWS_Os_00786／00794 的 Available via；Os_Types.h 引用服务类型，普通 OS API 类型仍由 OS 定义。生成器从同一受信 Rte_Type.h 扩充应用类型，Os_Service.arxml 的 native 声明与 TYPE-EMITTER 指向真实所有者。已核对（contracts::contract_headers、os_service::files、独立 public-types 的两种包含顺序及生成服务消费者）[Os]：SWS_BSW_00147 |
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
| 交付描述 Os 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，五个 OS hooks 由 integration/src/Ecu_OsHooks.c／Ecu_Target.c 提供，Os expectedEntry 与集成层 implementedEntry 引用同一入口；constr_4016 限定的实体类型配对由描述测试核验；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[Os]：SWS_BSW_00001 |
| 交付描述 Os 00002 | 已核对（module_documentation::module 交付 modules/Os/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[Os]：SWS_BSW_00002 |
| 交付描述 Os 00003 | 已核对（随包模块手册指向实际 target.json 的目标、编译器版本／选项与库；控制目标为 x64 主机，不虚构 MCU 厂商、系列、型号或掩膜版本）[Os]：SWS_BSW_00003 |
| 交付描述 Os 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 Os 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[Os]：SWS_BSW_00038 |
| 交付描述 Os 00041 | 已核对（Os.h 与 Os_Hooks.h 的已交付入口和源清单已描述；当前选定 OS 模块没有硬件 ISR，主机中断分发及应用 ISR 的调用契约在公开头与主机资源说明中区分）[Os]：SWS_BSW_00041 |
| 交付描述 Os 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[Os]：SWS_BSW_00054 |
| 交付描述 Os 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[Os]：SWS_BSW_00061 |
| BSWMD 模式组 | OS 的 BSWMD 没有 providedModeGroup。 已核对（BSWMD 生成）[Os]：SWS_BSW_00238 |
| Symbolic Name Value | Ecu_TargetConfig.h 包含 Os_Cfg.h，owner task 使用该头唯一生产的 OS_TASK_ID_*；计数器继续从 Os_Counter.h 导入。已核对（配置记录与实际生成头／消费者）[Os]：SWS_BSW_00200 |
