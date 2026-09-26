Type: grilling
Status: resolved
Blocked by: 03, 05, 06, 09

## 问题

对每个受支持的功能与平台组合，内部研发阶段何时算验收通过？区分主机虚拟目标、未来实机和对外发布的证据；明确哪些安全、一致性和量产声明不能从内部测试推导。

证据范围还应包括：真实 R24-11 ARXML 与 `AUTOSAR_00053.xsd` 在 Rust/Windows 的解析与校验、跨文件引用、导入→编辑→保存往返保真、保留项不丢失、不安全编辑阻断及变体依赖下的生成拒绝；不能把单次 XSD 通过当作这些语义的证明。

## 答案

现阶段范围限定为**本地研发，不公开发布或分发**。因此验收门槛是内部可复现的“受支持配置”证据，而不是软件商店、对外版本或营销声明；开源发布时间另行决策。暂不发布也不能把编译通过或 mock 回显当作功能完成。

首个主机虚拟 CAN 闭环内部验收须同时证明：

1. 对已实施的 R24-11 功能与约束记录可追溯的规范条目、明确的支持配置和不支持配置；不把下载了全部 PDF 当成实现符合性。
2. 在实际 Tauri 界面完成从空项目配置与多文件 ARXML 导入/编辑/保存；Rust 使用本地 `AUTOSAR_00053.xsd` 校验结构、解析跨文件引用并检查跨模块语义。未修改文件不被重写，保留项的元素/属性/值/引用不丢失；无法安全编辑时拒绝该次修改或保存，未知变体影响生成时拒绝生成。
3. 对相同配置生成稳定的完整 C99 工程，主机编译、运行两个不同配置的虚拟 ECU；用独立预期报文/信号向量检验位打包、收发和状态变化，而非两个相同实现互相回显。实际演练超时、错误状态与不支持输入的失败路径。
4. 保存可复现的输入、生成物比较结果、构建/运行命令及行为检查记录；静态检查与现有自动测试不替代真实界面和生成工程的端到端使用。

生成 C 工程另按**产物 × 模块 × 受支持配置 × 目标/工具链**逐项验收，而非以全项目一条笼统的“符合 AUTOSAR”结论代替：

- 对用 C 实现的 BSW 模块，核对 `SWS_BSW_00115` 的 MISRA C:2012；仅技术上合理的例外可偏离，偏离须在 C 源码中明确标识并说明理由。核对 `SWS_BSW_00234` 的 BSW 外部接口 C99 约束，以及 `SWS_BSW_00006` 的模块实现源文件包含 `<Mip>_MemMap.h`。若模块有定义为 `const` 的链接时配置参数，按 `SWS_BSW_00013` 核对对应配置源文件；没有该参数时记录不适用，而非强制生成空文件。
- 对声称标准 BSW 的模块，按 `SWS_BSW_00004`、`SWS_BSW_00020` 核对实现源码与公开头文件的命名和声明位置，并按 `SWS_BSW_00002` 核对随实现交付的模块文档及规范/要求偏离说明；不是只看 `.c/.h` 能否编译。源码中的 MISRA 偏离注释须指向实际违规位置和理由；缩进、括号布局、普通注释的语言由项目风格确定，不能伪称为无条件适用的 AUTOSAR SHALL。主机专属文件与生成的 BSW/RTE 工件分别记录角色，不把某一类的义务直接套给全部 C 文件。
- 对受支持的 RTE 范围，按 `SWS_Rte_05086` 核对生成阶段的 RTE Basic Software Module Description，并按 `SWS_Rte_05090` 核对其中的生成产物记录；`SWS_Rte_01157` 规定 C/C++ 组件使用的固定 RTE 头文件名 `Rte.h`，不能以存在 `Rte.h` 充当整个 RTE 生成与行为通过的证据。各模块还须结合适用的 ECUC、BSW Module Description 模板和该模块 SWS 核对 API、类型、配置与运行行为；R24-11 已移除旧编译器抽象要求，不将 `Compiler.h` 列为必备产物。
- 每个受支持组合保存需求 ID 与适用/不适用理由、生成源文件/头文件/接口/类型/配置/内存映射及 BSWMD 对照、适用模块的 MISRA 静态分析报告或逐项有理由的偏离记录、指定目标和工具链的构建/链接记录，以及独立预期结果驱动的运行行为证据。XSD 校验与编译器警告检查仅是证据的一部分，不能替代这些检查。

依据：R24-11 [General Specification of Basic Software Modules](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)、[Specification of RTE Software](../../../docs/official/R24-11/CP/RTE/AUTOSAR_CP_SWS_RTE.pdf)、[Specification of ECU Configuration](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_ECUConfiguration.pdf)、[Basic Software Module Description Template](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf)。这四份 PDF 是用户本地提供、被 Git 忽略的参考资料，不随仓库分发。这是后续各组合的验收纪律，不是追认当前主机虚拟 C 已通过完整 AUTOSAR、MISRA 或实机硬件验证。

主机验收只证明该配置在虚拟目标成立，不代表真实芯片、CAN 电气层或中断时序通过。将来选择实机时，针对指定 MCU、板卡和工具链另做驱动与总线证据；涉及 ISO 26262、安全等级、量产可用或 AUTOSAR 官方一致性声明，都需要独立定义并取得适用证据，当前不作此类声明。公开发布前还须解决版权/许可、依赖来源及分发权利，但这不是当前本地研发的发布任务。

## 2026-09-26 跨层质量档案与阻断门（补充原有验收决定）

以下是**项目内部受支持声明的证据规则**，不是 AUTOSAR 提供的一套通用认证流程；未执行的检查不得写“通过”。一条档案对应**CP/FO 版次 × 模块/功能及配置子集/选定变体 × 目标/板卡及工具链**；其字段和值可随阶段扩充，但不得把主机证据复用到另一芯片或默认配置扩展成所有变体。档案需区分“决策已定、实现已具备、证据待审/不通过/通过、可对外发布”及审查人、日期、复现入口与证据位置；只有该组合全部适用门槛的证据经审查通过，才可作对应范围的内部支持表述。既有“主机可运行”事实保留，不能无证据追认完整 AUTOSAR、MISRA、实机或安全通过。

现有 [`README.md`](../../../README.md) 的“当前支持”描述的是**已实现的有界主机行为**，并非本节新增的逐组合完整保证档案已验收；新增门槛约束后续证据与声明升级，不否认已有主机运行结果，也不无证据追溯重标历史状态。

| 档案层 | 最低可复核内容与未通过时的处理 |
| --- | --- |
| 声明与依据 | 记录产品可观察结果、模块和接口边界、支持/拒绝的参数与变体、所依赖的模块及适用 R24-11 需求 ID；逐项给出适用证据或有条件的 N/A 理由。规范条款、项目额外质量门、主机专属行为分列，避免把项目流程说成规范 SHALL。 |
| 输入与往返 | 保存配置文件集合、来源和版次/Schema 基线、必要 ECUC 定义与值、系统/SWC/BSW 描述、所选变体及外部依赖；跨文件引用、类型/DEST、编辑后保留项、保存/回滚和版本拒绝按[项目工件](09-project-artifacts.md)验收。含未知有效内容不等于全项目禁用，影响生成而未解析的依赖必须拒绝。 |
| 工件闭包 | 按[生成器契约](06-generator-architecture.md)对生成/复用的源码、头文件、回调/符号及签名、公开 API、类型、ECUC/BSWMD/条件适用的 SWCT、RTE 工件、配置类、MemMap 段与目标适配建立生产者—消费者对照；引用缺失、接口不符或伪造空实现则该组合不通过。记录完整输出清单及复现比较，不只比较 `Ecu_Config.c`。 |
| 构建与静态质量 | 固定生成器/通用源码版本、工具链版本和构建选项，保留指定目标实际编译**及链接**证据和所用输入/输出标识。仅对适用的 BSW C、生成 RTE C 及各模块的适用要求执行静态分析；报告缺陷和有依据的受控偏离，不把 SWC 应用 C 一概算作 RTE，也不把“警告为零”当成 MISRA 合格。内存段/链接放置按目标证实。 |
| 独立行为与界限 | 用非同一打包/解析实现推导的预期报文、状态和适用的服务响应验证正向、边界、拒绝及该配置实际具备的故障路径（如超时、注入故障、持久存储损坏/失败、停机/重启与恢复）；检查错误不会被显示为有效数据或虚假成功，不强制 CAN-only 档案证明未配置的诊断/NvM。明确测试器/对端版本及互操作边界；按目标和所声明的保证定义时序、RAM/ROM/栈、总线与缓冲等适用资源预算并测量核对。若实机目标声明需要时序或资源保证而预算尚未定义、测量尚未完成，该组合不得通过支持验收；仅对确实不适用的主机/配置项记录具体 N/A 理由，不臆造统一数值或凭虚拟时间推定实机时限。 |

**产品用户可完成的流程（未来验收，不追认当前 UI）：**用真实多文件 ECU 项目从新建/导入开始，让非实现者在工作区识别“可编辑/已保留但不生成/影响有效配置而阻断”的对象和受支持的版本、变体、目标组合；修改配置后核对保存前后文件与生成工程的可查看差异，不发生数据丢失、静默类型/位序/参数强制转换或暗选变体。阻断应能定位文件、对象/路径、适用需求或项目规则、原因和可采取的补救步骤；不适用的需求不伪造要求 ID。用户应能以交付目录内明确的依赖、配置、构建方法与产物独立构建/运行或交给另一使用者，对照所声明的预期信号、适用时的诊断报文和错误状态观察结果，再关闭重开项目、复现输入—生成物—结果。验收记录需逐步标清“ARXML 已保存/校验、工程已生成、主机已构建、主机行为已验证、指定实机已验证、适用规范义务证据已审、获准发布”的差别；绿色构建或虚拟测试不能被界面文字、交接清单或支持矩阵误读成后续状态。此段规定可观察结果，不指定第 10 议题之外的新 UI 布局。

至少以**与所声明配置相符的代表性正向和负向用户场景**验证该流程：其一从真实多文件导入并编辑受支持的 CAN 帧/信号，预览生成差异、完成离线交接，由另一执行者按记录构建运行并与独立预期比对，重开后复现；仅当该档案声明诊断能力时，才增加相应诊断配置、报文/服务及重启验证。其二按该档案的适用输入与失败路径注入版本不符、保留项反向引用或未决变体、保存中断或外部改动、生成/构建失败并观察拒绝位置、可恢复文件/备份、旧产物与新失败状态的区分；仅诊断档案再覆盖诊断故障恢复。CAN-only 档案不得因未执行不适用的诊断场景而被阻断，但仍须有正向交接与至少一条适用的负向/恢复路径。具体测试数据与证据位置随组合记录，不能用只有点击成功的演示替代失败路径。

**失败关闭与证据升级：**生成计划无法证明输入、变体或符号/配置闭包时，不生成该声称的完整组合；生成/构建已成功但静态、集成、独立行为或目标证据缺失时，保留已观察结果，档案标为待审/未通过，不升级为“受支持组合”。阶段退出只按已声明的组合与能力办理，公开发布另经许可、来源、交付包、维护承诺和适用声明核定，不能借用内部绿灯。对于依赖 MCU 的 BSW，目标及工具链信息依适用的 `SWS_BSW_00003` 记录；内存占用详表不能冒称 `SWS_BSW_00002` 的普遍 SHALL，资源预算是项目按目标定义的门。

**主机 DCM 回调闭包已修复；第三方 ECUC 集成仍未验证：**历史检查记录了主机生成器缺少读回调、写/例程定义为不可见且签名不匹配，以及运行时直接读 RTE 的缺口；这些是历史发现，不是当前行为。`6190260` 已闭合主机读/写回调：当前 [`generator.rs`](../../../core/src/generator.rs#L67-L104) 生成并导出 `Std_ReturnType Ecu_DcmRead_<index>(uint8_t *data)` 和可选 `Std_ReturnType Ecu_DcmWrite_<index>(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code)`，并把它们接入诊断配置；主机 [`Dcm.c`](../../../runtime/src/Dcm.c#L97-L126) 通过配置回调表调用。主机 [`Ecu_DcmCallbackTypes.h`](../../../runtime/include/Ecu_DcmCallbackTypes.h) 明确声明其类型别名只是 host-target ABI subset，不是 AUTOSAR `Std_Types.h` 或 `Rte_Dcm_Type.h`。因此，此闭包不证明第三方 Dcm ECUC/ABI、目标链接或集成行为互操作；这些仍须按适用的 ECUC MOD/SWS 与具体目标独立核验。本地 R24-11 [Dcm SWS](../../../docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) 中 `SWS_Dcm_00794` 的同步写回调签名及 `SWS_Dcm_01203` 对 normal Start 的 `OpStatus`/`ErrorCode` 要求仍是相关标准配置的核对依据，不因主机闭包而豁免。

**DRAFT 选项的声明隔离：**本地 R24-11 [Dcm SWS `[ECUC_Dcm_01215]`](../../../docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf)（提取文本 25099–25126 行）仅将 `DcmDspRoutineFncSignature` 参数及 `ROUTINE_FNC_NORMAL` 选项标为 DRAFT，不把 Dcm 模块、整个 RoutineControl 服务或 `SWS_Dcm_01203` 的回调签名整体标为 DRAFT。历史主机 0x31 路径曾被编码进 Dcm ECUC；`6ddcab9` 将其移出标准 Dcm Routine ECUC。当前 [`arxml_render.rs`](../../../core/src/arxml_render.rs#L333-L346) 只在工具专用 ADMIN-DATA 记录主机 RID/会话，不生成 `DcmDspRoutine` 或 `DcmDspRoutineFncSignature`/`ROUTINE_FNC_NORMAL`；主机 [`generator.rs`](../../../core/src/generator.rs#L106-L118) 和 [`Dcm.c`](../../../runtime/src/Dcm.c#L127-L151) 仍提供有界的 0x31 reset 行为。这不构成标准 ECUC RoutineControl 或第三方 Dcm 互操作证据；若未来配置标准 Routine，仍须把该 DRAFT 参数/选项按其状态隔离，不将 DRAFT 扩大解释为整个模块或服务的状态。

适用条款与 SHALL/SHOULD/MAY 限定参见[生成代码质量门研究](../research/generated-c-quality-gates.md)及其链接的本地、Git 忽略的 R24-11 官方 PDF。尤其 `SWS_BSW_00001` 的 BSWMD、条件性的 `TPS_BSWMDT_04000` SWCT、`SWS_BSW_00036` 版本检查及 `SWS_BSW_00054` 调度顺序说明可纳入适用性核对；`SWS_BSW_00013` 的配置源仅在其条件成立时要求。生成 RTE 的 MISRA 例外与 SWC C 的责任不可混淆，不增设无依据的 `Compiler.h` 或空配置源。

## 2026-09-29 P0：第三方 CanIf/CAN 与生命周期互操作前置条件

在宣称第三方 CanIf/CAN 互操作，或 EcuM→BswM→ComM→CanSM 生命周期剖面可互操作之前，必须先按 R24-11 ECUC MOD 补齐并逐引用验证 CanIf/CAN 配置闭包。该门槛是配置/声明的 P0 阻断条件，不是把 CAN 虚拟主机阶段改判为完成，也不表示相关实现或证据已经通过。

- **CanIf 根与控制器/驱动：**MOD 要求 CanIf 的 `CanIfDispatchCfg`、`CanIfInitCfg`（含必需 `CanIfInitCfgSet`）、`CanIfPrivateCfg`、`CanIfPublicCfg` 各为 1..1；`CanIfCtrlDrvCfg` 为 1..*，每个驱动配置须提供 `CanIfCtrlDrvInitHohConfigRef`（指向 `CanIfInitHohCfg`）和 `CanIfCtrlDrvNameRef`（指向 `Can/CanGeneral`）。每个 `CanIfCtrlCfg` 还要求 `CanIfCtrlId`、`CanIfCtrlWakeupSupport`、`CanIfCtrlCanCtrlRef`；后者指向 `Can/CanConfigSet/CanController`。MOD：[`AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip)，ARXML selectors `:20750-21822`、`:21822-22108`、`:24189-24514`。
- **HOH 与缓冲：**被驱动配置引用的 `CanIfInitHohCfg` 必须闭合实际 HRH/HTH；HOH 关联控制器并引用 CAN Driver 的 HardwareObject。Rx PDU 的 `CanIfRxPduHrhIdRef` 必须指向 `CanIfHrhCfg`；Tx PDU 的 `CanIfTxPduBufferRef` 必须指向 `CanIfBufferCfg`。这些不是系统 PDU 名称的别名。MOD selectors：`:21822-22781`、`:22013-22108`、`:23420-23910`。
- **必需 PDU 配置：**对每个 Tx PDU，除全局 `CanIfTxPduRef` 外，还要提供 1..1 `CanIfTxPduBufferRef`、`CanIfTxPduType`、`CanIfTxPduReadNotifyStatus`、`CanIfTxPduTruncation`。对每个 Rx PDU，除全局 `CanIfRxPduRef` 外，还要提供 1..1 `CanIfRxPduHrhIdRef`、`CanIfRxPduDataLengthCheck`、`CanIfRxPduReadData`、`CanIfRxPduReadNotifyStatus`。MOD selectors：Rx `:22782-23212`，Tx `:23420-23910`。
- **Can 模块目标必须真实存在于配置模型中：**CanIf 的驱动名/控制器引用分别要求 `Can/CanGeneral` 与 `Can/CanConfigSet/CanController`；Can Driver 的 `CanHardwareObject` 又要求 `CanControllerRef` 指向该控制器。EcuC `Pdu` 只能解决 COM 栈全局 PDU 标识，不会替代 Can Driver、控制器或 HOH。MOD selector：`:18999-19703`（CanController、CanHardwareObject/CanControllerRef），并见上述 CanIf `CanIfCtrlDrvNameRef`/`CanIfCtrlCanCtrlRef`。

当前生成器为**固定的 11 位主机虚拟剖面**输出 Mcu 时钟参考点、Can 控制器/硬件对象、CanIf 根/驱动/HOH/零容量缓冲及 PDU 配置；导入器拒绝缺失或改动这些受支持绑定的输入。集成测试从本地 R24-11 MOD 读取必需重数、参数范围和引用目标，核对生成 ARXML，并对断开的时钟/HOH/缓冲引用做拒绝测试。这关闭了该主机剖面的配置结构缺口，不证明虚拟基地址、波特率与真实 MCU 匹配，也不证明主机 C API 符合第三方 CanIf/Can ABI；第三方互操作的 P0 档案仍待实际对端、目标与模块 SWS 证据。引用上述 MOD 不把可选参数或整个 CanIf/Can 模块标为 DRAFT/OBSOLETE。

历史提交 `c7d08a8` **只关闭 COM+EcuC 全局 PDU 子集**；本次固定主机剖面的 CanIf/CAN/Mcu 配置增量另行核对，不能追溯归功于该提交。EcuM、BswM、ComM、CanSM 仍未建模；生命周期链仍须分别完成这些模块的 ECUC/SWS 配置引用与端到端行为验证。

`AUTOSAR_00053.xsd` 通过也不足以证明该 ECUC 闭包有效：XSD 结构通过不等于引用目标符合 ECUC MOD 的 `DESTINATION-REF`，也不等于模块定义要求的容器/参数最小重数已满足。对拟声明的组合必须额外按 R24-11 MOD 检查上述引用解析和必需重数；缺项时档案保持“未验证/不支持此互操作声明”，不得由 host build 或虚拟冒烟升级。该补充只细化既有逐组合门槛，不改本地图的阶段状态或引入进度百分比。
