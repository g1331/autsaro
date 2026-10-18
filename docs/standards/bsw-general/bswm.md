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
| 发布版本信息 | 差距 #59[BswM]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA | 差距 #58[BswM]：SWS_BSW_00115 |
| 完整 BSWMD／模块文档 | 交付的 BSWMD 已描述 BswM 入口与独占区 BSWM_STATE，但不是完整的模块描述与文档，BswM_MainFunction 的调用顺序也没有文档。 差距 #159[BswM]：SWS_BSW_00001、00002、00038、00054、00061 |
| BSWMD 模式组 | BswM 的 BSWMD 没有 providedModeGroup，因此没有需要映射的 synchronizedModeGroup。 已核对（BSWMD 生成）[BswM]：SWS_BSW_00238 |
| Symbolic Name Value | 配置、动作与通道比较包含 ComM_Cfg.h 并使用 ComMConf_ComMChannel_*；独立非默认句柄生成工程实际运行。已核对（生成头与真实消费者）[BswM]：SWS_BSW_00200 |
