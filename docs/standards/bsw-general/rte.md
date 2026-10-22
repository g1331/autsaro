# BSW General：Rte

本篇属于 [BSW General](README.md)，按 Rte 单独核算全部 150 条。依据为固定 SpecificationsARXML 的完整条文，RTE 特定义务见 [RTE](../rte/README.md)，SchM 生命周期见 [SchM](../schm.md)。RTE 是 BSW Scheduler 的实现者；不能把“普通模块不得调用 MainFunction”误用为禁止调度器分派已映射的主函数。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与名字 | RTE 的实现为 Rte.c、Rte_OsService.c，公共头为 Rte.h、Rte_Main.h、Rte_Type.h 和组件/服务头；SchM 使用 SWS_RTE 专门规定的 SchM 前缀。生成器统一检查文件名大小写、公开 producer、参数与实际引用的运行时名字。已核对（composition::check_names、outputs 与 contracts）[Rte]：SWS_BSW_00101–00105、00170、00171 |
| 实现源 | Rte.c 包含 Rte.h、Rte_Com.h、Com.h、Os.h；Rte/SchM 的数据与内部函数保留 static，标准 const CDS/handle 定义在源中，应用声明来自各自 Application Header。Rte_MemMap.h 是 RTE/生成配置前缀的唯一 producer；没有 Dem 调用。已核对（Rte.c、SchM_Main.c 与独立消费者的实际 include 闭包）[Rte]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | 当前 RTE 没有 ISR、interrupt frame 或寄存器访问。OsTask 的代码段来自校验过的 SwAddrMethod；内核、主机线程与时钟属于 OS/目标适配。已核对（Rte.c、SchM_Main.c 和所选单 owner 调度）[Rte]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 当前只接纳预编译配置，C 数据可读且 const；没有 link-time/post-build 参数、外部配置集选择或变体。已核对（configuration、native_configuration 与生成配置）[Rte]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184、00226–00228 |
| 实现头 | Rte.h 自包含并导入 Std_Types/ComStack_Types；应用类型、生命周期、COM callback 声明各在相应标准头。MemMap 为重复包含协议，其余头有唯一 guard；多 Application Header 的专门拒绝按 SWS_RTE 实现。已核对（contracts、rte_types 与 C/C++ 消费者）[Rte]：SWS_BSW_00020、00023、00024、00048、00110、00210、00249 |
| 应用与回调 | Application Header 只进入实际 RTE glue 源，不进入模块公共头；直接 C/S 按标准形状传参。COM 的两个通知返回 void，按 handle 分派；启动顺序和 source owner 在首次分派前建立。已核对（Rte.c、Rte_Com.h、Rte_DcmService.h 和生成启动器）[Rte]：SWS_BSW_00025、00069、00172、00173、00180、00218 |
| Callout | RTE 调用的 SWC runnable 是 SWS_RTE 的组件 entry，原型与种子使用该 SWC 的 MemMap；RTE 不定义普通 BSW callout。已核对（contracts 与 source-derived application slots）[Rte]：SWS_BSW_00135、00136、00254 |
| 类型与表达式 | Rte.c/SchM_Main.c 使用 AUTOSAR 类型；boolean 只赋值、返回及与 TRUE/FALSE 比较。标准 Rte_CDS/Rte_Instance/SchM_ConfigType 名字由 SWS_RTE 专门规定，其他模块类型与输入类型按来源归属；编译器属性限于 MemMap。预处理的 defined 用于标准 header/runnable/section 协议，不是功能开关。已核对（rte、contracts、memmap 的实际输出）[Rte]：SWS_BSW_00120、00121、00123–00125、00127、00142、00146、00147、00186 |
| C99 与 OS | 标准消费方以 C99/pedantic/Werror 编译链接，OS 计数器服务由独立标准绑定调用。RTE/BSW Scheduler 属于通用 OS 服务表允许的调用方。已核对（Rte_OsService.c、Ecu_Config.c、target 与 native 消费者）[Rte]：SWS_BSW_00138、00234、00257 |
| 调度器角色 | RTE/SchM 是 MainFunction 的调用者，调用顺序来自 RteEventToTaskMapping 与 RteBswEventToTaskMapping；普通模块不得自行调度的规则由实际被调模块承担。不适用（归属：被调 BSW 模块的 MainFunction 契约；RTE/SchM 负责调度）[Rte]：SWS_BSW_00133、00037、00153、00154、00156 |
| 独占区 | RTE 接收值只由一个 owner 访问，不增加跨模块锁；Rte/SchM 模式状态只用自身 SCHM_MODE 区，BSWMD 只给实际 Mode/Switch entities 关联 canEnter，BSW 模块各用自己的区。已核对（Rte.c、SchM_Main.c、ADR 0018）[Rte]：SWS_BSW_00038、00134 |
| 生命周期 | StartupHook/ShutdownHook 的生成启动器承担调用者角色，SchM_Init/Start/StartTiming 和 Rte_Start 按阶段执行，关闭先 Rte_Stop 后 SchM_Deinit；生命周期检查拒绝乱序、他 owner 和重复资源分配。已核对（生成闭包与 schm_contract.c）[Rte]：SWS_BSW_00071、00072、00230–00233；不适用（归属：EcuM/BswM 的生命周期角色由 Ecu_StackBootstrap/Retire 承担）[Rte]：SWS_BSW_00150、00152 |
| 配置方式 | ECUC 元数据表达固定官方类型/范围/默认值；主线的 generation 开关在参数处拒绝其他值，不在 emitter 字符串中解释配置。已核对（builtin::rte、configuration、native_configuration）[Rte]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 当前只接纳单核单分区，RTE 数据没有跨 owner 入口。已核对（分区校验与 Ecu_TargetIsOwner）[Rte]：SWS_BSW_00190–00192 |
| 错误分类 | 使用 SWS_RTE 定义的 infrastructure/application 状态，配置和输入诊断在生成前定位；没有自行增加的运行时错误、生产错误或安全事件。已核对（rte_types::STATUSES、Rte.c、SWS_RTE 错误章）[Rte]：SWS_BSW_00073、00144、00209、00125、00219、00222、00204–00208、00143、00046、00066、00244、00250–00253、00258 |
| 开发错误关闭 | RteDevErrorDetect/RteDevErrorDetectUninit 当前均只接纳 false；owner/lifecycle guard 是目标协议的拒绝，不伪装成 DET 开发错误报告。已核对（参数拒绝与 Rte.c 的返回语义）[Rte]：SWS_BSW_00042；不适用（配置：ECUC_Rte_09008=true）[Rte]：SWS_BSW_00255、00201、00203、00045、00243、00049、00212、00050 |
| 版本 API | SchM 版本 API 开关是 RteSchMVersionInfoApi，必填且只接受 false，按 SWS_RTE 的专门名字配置。已核对（builtin 与拒绝测试）[Rte]：SWS_BSW_00051、00236；不适用（配置：ECUC_Rte_09062=true）[Rte]：SWS_BSW_00052、00064、00164 |
| MetaData | RTE 通过 Com_SendSignal/ReceiveSignal 访问信号，不引用或分配 EcuC 全局 PDU 的 MetaData；该存储属于实际 PDU producer。已核对（COM 接缝与 Rte.c）[Rte]：SWS_BSW_00239–00242 |
| BSWMD | 当前实际 RTE entries、逐 API 依赖、MemorySections、static/constant memory、MC NONE、来源配置版本均生成；模式的 SWC-BSW 同步关系由 Dcm 的描述与映射提供。已核对（artifacts、rte_memory 与生成描述测试）[Rte]：SWS_BSW_00001、00040、00238 |
| 跨模块共同事项 | RTE 生成头和实现的适用外部导入检查已补齐；软件实现版本由项目共享 owner 发布，配置版本保留源 DOC-REVISION；完整 published identity 仍由共同整改承接。已核对（`Rte.h`／`Rte.c` 及 `release_contracts::direct_external_headers_refuse_only_missing_or_wrong_major_minor`）[Rte]：SWS_BSW_00036；release 4.10.0 已发布，正式 vendor 身份仍为差距 #59[Rte]：SWS_BSW_00059、00161、00256；已核对（[软件发行政策](README.md#软件发行版本)、`Autsaro_Version.h` 及 `shared_software_owner_propagates_independent_component_upgrades`）[Rte]：SWS_BSW_00162 |
| 交付描述 Rte 00002 | 已核对（module_documentation::module 交付 modules/Rte/README.md、API 与配置分页；descriptions::module_descriptions_include_delivered_public_entries_and_manuals 检查导航、封面、历史、文件、资源、集成、限制与实例入口）[Rte]：SWS_BSW_00002 |
| 交付描述 Rte 00054 | 已核对（module_documentation 从同一 schedule.entities 交付任务、位置、周期与触发源，说明同任务到期项按 position 执行；无独立计划项不虚构调度顺序）[Rte]：SWS_BSW_00054 |
| 交付描述 Rte 00061 | 已核对（module_documentation 复用 DefinitionCatalog 的说明、范围、单位及默认值和 PlanDescription 的值／引用；禁用功能复用 native_configuration::DISABLED_FEATURES；selected_configuration_manual_tracks_the_validated_input_value 检查连贯配置变化）[Rte]：SWS_BSW_00061 |
| Symbolic Name Value | Rte.c 包含 Com_Cfg.h，并在 signal API 与接收回调中使用其唯一生产的 ComConf_ComSignal_*；Rte_Cfg.h 唯一导出 ComUserSignal 的 ComUserCbkHandleId，COM 配置与两个 RTE 回调比较导入同一 RteConf_ComUserSignal_*。已核对（symbolic_headers_and_consumers_preserve_provider_names_and_values、改名／非默认值编译链接与 HostBatch 回归）[Rte]：SWS_BSW_00200 |
| MISRA | 保留标准要求与原始诊断；221 项人工核对、采用代码责任与偏离审批仍未全部完成。差距 #58[Rte]：SWS_BSW_00115 |
