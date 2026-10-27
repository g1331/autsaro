# BSW General：Dcm

本篇属于 [BSW General](README.md)，按 Dcm 单独处置全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。Dcm 选择边界与拒绝测试见 [Dcm](../dcm/README.md)，其他模块的标记不替本模块处置。

Dcm 使用自己的 `DCM_STATE`，跨模块调用与应用回调都在区外。同步应用调用使用单飞执行器和调用期存储，模式与 ComM 通知使用单飞发布器；Init 保留仍在执行的 actor 身份，避免重启交错破坏非重入义务。生成的立即 SchM 模式读写以自身区域串行化，可由 Dcm 的合法调用上下文使用。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 模块及实现前缀均为 Dcm；文件按职责命名，大小写无歧义。已核对（源码）[Dcm]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | Dcm.c 包含自身头、MemMap、SchM 及所调用 BSW 的头；私有函数、数据均为 static。应用读取经生成的 RTE 桥接，该桥接所在生成源包含其应用接口头。已核对（源码与生成样本）[Dcm]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | Dcm 不实现 ISR、interrupt frame 或寄存器访问，不依赖 MCU。Rx／Tx 回调只复制有界数据和更新状态；应用 ReadData 只由 Main 调用。已核对（源码）[Dcm]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只有 pre-compile 选择；只读 dcm_config 由 Ecu_RuntimeConfig.c 生成，没有 link-time／post-build 配置文件。已核对（生成样本）[Dcm]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184 |
| 实现头 | Dcm.h 自包含、带唯一保护，公开 Std_Types、Rte_Dcm_Type 及必要 API；Main 在 SchM_Dcm.h。已核对（源码）[Dcm]：SWS_BSW_00020、00023、00024、00048、00110、00210、00249 |
| RTE 与应用接口 | 生成 RTE 桥接包含应用头，公共 Dcm 头只导入服务类型，不包含应用访问头。首次 Main 前只进行 SchM 模式发布与 BSW 通知，不调用应用 Rte_Call。已核对（源码与生成样本）[Dcm]：SWS_BSW_00025、00069、00180、00218 |
| 可选 callout | 同步 ReadData 使用标准 C/S port；配置不接纳内存、文件与外置诊断服务函数，不生成这些 callout。不可把函数指针适配当作已实现全部 callout。 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00735、ECUC_Dcm_00777）[Dcm]：SWS_BSW_00135、00136、00254 |
| 编码与命名 | 使用 AUTOSAR 整数／boolean；boolean 仅赋值、返回和与 TRUE／FALSE 比较；宏、类型与私有控制流遵循项目约定，不直接依赖平台关键字。已核对（源码）[Dcm]：SWS_BSW_00120、00121、00123–00127、00142、00146 |
| 标准类型 | 服务类型由 Rte_Dcm_Type.h 提供；配置派生的标准数组别名引用 Rte_Type.h 中的真实应用类型。输入指针保持 const。已核对（公开头与生成样本）[Dcm]：SWS_BSW_00147、00186 |
| C99 与调度依赖 | 公开消费者以 C99、pedantic、Werror 编译／链接。Dcm 不直接调用 OS 服务或其他 Main；其调度周期来自生成计划。已核对（构建与源码）[Dcm]：SWS_BSW_00133、00138、00234、00257 |
| 独占区 | 模块只保护自身数据，区内不调用其他模块；真实线程在 Exit 后进入消费者前交错。测试中另有带模块限定的 Verifies 标记。条款由原生线程测试的 Verifies[Dcm] 标记核算 |
| Main | 唯一 Dcm_MainFunction 无参数、返回 void；未初始化不执行处理，不等待回调或其他线程。已核对（源码与原生消费者）[Dcm]：SWS_BSW_00037、00153、00154、00156 |
| 初始化顺序 | Ecu_StackBootstrap 按阶段调用 Dcm_Init；正常启动只调用一次，不提供 Dcm Deinit。重启压力测试用于验证已进入的调用身份，不改变生产生命周期顺序。已核对（生成样本）[Dcm]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，由生成的 Ecu_StackBootstrap 承担启动／关闭顺序。不适用（归属：EcuM／BswM 生命周期角色由生成启动器承担）[Dcm]：SWS_BSW_00150、00152 |
| 回调 | TP 的复制／接收入口保留 Dcm SWS 明确规定的 BufReq_ReturnType；完成通知为 void，参数按标准传递。已核对（Dcm API 原文与公开消费者）[Dcm]：SWS_BSW_00172、00173 |
| 配置方式 | 使用 ECUC 工具生成可读配置源，只接纳 VARIANT-PRE-COMPILE，未选择的可选功能拒绝启用。已核对（配置模型）[Dcm]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 主线只允许一个分区；线程安全仍按模块协议验证，不由单 owner 假定替代。已核对（配置模型与源码）[Dcm]：SWS_BSW_00190–00192 |
| 错误分类 | 原文表定义错误分类和类型描述格式；没有实现专有错误。已核对（原文与源码）[Dcm]：SWS_BSW_00073、00125、00144、00209 |
| 开发错误开关 | DcmDevErrorDetect 默认且固定为 false；参数拒绝有源位置测试。已核对（配置模型）[Dcm]：SWS_BSW_00042；不适用（配置：ECUC_Dcm_00823=true）[Dcm]：SWS_BSW_00045、00049、00050、00201、00203、00212、00243、00255 |
| 运行时错误 | Dcm 的接口 timeout／buffer-overflow 类别按其原文表核对。所选同步、静态长度接口不接纳 pending；正常 API 的容量拒绝返回规定的 BufReq／NRC，不伪装成异步 timeout；本模块未检测到需报告的运行时错误时不调用 Det。已核对（所选调用路径与原文条件）[Dcm]：SWS_BSW_00219、00222 |
| 生产错误 | Dcm SWS §7.10.3／7.10.4 明确没有 production／extended production errors；没有 Dem event monitor。已核对（原文）[Dcm]：SWS_BSW_00046、00066、00143、00204–00208 |
| 安全事件 | 开关位于 DcmGeneral，默认 false；主线拒绝启用及事件配置。已核对（配置模型）[Dcm]：SWS_BSW_00250；不适用（配置：ECUC_Dcm_01187=true）[Dcm]：SWS_BSW_00244、00251–00253、00258 |
| 版本 API | DcmVersionInfoApi 默认 false，仅预编译选择。已核对（配置模型）[Dcm]：SWS_BSW_00051、00236；不适用（配置：ECUC_Dcm_00821=true）[Dcm]：SWS_BSW_00052、00064、00164 |
| MetaData | 接收和配置均只接纳非 generic 的物理连接；generic ECU 地址配置拒绝。 不适用（配置：ECUC_Dcm_01081）[Dcm]：SWS_BSW_00239–00242 |
| Post-build variation | 主线拒绝 link-time／post-build 模块变体。 不适用（配置：ECUC_Dcm_01082）[Dcm]：SWS_BSW_00226–00228 |
| 发布版本信息 | 已核对（`Dcm.h`／`Dcm.c` 及 `release_contracts::direct_external_headers_refuse_only_missing_or_wrong_major_minor`）[Dcm]：SWS_BSW_00036；release 4.10.0 已发布，正式 vendor 身份仍为差距 #59[Dcm]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[Dcm]：SWS_BSW_00162 |
| MISRA | 差距 #58[Dcm]：SWS_BSW_00115 |
| 交付描述 Dcm 00001 | 已核对（artifacts::files 从交付公开头补齐真实入口，保留计划所选动态入口；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 与固定 XSD oracle 检查实际交付 BSWMD）[Dcm]：SWS_BSW_00001 |
| 交付描述 Dcm 00002 | 已核对（module_documentation::module 交付 modules/Dcm/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[Dcm]：SWS_BSW_00002 |
| 交付描述 Dcm 00038 | 已核对（artifacts::exclusive_area 与 validate_areas 核对 Dcm 所选区域、实际 SchM Enter/Exit 声明及调用；mismatched_schm_declarations_and_calls_are_rejected_before_delivery 检查错配拒绝）[Dcm]：SWS_BSW_00038 |
| SW-C 服务接口 | DCMServices 服务端口不由本次描述补全提供，保持原支持边界。差距 #187[Dcm]：SWS_BSW_00040 |
| 交付描述 Dcm 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[Dcm]：SWS_BSW_00054 |
| 交付描述 Dcm 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[Dcm]：SWS_BSW_00061 |
| 交付描述 Dcm 00238 | 已核对（artifacts 的 providedModeGroup 与 SwcBswMapping 保留 #186/#213 已交付关系；dcm_entities_declare_both_modes_for_every_possible_publisher 核对两个模式组及发布实体）[Dcm]：SWS_BSW_00238 |
| Symbolic Name Value | Dcm_Cfg.h 导出所选接收／发送／连接／会话及协议符号；Ecu_RuntimeConfig.c 从 PduR_Cfg.h／ComM_Cfg.h 导入对端句柄。已核对（生成头与真实消费者）[Dcm]：SWS_BSW_00200 |

## Dcm 映射头的必需重复处理

Dcm实现的CODE由`generated/include/SchM_Dcm.h、bsw/include/Dcm_ComM.h、bsw/include/Dcm.h、bsw/src/Dcm.c`消费；VAR_CLEARED_UNSPECIFIED由`bsw/src/Dcm.c`消费。这些具体请求与声明／定义的归属沿本模块及对应标准篇核查；不是仅凭头名推定角色。固定MemoryMapping00003／00011／00023（PDF36、41、37）要求完整开闭、撤销请求且不在函数体内包含头。整头guard会吞掉STOP／后续class，保留已处理关键词也违反00011；固定属性仍单独保护，每次检查嵌套／错配及请求／属性撤销。

D4.10为Required（主文32–33／PDF40–41，TC2.2无映射例外），R20.5为Advisory（158／166）；AMD2追加C11不改变当前C99适用性。记录绑定cf5d6fa1的字面指令；本次仅增注释，全部C／预处理token保持。原3ea747fe..9204ed30的请求／头闭包复核及后续对应模块声明映射复核为技术依据；C／C++实际配对、消费、属性撤销和无效／多请求拒绝收据按原head保留。改变类别、前缀、目标、实体或条件须重评。

| MISRA 条目 | 源码范围 | 必需性与风险控制 | 原授权与技术处置 |
| --- | --- | --- | --- |
| MISRA C:2012 Directive 4.10 | `runtime/bsw/include/Dcm_MemMap.h` 中 `@file` | 00003／00011需逐次处理START／STOP；固定属性guard不遮蔽分派主体。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |
| MISRA C:2012 Rule 20.5 | `runtime/bsw/include/Dcm_MemMap.h` 中 `#undef DCM_START_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef DCM_START_SEC_CODE`、`#undef DCM_STOP_SEC_CODE` | 00011要求消费这些请求；保留class／active校验及STOP属性清理。 | 维护者 人类，2026-10-17条件授权；ADR0023；2026-10-27逐处评估，非MISRA符合性批准 |

原授权见[#58决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)。SWS_BSW_00115就近技术注释与上列指令／文件主体对应；内部active／属性清理和R20.1不随之获准。共享源注释不声称新Windows执行、MISRA合法偏离或整体符合；原始诊断、not_assessed、strict refusal及其余00115差距保留。
