# BSW General：ComM

本篇属于 [BSW General](README.md)，按 ComM 单独处置全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。ComM 的选择边界与拒绝测试见 [ComM](../comm/README.md)，其他模块的标记不替本模块处置。

ComM 只有一个 CDD 通道（ComMNmVariant 为 NONE），经 Ecu_HostBusSM 请求总线模式；COMM_STATE 只保护自身数据，跨模块调用都在区外。开发错误、版本 API、PNC、禁止与限制等可选功能在生成前拒绝，下表的“不适用（配置）”依据这些参数。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 模块缩写与实现前缀均为 ComM；文件为 ComM.c、ComM.h、ComM_BswM.h、ComM_Dcm.h、ComM_EcuM.h、ComM_Internal.h、ComM_MemMap.h，RTE 与 SchM 头沿用 Rte_ComM_Type.h 与生成的 SchM_ComM.h；文件名大小写无歧义。 已核对（源码）[ComM]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | ComM.c 包含自身头、ComM_MemMap.h、SchM_ComM.h，以及所调用 BswM 回调的 BswM_ComM.h；Dcm 通知与总线状态机请求经配置中的函数指针调用；不向 Dem 报告。私有函数与全部数据为 static，没有全局变量。 已核对（源码）[ComM]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | ComM 不实现 ISR、interrupt frame 或寄存器访问，不依赖 MCU。回调只更新有界状态并按序发布，在宿主总线状态机的任务上下文中调用，不由 ISR 进入。 已核对（源码）[ComM]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只有 pre-compile 选择；只读 comm_config 由 Ecu_RuntimeConfig.c 生成，经 ComM_Init 的 const 指针传入，没有 link-time／post-build 配置文件。 已核对（生成样本）[ComM]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184 |
| 实现头 | ComM.h 自包含、带唯一保护，包含 ComStack_Types.h 与 Rte_ComM_Type.h（后者直接包含 Std_Types.h）；BswM、Dcm 与 EcuM 的入口分别在 ComM_BswM.h、ComM_Dcm.h、ComM_EcuM.h；主函数在生成的 SchM_ComM.h，不在模块头；ComM_Internal.h 只供生成的主函数包装使用。 已核对（源码与生成样本）[ComM]：SWS_BSW_00020、00023、00024、00048、00110、00210、00249 |
| RTE 与应用接口 | ComM 源码没有 Rte_ 调用，不包含应用头，公共头不引入应用访问头；回调不经 RTE。ComM SWS 的 SW-C 用户接口（ComM_UserRequest、ComM_CurrentMode 等端口）尚未生成，交付的 BSWMD 没有对应的 AUTOSAR 接口，按用户的模式指示因此也未经 RTE 发出。 已核对（源码）[ComM]：SWS_BSW_00025、00069、00180、00218；差距 #195[ComM]：SWS_BSW_00040 |
| callout | ComM SWS 没有 callout；总线状态机、BswM 与 Dcm 都是 ComM 调用的接口或回调，源码不含 callout 调用，也不需要 ComM_Externals.h。 已核对（原文与源码）[ComM]：SWS_BSW_00135、00136、00254 |
| 编码与命名 | 使用 AUTOSAR 整数与 boolean；boolean 只取 TRUE／FALSE 并直接用作条件；平台关键字只出现在 ComM_MemMap.h 的映射宏中；没有以 defined 为判据的编译开关；宏、枚举值与错误值以 COMM_ 开头且全大写，类型按 ComM_<Tn>Type 命名（ComM_ConfigType 为 SWS 规定的例外）；状态与控制流集中在 ComM_Evaluate、ComM_IssueBus、ComM_Publish，没有重复实现。 已核对（源码）[ComM]：SWS_BSW_00120、00121、00123–00125、00127、00142、00146 |
| 标准类型 | ComM_ModeType、ComM_UserHandleType、ComM_InhibitionStatusType 由 Rte_ComM_Type.h 定义；输入指针（ConfigPtr）保持 const。 已核对（公开头）[ComM]：SWS_BSW_00147、00186 |
| C99 与调度依赖 | 公开头与实现以 C99、pedantic、Werror 构建；ComM 不直接调用 OS 服务，也不调用任何主函数。 已核对（构建与源码）[ComM]：SWS_BSW_00133、00138、00234、00257 |
| 独占区 | COMM_STATE 只保护模块数据，区内不调用其他模块；总线请求、实际模式发布与总线模式查询都在区外，由单飞调用者有序发出。测试 comm_bus_requests_and_mode_publications_stay_ordered_while_another_waits_outside_the_area 用真实线程在 Exit 后进入消费者前交错，并带 Verifies[ComM] 标记；独占区条款由该标记核算。  |
| 主函数 | 生成的 ComM_MainFunction_<Channel>（每通道一个，名称由 ComM SWS 规定）无参数、返回 void，只调用私有 ComM_RunChannel；未初始化时它不推进状态、不等待回调或其他线程。测试 comm_cdd_mode_demand_and_immediate_bswm_use_actual_lower_modes 经 ComM_RunChannel 逐周期推进。 已核对（生成样本与源码）[ComM]：SWS_BSW_00037、00153、00154、00156 |
| 初始化顺序 | Ecu_StackBootstrap 在 Com 之后、BswM 之前调用一次 ComM_Init；ComM_Init 返回前模块已初始化，无效配置不改变状态。ComM_DeInit 只在 NO_COM_NO_PENDING_REQUEST 且总线状态机报告实际 NO 时，在区内一次性回到未初始化；这两个前置条件由 ComM SWS 对 DeInit 的限制决定，所以不能在开始时无条件置位。未初始化时 ComM_DeInit 无动作，当前生成的关闭序列不调用它。 已核对（生成样本与源码）[ComM]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，由生成的 Ecu_StackBootstrap 承担启动与关闭顺序。 不适用（归属：EcuM／BswM 生命周期角色由生成启动器承担）[ComM]：SWS_BSW_00150、00152 |
| 回调 | ComM 提供的回调（ComM_BusSM_ModeIndication、ComM_CommunicationAllowed、ComM_DCM_ActiveDiagnostic、ComM_DCM_InactiveDiagnostic、ComM_EcuM_WakeUpIndication）都返回 void，参数按各自 SWS 传递。 已核对（公开头）[ComM]：SWS_BSW_00172、00173 |
| 配置方式 | 使用 ECUC 工具生成可读配置源，只接纳 VARIANT-PRE-COMPILE；未选择的可选功能在生成前拒绝，对应开关只接受 false；参数名沿用 ECUC 定义。 已核对（配置模型）[ComM]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 主线只允许一个分区；并发安全仍按模块协议验证。 已核对（配置模型与源码）[ComM]：SWS_BSW_00190–00192 |
| 错误分类 | 原文表定义错误分类与类型描述格式；ComM 没有实现专有错误。 已核对（原文与源码）[ComM]：SWS_BSW_00073、00144、00209 |
| 开发错误开关 | ComMDevErrorDetect 默认且固定为 false，参数拒绝有源位置测试；实现不检测、不报告开发错误，也不包含 Det.h。未初始化的 API 按 ComM SWS 不动作并返回 E_NOT_OK。 已核对（配置模型与源码）[ComM]：SWS_BSW_00042；不适用（配置：ECUC_ComM_00555=true）[ComM]：SWS_BSW_00045、00049、00050、00201、00203、00212、00243、00255 |
| 运行时错误 | ComM SWS 只定义 COMM_E_READ_NV_FAILED，仅在读取 NvM 失败时产生；所选配置不使用 NvM，没有该检测点，不调用 Det。 已核对（原文与所选配置）[ComM]：SWS_BSW_00219、00222 |
| 生产错误 | ComM SWS 没有 production error、extended production error 或 DemInitMonitor 回调；没有 Dem 事件。 已核对（原文）[ComM]：SWS_BSW_00046、00066、00143、00204–00208 |
| 安全事件 | ComM SWS 没有定义安全事件，ComMGeneral 没有对应开关，也不调用 IdsM。 已核对（原文与 ECUC 容器）[ComM]：SWS_BSW_00244、00250–00253、00258 |
| 版本 API | ComMVersionInfoApi 默认 false，仅预编译选择；主线只接受 false，不交付 ComM_GetVersionInfo。 已核对（配置模型）[ComM]：SWS_BSW_00051、00236；不适用（配置：ECUC_ComM_00622=true）[ComM]：SWS_BSW_00052、00064、00164 |
| MetaData | ComM 不引用全局 PDU，没有 PDU 元数据可分配、对齐或访问。 不适用（归属：生产与转发 PDU 的 Com、PduR、CanIf 等模块；ComM 不引用全局 PDU）[ComM]：SWS_BSW_00239–00242 |
| Post-build variation | 主线只生成一个预编译配置集，不生成 post-build 变体，也没有变体选择入口。 不适用（归属：配置工具只生成单一预编译配置集；变体选择由 EcuM 角色承担，主线没有 EcuM）[ComM]：SWS_BSW_00226–00228 |
| 发布版本信息 | 差距 #59[ComM]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA | 差距 #58[ComM]：SWS_BSW_00115 |
| 交付描述 ComM 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[ComM]：SWS_BSW_00001 |
| 交付描述 ComM 00002 | 已核对（module_documentation::module 交付 modules/ComM/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[ComM]：SWS_BSW_00002 |
| 交付描述 ComM 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 ComM 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[ComM]：SWS_BSW_00038 |
| 交付描述 ComM 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[ComM]：SWS_BSW_00054 |
| 交付描述 ComM 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[ComM]：SWS_BSW_00061 |
| BSWMD 模式组 | ComM 的 BSWMD 没有 providedModeGroup，因此没有需要映射的 synchronizedModeGroup；SW-C 模式端口补齐后须重新核对。 已核对（BSWMD 生成）[ComM]：SWS_BSW_00238 |
| Symbolic Name Value | ComM_Cfg.h 唯一导出所选配置符号；生成配置从实际提供方的配置头导入，symbolic_handles_compile_link_and_run_renamed_nondefault_mapping 编译、链接并运行。已核对（生成头与真实消费者）[ComM]：SWS_BSW_00200 |
