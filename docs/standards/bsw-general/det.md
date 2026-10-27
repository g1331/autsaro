# BSW General：Det

本篇属于 [BSW General](README.md)，按 Det 单独处置 R24-11 的全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。Det 的配置、通知和失败语义见 [Det](../det.md)，其他模块的标记不替本模块处置。

`DET_STATE` 只保护配置指针和初始化状态。两个报告函数在区内取得不可变配置的快照，在区外调用全部 hooks；Init 重置后在一个临界区中发布新配置。配置对象由调用方持有，旧配置须存活到最后一次使用它的报告结束；并发 Init 不受支持。受控主机的停止和每线程开发报告递归边界均留在私有 `Det_Host` 适配中。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 标准模块缩写和实现前缀均为 Det；实现为 Det.c，公开头为 Det.h，主机接缝为 Det_Host.c／Det_Host.h；生成的 Det_Callouts.c 与 Det_Externals.h 提供已选通知，文件没有大小写歧义。已核对（这些源码与生成路径）[Det]：SWS_BSW_00101–00105、00170、00171 |
| 实现源与包含 | Det.c 包含 Det.h、Det_Host.h、SchM_Det.h 和自身 MemMap；只调用不可变配置中的函数指针，不直接命名集成方函数；生成的 Ecu_RuntimeConfig.c 包含声明该指针目标的 Det_Externals.h，Det_Callouts.c 同时包含其声明头和 ECU 消费者头。模块不调用 Dem。已核对（Det.c、Det_Callouts.c、Ecu_RuntimeConfig.c）[Det]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | Det.c 不实现 ISR、interrupt frame、寄存器访问或 MCU 初始化，也不调用任何 OS 服务。主机停止策略与 TLS 在适配层中；当前主机报告来自任务或回调，不声明 MCU ISR 可用性。已核对（Det.c 的完整调用路径）[Det]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只接纳预编译变体，det_config 和有界 callout 表是生成源中的只读数据；Det SWS 没有 link-time／post-build 参数，也不支持 post-build variant。已核对（ECUC_Det_00001 与 Ecu_RuntimeConfig.c）[Det]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184、00226–00228 |
| 实现头 | Det.h 自包含，包含 Std_Types 并只公开配置类型和四个标准 API；没有主函数或 BSW C/S runnable 原型，私有主机头不被公开头间接包含。已核对（Det.h）[Det]：SWS_BSW_00020、00024、00048、00110、00210、00249 |
| RTE 服务类型 | DetConfigSet 的非空 DetModule 会在生成前拒绝，没有 SWC 服务类型和端口。 不适用（配置：ECUC_Det_00008）[Det]：SWS_BSW_00023、00147 |
| RTE 调用 | Det 不调用 RTE，也不包含应用头或经 RTE 发布 callback；直接 hook 通过标准函数指针调用。已核对（Det.c 和生成的 Det_Callouts.c）[Det]：SWS_BSW_00025、00069、00180、00218 |
| 已选 callout | Ecu_DetRuntimeCallout 的声明在 Det_Externals.h，定义在独立 Det_Callouts.c；二者使用 DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE，目标文件具有 .text.det.Ecu_DetRuntimeCallout。det_callout.c 通过实际头与 callout 源编译、链接、运行，原生测试检查目标段；条款由测试中的 Verifies[Det] 核算 |
| 编码与类型 | Det.c／Det.h 使用 AUTOSAR uint16、uint8 和 boolean，boolean 只赋值、返回及与 TRUE／FALSE 比较；普通模块没有平台关键字或可选编译分支。平台属性只在 MemMap 映射中，guard 和段选择不属于功能开关。已核对（Det.c、Det.h、Det_MemMap.h）[Det]：SWS_BSW_00120、00121、00123–00125、00127、00142、00146 |
| 指针与 C99 | Init 使用 const 配置指针，两个通知数组也不可修改；标准消费者以 C99／pedantic／Werror 独立编译、链接、执行。模块不调用其他 Main，也不直接调用 OS 服务。已核对（Det.h、Det.c 与 det_contract.c 的独立签名）[Det]：SWS_BSW_00133、00138、00186、00234、00257 |
| 独占区 | 只保护自身配置与状态，hooks 在区外；真实线程 callout 再进入 DET_STATE，不会嵌套持区或丢失通知。条款由 det_parallel.c 中的 Verifies[Det] 核算 |
| 主函数 | Det SWS 没有 MainFunction，Det.c 没有计划执行入口，也不等待其他主函数。已核对（Det API 全集与源码）[Det]：SWS_BSW_00037、00153、00154、00156 |
| 初始化顺序 | Ecu_StackBootstrap 在其他 BSW 之前调用 Det_Init、Det_Start，Init 返回前配置与状态已发布；正常启动一次初始化，没有 Det_DeInit。SWS_Det_00020 特别要求每次 Init 回到初态，独立消费者验证该模块特定义务。已核对（Ecu_StackBootstrap 与 Det_Init）[Det]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，生成的 Ecu_StackBootstrap 承担生命周期角色；测试中的直接 Init 不代表生产初始化入口。 不适用（归属：EcuM／BswM 生命周期角色由生成启动器承担）[Det]：SWS_BSW_00150、00152 |
| 回调提供者 | Det 提供标准报告服务，不向其他模块提供 callback；用户 hooks 的 Std_ReturnType 由 Det SWS 明确规定。已核对（SWS_Det_00009／01001／00181／00184 和 Det.h）[Det]：SWS_BSW_00172、00173 |
| 配置方式与可选功能 | 内置定义严格表达官方 ECUC 参数；实际输入只有 pre-compile，配置为可读 ARXML 和 C。DLT、版本 API、其他 hooks 和 SWC Det 服务均在源对象拒绝，不静默忽略。已核对（builtin::error_tracing、configuration::det 与普通校验）[Det]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 主线只接纳一个分区；报告本身仍由真实线程测试并发安全性，不能由单 ECU owner 假定替代。已核对（分区计划与 det_parallel.c）[Det]：SWS_BSW_00190–00192 |
| 模块自身错误 | Det SWS §7.5 只允许 GetVersionInfo 的空指针开发错误，不允许 Det 自己再报告 runtime、production 或 extended production errors；未初始化 runtime 报告按专门条款直接返回，开发报告按专门条款停止。已核对（Det SWS §7.5、00024、00208 和完整报告路径）[Det]：SWS_BSW_00042、00045、00046、00049、00050、00066、00073、00125、00143、00144、00201、00203–00209、00212、00219、00222、00243、00255 |
| 安全事件 | Det SWS 没有安全事件，DetGeneral 没有对应开关，模块也不调用 IdsM。已核对（Det SWS §7.5、全部 Det ECUC 定义和 Det.c）[Det]：SWS_BSW_00244、00250–00253、00258 |
| 版本 API | DetVersionInfoApi 必填、默认 false；目标拒绝 true，不提供 Det_GetVersionInfo。已核对（builtin::error_tracing 与配置正反例）[Det]：SWS_BSW_00051、00236；不适用（配置：ECUC_Det_00003=true）[Det]：SWS_BSW_00052、00064、00164 |
| MetaData | Det 不处理 PDU、不引用 EcuC 全局 PDU 或分配 MetaData；它只按值传递四个标准报告身份。已核对（Det 全部 API 与 ECUC 参数）[Det]：SWS_BSW_00239–00242 |
| 发布版本信息 | 不适用（归属：SWS_Det_NA_00999 排除 SRS_BSW_00004 的消费者义务）[Det]：SWS_BSW_00036；头 release 已发布，完整发布信息仍为差距 #59[Det]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[Det]：SWS_BSW_00162 |
| MISRA | 保留源码分析原始结果；完整授权原文、221 项人工评估与主机 abort 的评估尚未闭合。差距 #58[Det]：SWS_BSW_00115 |
| 交付描述 Det 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[Det]：SWS_BSW_00001 |
| 交付描述 Det 00002 | 已核对（module_documentation::module 交付 modules/Det/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[Det]：SWS_BSW_00002 |
| 交付描述 Det 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 Det 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[Det]：SWS_BSW_00038 |
| SW-C 服务接口 | 非空 DetModule 在生成前拒绝，不提供 SW-C 服务接口。不适用（配置：ECUC_Det_00008）[Det]：SWS_BSW_00040 |
| 交付描述 Det 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[Det]：SWS_BSW_00054 |
| 交付描述 Det 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[Det]：SWS_BSW_00061 |
| BSWMD 模式组 | 已核对（Det 没有模式声明或模式接口）[Det]：SWS_BSW_00238 |
| Symbolic Name Value | 所选配置仅有 DetGeneral／DetNotification，没有 DetModule 实例及需消费的 Symbolic Name Value；BSW 发布的固定 ModuleId 不是当前 ECUC 的 DetModuleId 值。导出器识别 DetModuleId，但本配置不生成没有提供值的 Det_Cfg.h，也不虚构配置实例。已核对（配置记录与实际生成头／消费者）[Det]：SWS_BSW_00200 |

## Det 映射头的必需重复处理

Det实现的CODE由`integration/src/Det_Host.h、integration/src/Det_Host.c、bsw/include/Det.h、bsw/src/Det.c`消费；ECU_DETRUNTIMECALLOUT_CODE由`generated/include/Det_Externals.h、generated/src/Det_Callouts.c`消费；VAR_CLEARED_8由`integration/src/Det_Host.c`消费；VAR_CLEARED_UNSPECIFIED由`bsw/src/Det.c`消费。这些具体请求与声明／定义的归属沿本模块及对应标准篇核查；不是仅凭头名推定角色。固定MemoryMapping00003／00011／00023（PDF36、41、37）要求完整开闭、撤销请求且不在函数体内包含头。整头guard会吞掉STOP／后续class，保留已处理关键词也违反00011；固定属性仍单独保护，每次检查嵌套／错配及请求／属性撤销。

D4.10为Required（主文32–33／PDF40–41，TC2.2无映射例外），R20.5为Advisory（158／166）；AMD2追加C11不改变当前C99适用性。记录绑定cf5d6fa1的字面指令；本次仅增注释，全部C／预处理token保持。原3ea747fe..9204ed30的请求／头闭包复核及后续对应模块声明映射复核为技术依据；C／C++实际配对、消费、属性撤销和无效／多请求拒绝收据按原head保留。改变类别、前缀、目标、实体或条件须重评。

| MISRA 条目 | 源码范围 | 必需性与风险控制 | 原授权与技术处置 |
| --- | --- | --- | --- |
| MISRA C:2012 Directive 4.10 | `runtime/bsw/include/Det_MemMap.h` 中 `@file` | 00003／00011需逐次处理START／STOP；固定属性guard不遮蔽分派主体。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |
| MISRA C:2012 Rule 20.5 | `runtime/bsw/include/Det_MemMap.h` 中 `#undef DET_START_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef DET_START_SEC_VAR_CLEARED_8`、`#undef DET_STOP_SEC_VAR_CLEARED_8`、`#undef DET_START_SEC_CODE`、`#undef DET_STOP_SEC_CODE`、`#undef DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE`、`#undef DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE` | 00011要求消费这些请求；保留class／active校验及STOP属性清理。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |

原授权见[#58决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)。SWS_BSW_00115就近技术注释与上列指令／文件主体对应；内部active／属性清理和R20.1不随之获准。共享源注释不声称新Windows执行、MISRA合法偏离或整体符合；原始诊断、not_assessed、strict refusal及其余00115差距保留。
