# BSW General：BswM

本篇属于 [BSW General](README.md)，按 BswM 单独处置全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。BswM 的选择边界与拒绝测试见 [BswM](../bswm/README.md)，其他模块的标记不替本模块处置。

BswM 只有一个 IMMEDIATE 的 ComM 模式输入和三条规则，动作是静态的 BswMUserCallout；BSWM_STATE 只保护自身数据，动作列表在区外运行。开发错误、版本 API、通用请求等可选功能在生成前拒绝，下表的“不适用（配置）”依据这些参数。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 模块缩写与实现前缀均为 BswM；文件为 BswM.c、BswM.h、BswM_ComM.h、BswM_MemMap.h 与 SchM_BswM.h；文件名大小写无歧义。 已核对（源码）[BswM]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | BswM.c 包含自身头、BswM_MemMap.h、BswM_ComM.h 与 SchM_BswM.h（提供独占区与主函数）；动作列表经配置中的函数指针调用，不直接调用其他模块的回调；不向 Dem 报告。私有函数与全部数据为 static，没有全局变量。 已核对（源码）[BswM]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | BswM 不实现 ISR、interrupt frame 或寄存器访问，不依赖 MCU。ComM 回调在宿主任务上下文执行，只更新有界状态并在区外运行动作列表。 已核对（源码）[BswM]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只有 pre-compile 选择；只读 bswm_config 由 Ecu_RuntimeConfig.c 生成，经 BswM_Init 的 const 指针传入，没有 link-time／post-build 配置文件。 已核对（生成样本）[BswM]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184 |
| 实现头 | BswM.h 自包含、带唯一保护，经 ComM.h 取得 Std_Types.h 与 ComM_ModeType；ComM 的入口 BswM_ComM_CurrentMode 在 BswM_ComM.h；主函数在 SchM_BswM.h，不在模块头。 已核对（源码）[BswM]：SWS_BSW_00020、00024、00048、00110、00210、00249 |
| RTE 与应用接口 | BswM 源码没有 Rte_ 调用，不包含应用头，BswM 头也不引入应用访问头。 已核对（源码）[BswM]：SWS_BSW_00069、00218 |
| AUTOSAR 服务 | BswM 的 AUTOSAR 服务只有 BswM_RequestMode（通用请求端口）；BswMGenericRequestEnabled 只接受 false，所选配置不实现任何 AUTOSAR 服务，不生成端口、回调或 BSWMD 服务接口。 不适用（配置：ECUC_BswM_00949=true）[BswM]：SWS_BSW_00023、00025、00040、00180 |
| callout | BswM SWS 没有规定 callout 原型；BswMUserCallout 按 ECUC_BswM_00843／00955 调用一个已有函数，并包含 BswMUserIncludeFile 指定的头。主线调用的 Ecu_HostBusSM_ApplyMode 是主机总线状态管理器在 Ecu_HostBusSM.h 声明、在其 ECU_HOSTBUSSM 代码段实现的公开函数，生成的 Ecu_BswMAction 包含该头；BswM 自身不声明 callout，也就没有 BswM_Externals.h 与 callout 代码段。 不适用（归属：Ecu_HostBusSM：被调用函数的声明与内存段由提供它的模块承担）[BswM]：SWS_BSW_00135、00136、00254 |
| 编码与命名 | 使用 AUTOSAR 整数与 boolean；boolean 只取 TRUE／FALSE 并直接用作条件；平台关键字只出现在 BswM_MemMap.h 的映射宏中；没有以 defined 为判据的编译开关；宏与枚举以 BSWM_ 开头且全大写，类型按 BswM_<Tn>Type 命名（BswM_ConfigType 为 SWS 规定的例外）；BswM 没有错误值定义。 已核对（源码）[BswM]：SWS_BSW_00120、00121、00123–00125、00127、00142、00146 |
| 标准类型 | BswM_ModeType 与 BswM_UserType 按 SWS_BswM_00214／00216（“Available via BswM.h”）由 BswM.h 提供，所选配置没有使用它们的 RTE 接口；ComM_ModeType 来自 Rte_ComM_Type.h；输入指针（ConfigPtr）保持 const。 已核对（公开头）[BswM]：SWS_BSW_00147、00186 |
| C99 与调度依赖 | 公开头与实现以 C99、pedantic、Werror 构建；BswM 不直接调用 OS 服务，也不调用任何主函数。 已核对（构建与源码）[BswM]：SWS_BSW_00133、00138、00234、00257 |
| 独占区 | BSWM_STATE 只保护模块数据，区内不调用其他模块；动作列表在区外运行，动作期间到达的输入被推迟，在同一调用者上下文中于当前动作之后各仲裁一次。测试 bswm_actions_run_once_in_order_while_another_waits_outside_the_area 用真实线程交错，并带 Verifies[BswM] 标记；独占区条款由该标记核算。  |
| 主函数 | BswM_MainFunction 在 SchM_BswM.h 声明，无参数、返回 void；所选配置没有 DEFERRED 输入，函数体为空，未初始化时同样立即返回，也不进入等待；生成计划不调度它。 已核对（源码）[BswM]：SWS_BSW_00037、00153、00154、00156 |
| 初始化顺序 | Ecu_StackBootstrap 在 ComM_Init 之后调用一次 BswM_Init；BswM_Init 在区内一次性置为已初始化，无效配置不改变状态。BswM_Deinit 在开始时置为未初始化，之后任何输入都无效；生成的关闭序列只对已到达该阶段的流程调用一次。BswM 在未初始化时对输入不动作。 已核对（生成样本与源码）[BswM]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，由生成的 Ecu_StackBootstrap 承担启动与关闭顺序。 不适用（归属：EcuM／BswM 生命周期角色由生成启动器承担）[BswM]：SWS_BSW_00150、00152 |
| 回调 | BswM 提供的回调 BswM_ComM_CurrentMode 返回 void，参数按 BswM SWS 传递。 已核对（公开头）[BswM]：SWS_BSW_00172、00173 |
| 配置方式 | 使用 ECUC 工具生成可读配置源，只接纳 VARIANT-PRE-COMPILE；未选择的可选功能在生成前拒绝，对应开关只接受 false；参数名沿用 ECUC 定义。 已核对（配置模型）[BswM]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 主线只允许一个分区；并发安全仍按模块协议验证。 已核对（配置模型与源码）[BswM]：SWS_BSW_00190–00192 |
| 错误分类 | 原文表定义错误分类与类型描述格式；BswM 没有实现专有错误。 已核对（原文与源码）[BswM]：SWS_BSW_00073、00144、00209 |
| 开发错误开关 | BswMDevErrorDetect 默认且固定为 false，参数拒绝有源位置测试；实现不检测、不报告开发错误，也不包含 Det.h。 已核对（配置模型与源码）[BswM]：SWS_BSW_00042；不适用（配置：ECUC_BswM_00811=true）[BswM]：SWS_BSW_00045、00049、00050、00201、00203、00212、00243、00255 |
| 运行时错误 | BswM SWS 只定义 BSWM_E_ACTION_FAILED，且仅在动作项配置 BswMReportFailRuntimeErrorId 时报告；所选配置不得含该参数，BswM 不调用 Det。运行时错误值类型 uint8 按原文核对，BswM 不定义该值。 已核对（原文与源码）[BswM]：SWS_BSW_00219；不适用（配置：ECUC_BswM_01050）[BswM]：SWS_BSW_00222 |
| 生产错误 | BswM SWS 没有 production error、extended production error 或 DemInitMonitor 回调；没有 Dem 事件。 已核对（原文）[BswM]：SWS_BSW_00046、00066、00143、00204–00208 |
| 安全事件 | BswM SWS 没有定义安全事件，BswMGeneral 没有对应开关，也不调用 IdsM。 已核对（原文与 ECUC 容器）[BswM]：SWS_BSW_00244、00250–00253、00258 |
| 版本 API | BswMVersionInfoApi 默认 false，仅预编译选择；主线只接受 false，不交付 BswM_GetVersionInfo。 已核对（配置模型）[BswM]：SWS_BSW_00051、00236；不适用（配置：ECUC_BswM_00812=true）[BswM]：SWS_BSW_00052、00064、00164 |
| MetaData | BswM 不引用全局 PDU，没有 PDU 元数据可分配、对齐或访问。 不适用（归属：生产与转发 PDU 的 Com、PduR、CanIf 等模块；BswM 不引用全局 PDU）[BswM]：SWS_BSW_00239–00242 |
| Post-build variation | 主线只生成一个预编译配置集，不生成 post-build 变体，也没有变体选择入口。 不适用（归属：配置工具只生成单一预编译配置集；变体选择由 EcuM 角色承担，主线没有 EcuM）[BswM]：SWS_BSW_00226–00228 |
| 发布版本信息 | 已核对（`BswM.h`／`BswM.c` 及 `release_contracts::direct_external_headers_refuse_only_missing_or_wrong_major_minor`）[BswM]：SWS_BSW_00036；release 4.10.0 已发布，正式 vendor 身份仍为差距 #59[BswM]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[BswM]：SWS_BSW_00162 |
| MISRA | 差距 #58[BswM]：SWS_BSW_00115 |
| 交付描述 BswM 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[BswM]：SWS_BSW_00001 |
| 交付描述 BswM 00002 | 已核对（module_documentation::module 交付 modules/BswM/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[BswM]：SWS_BSW_00002 |
| 交付描述 BswM 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 BswM 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[BswM]：SWS_BSW_00038 |
| 交付描述 BswM 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[BswM]：SWS_BSW_00054 |
| 交付描述 BswM 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[BswM]：SWS_BSW_00061 |
| BSWMD 模式组 | BswM 的 BSWMD 没有 providedModeGroup，因此没有需要映射的 synchronizedModeGroup。 已核对（BSWMD 生成）[BswM]：SWS_BSW_00238 |
| Symbolic Name Value | 配置、动作与通道比较包含 ComM_Cfg.h 并使用 ComMConf_ComMChannel_*；独立非默认句柄生成工程实际运行。已核对（生成头与真实消费者）[BswM]：SWS_BSW_00200 |

## BswM 映射头的必需重复处理

BswM实现的CODE由`generated/include/SchM_BswM.h、bsw/include/BswM_ComM.h、bsw/include/BswM.h、bsw/src/BswM.c`消费；VAR_CLEARED_UNSPECIFIED由`bsw/src/BswM.c`消费。这些具体请求与声明／定义的归属沿本模块及对应标准篇核查；不是仅凭头名推定角色。固定MemoryMapping00003／00011／00023（PDF36、41、37）要求完整开闭、撤销请求且不在函数体内包含头。整头guard会吞掉STOP／后续class，保留已处理关键词也违反00011；固定属性仍单独保护，每次检查嵌套／错配及请求／属性撤销。

D4.10为Required（主文32–33／PDF40–41，TC2.2无映射例外），R20.5为Advisory（158／166）；AMD2追加C11不改变当前C99适用性。记录绑定cf5d6fa1的字面指令；本次仅增注释，全部C／预处理token保持。原3ea747fe..9204ed30的请求／头闭包复核及后续对应模块声明映射复核为技术依据；C／C++实际配对、消费、属性撤销和无效／多请求拒绝收据按原head保留。改变类别、前缀、目标、实体或条件须重评。

| MISRA 条目 | 源码范围 | 必需性与风险控制 | 原授权与技术处置 |
| --- | --- | --- | --- |
| MISRA C:2012 Directive 4.10 | `runtime/bsw/include/BswM_MemMap.h` 中 `@file` | 00003／00011需逐次处理START／STOP；固定属性guard不遮蔽分派主体。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |
| MISRA C:2012 Rule 20.5 | `runtime/bsw/include/BswM_MemMap.h` 中 `#undef BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef BSWM_START_SEC_CODE`、`#undef BSWM_STOP_SEC_CODE` | 00011要求消费这些请求；保留class／active校验及STOP属性清理。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |

原授权见[#58决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)。SWS_BSW_00115就近技术注释与上列指令／文件主体对应；内部active／属性清理和R20.1不随之获准。共享源注释不声称新Windows执行、MISRA合法偏离或整体符合；原始诊断、not_assessed、strict refusal及其余00115差距保留。
