# BSW General：LSduR 与 PduR

本篇属于 [BSW General](README.md)，标记与核算方法见[标准与规范基线 §5](../README.md#5-条款处置与核算)。

`SWS_BSWGeneral` 的全部 150 条按模块分别处置，标记带 `[LSduR]` 或 `[PduR]`。由测试验证的条目见 `core/tests/fixtures/bsw-contracts/routing_contract.c`（`SWS_BSW_00186` 输入指针为 const、`00071` Init 结束时进入 ONLINE）与 `routing_parallel.c`（`00134` 独占区只保护模块数据）。“配置”类的依据与 [PduR](../pdur.md)、[LSduR](../lsdur.md) 篇相同：LSduR 模块整体不被接受，PduR 的对应参数在生成前被拒绝。

| 分组 | 依据 | 处置 |
| --- | --- | --- |
| 命名与文件 | 模块缩写与实现前缀为 `LSduR`／`PduR`，文件按 `<Mip>[_<Ie>]` 命名，文件名区分大小写且互不只差大小写 | 已核对（源码）[LSduR]：SWS_BSW_00101、00102、00103、00170、00171；已核对（源码）[PduR]：SWS_BSW_00101、00102、00103、00170、00171 |
| 导入与导出 | 只包含所需头文件；公开头只导出调用方与生成配置需要的类型和声明 | 已核对（源码）[LSduR]：SWS_BSW_00104、00105；已核对（源码）[PduR]：SWS_BSW_00104、00105 |
| 实现源文件 | `<Mip>.c` 包含自身实现头、`<Mip>_MemMap.h` 与 `SchM_<Mip>.h`，内部函数与数据为 static；不向 Dem 报告；LSduR 直接调用的 `CanIf_Transmit` 经 `CanIf.h` 声明，其余回调经生成路由表调用，`<Mip>_PBcfg.c` 包含定义这些回调的模块头 | 已核对（源码）[LSduR]：SWS_BSW_00004、00060、00005、00006、00007、00008、00010；已核对（源码）[PduR]：SWS_BSW_00004、00060、00005、00006、00007、00008、00010 |
| 中断与硬件 | 不实现 ISR、中断框架或硬件寄存器访问，不依赖微控制器 | 已核对（源码）[LSduR]：SWS_BSW_00003、00017、00181、00016、00021、00019、00018、00043、00068、00137、00167、00041、00065、00182、00179、00188、00119；已核对（源码）[PduR]：SWS_BSW_00003、00017、00181、00016、00021、00019、00018、00043、00068、00137、00167、00041、00065、00182、00179、00188、00119 |
| 配置源文件 | 没有 link-time 参数；路由表与配置标识生成在 `<Mip>_PBcfg.c` 的 `<Mip>_Config` 结构中，实现经配置指针访问 | 已核对（生成样本）[LSduR]：SWS_BSW_00013、00014、00015、00063、00184、00056、00057、00160、00117；已核对（生成样本）[PduR]：SWS_BSW_00013、00014、00015、00063、00184、00056、00057、00160、00117 |
| 实现头文件 | `<Mip>.h` 与按调用方分开的接口头声明全部 API，带 include 保护并直接包含 `Std_Types.h`；模块不提供 AUTOSAR Service、MainFunction 或 callout | 已核对（源码）[LSduR]：SWS_BSW_00020、00110、00023、00024、00048、00254、00210、00249、00025、00069、00040、00180、00218、00238、00135、00136；已核对（源码）[PduR]：SWS_BSW_00020、00110、00023、00024、00048、00254、00210、00249、00025、00069、00040、00180、00218、00238、00135、00136 |
| 编码约束 | 只用 AUTOSAR 整数类型与 `boolean` 的赋值、返回和相等比较；不直接用编译器关键字；没有编译开关；枚举与宏按 `<MIP>_` 大写命名，类型按 `<Ma>_<Tn>Type` 命名；外部链接的全局量只有生成的只读 `<Mip>_Config`；不调用调度函数，也不使用 OS 服务 | 已核对（源码）[LSduR]：SWS_BSW_00120、00142、00121、00123、00124、00146、00127、00129、00130、00131、00133、00138、00257、00147；已核对（源码）[PduR]：SWS_BSW_00120、00142、00121、00123、00124、00146、00127、00129、00130、00131、00133、00138、00257、00147 |
| C 标准 | 交付源码与契约夹具以 `-std=c99 -pedantic -Werror` 编译，外部标识符在 31 个字符以内 | 已核对（构建）[LSduR]：SWS_BSW_00234；已核对（构建）[PduR]：SWS_BSW_00234 |
| 配置方式 | 由工具从 ECUC 生成人可读的 C 配置，只提供 VARIANT-PRE-COMPILE，可选功能不实现，对应参数只接受 `false`；参数名沿用 ECUC 定义 | 已核对（配置模型）[LSduR]：SWS_BSW_00116、00029、00126、00237、00157、00183；已核对（配置模型）[PduR]：SWS_BSW_00116、00029、00126、00237、00157、00183 |
| 配置指针 | 只有生成的启动序列把 `&<Mip>_Config` 传给 `<Mip>_Init` | 已核对（生成样本）[LSduR]：SWS_BSW_00158；已核对（生成样本）[PduR]：SWS_BSW_00158 |
| 调度函数 | 模块没有调度函数 | 已核对（源码）[LSduR]：SWS_BSW_00054、00037、00153、00154、00156；已核对（源码）[PduR]：SWS_BSW_00054、00037、00153、00154、00156 |
| 初始化 | 启动序列在 Com 与 CanTp 之前各调用一次 `<Mip>_Init`；再次调用被忽略 | 已核对（生成样本）[LSduR]：SWS_BSW_00230、00231；已核对（生成样本）[PduR]：SWS_BSW_00230、00231 |
| De-Init | 模块 SWS 没有 De-Initialization 函数 | 已核对（规范原文）[LSduR]：SWS_BSW_00152、00072、00232、00233；已核对（规范原文）[PduR]：SWS_BSW_00152、00072、00232、00233 |
| 回调形式 | 返回 `Std_ReturnType`／`BufReq_ReturnType` 的回调签名由模块 SWS 规定 | 已核对（规范原文）[LSduR]：SWS_BSW_00172、00173；已核对（规范原文）[PduR]：SWS_BSW_00172、00173 |
| 多分区 | 主线只有一个分区 | 已核对（配置模型）[LSduR]：SWS_BSW_00190、00191、00192；已核对（配置模型）[PduR]：SWS_BSW_00190、00191、00192 |
| 错误分类与专有错误 | 分类定义与文档格式，没有实现义务；实现没有专有错误，所选配置也不定义错误值 | 已核对（规范原文）[LSduR]：SWS_BSW_00144、00209、00073、00125；已核对（规范原文）[PduR]：SWS_BSW_00144、00209、00073、00125 |
| 开发错误开关 | 开关为 false，实现不检测也不报告开发错误 | 已核对（源码）[LSduR]：SWS_BSW_00042；已核对（源码）[PduR]：SWS_BSW_00042 |
| 安全事件 | 模块 SWS 没有定义安全事件；00252／00253 为 obsolete | 已核对（规范原文）[LSduR]：SWS_BSW_00244、00250、00251、00252、00253、00258；已核对（规范原文）[PduR]：SWS_BSW_00244、00250、00251、00252、00253、00258 |
| 初始化调用方 |  | 不适用（归属：EcuM 角色——主线没有 EcuM 与 BswM，生成的 `Ecu_StackBootstrap` 承担启动顺序）[LSduR]：SWS_BSW_00150；不适用（归属：EcuM 角色——主线没有 EcuM 与 BswM，生成的 `Ecu_StackBootstrap` 承担启动顺序）[PduR]：SWS_BSW_00150 |
| 跨模块版本检查与发布信息 |  | 不适用（归属：SWS_LSduR_NA 排除 SRS_BSW_00004 的消费者义务）[LSduR]：SWS_BSW_00036；头 release 已发布，完整发布信息仍为差距 #59[LSduR]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[LSduR]：SWS_BSW_00162；已核对（`PduR.h`／`PduR.c` 及 `release_contracts::direct_external_headers_refuse_only_missing_or_wrong_major_minor`）[PduR]：SWS_BSW_00036；release 4.10.0 已发布，正式 vendor 身份仍为差距 #59[PduR]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[PduR]：SWS_BSW_00162 |
| MISRA |  | 差距 #58[LSduR]：SWS_BSW_00115；差距 #58[PduR]：SWS_BSW_00115 |
| 交付描述 LSduR 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[LSduR]：SWS_BSW_00001 |
| 交付描述 LSduR 00002 | 已核对（module_documentation::module 交付 modules/LSduR/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[LSduR]：SWS_BSW_00002 |
| 交付描述 LSduR 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 LSduR 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[LSduR]：SWS_BSW_00038 |
| 交付描述 LSduR 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[LSduR]：SWS_BSW_00061 |
| 交付描述 PduR 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[PduR]：SWS_BSW_00001 |
| 交付描述 PduR 00002 | 已核对（module_documentation::module 交付 modules/PduR/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[PduR]：SWS_BSW_00002 |
| 交付描述 PduR 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 PduR 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[PduR]：SWS_BSW_00038 |
| 交付描述 PduR 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[PduR]：SWS_BSW_00061 |
| Symbolic Name Value | PduR_Cfg.h 与 LSduR_Cfg.h 唯一生产各自符号；LSduR 仅导出由已验证路由合成的公开入口句柄，目的 ID 从实际提供模块导入，不接纳独立 LSduR ECUC。PBcfg 消费者包含提供方头，非等 ID／改名工程实际运行 | 已核对（symbolic_handles_compile_link_and_run_renamed_nondefault_mapping）[LSduR]：SWS_BSW_00200；已核对（symbolic_handles_compile_link_and_run_renamed_nondefault_mapping）[PduR]：SWS_BSW_00200 |
| 开发错误 |  | 不适用（配置：ECUC_PduR_00302=true）[PduR]：SWS_BSW_00255、00201、00203、00045、00243、00049、00212、00050；不适用（配置：ECUC_LSduR_00001）[LSduR]：SWS_BSW_00255、00201、00203、00045、00243、00049、00212、00050 |
| 运行时错误 |  | 不适用（配置：ECUC_PduR_00291、ECUC_PduR_00356）[PduR]：SWS_BSW_00219、00222；不适用（配置：ECUC_LSduR_00001）[LSduR]：SWS_BSW_00219、00222 |
| 生产错误 | LSduR SWS 没有定义生产错误 | 不适用（配置：ECUC_PduR_00365）[PduR]：SWS_BSW_00204、00205、00143、00046、00066、00206、00207、00208；已核对（规范原文）[LSduR]：SWS_BSW_00204、00205、00143、00046、00066、00206、00207、00208 |
| 版本信息 API | `PduRVersionInfoApi` 在预编译期配置、只接受默认值 `false` | 已核对（配置模型）[PduR]：SWS_BSW_00051、00236；不适用（配置：ECUC_PduR_00316=true）[PduR]：SWS_BSW_00064、00052、00164；不适用（配置：ECUC_LSduR_00001）[LSduR]：SWS_BSW_00051、00236、00064、00052、00164 |
| metadata |  | 不适用（配置：ECUC_PduR_00347=true）[PduR]：SWS_BSW_00239、00240、00241、00242；不适用（配置：ECUC_LSduR_00001）[LSduR]：SWS_BSW_00239、00240、00241、00242 |
| post-build 多变体 |  | 不适用（配置：ECUC_PduR_00293 声明 VARIANT-POST-BUILD）[PduR]：SWS_BSW_00226、00227、00228；不适用（配置：ECUC_LSduR_00001）[LSduR]：SWS_BSW_00226、00227、00228 |
