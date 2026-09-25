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
- 对受支持的 RTE 范围，按 `SWS_Rte_05086` 核对生成阶段的 RTE Basic Software Module Description，并按 `SWS_Rte_05090` 核对其中的生成产物记录；`SWS_Rte_01157` 规定 C/C++ 组件使用的固定 RTE 头文件名 `Rte.h`，不能以存在 `Rte.h` 充当整个 RTE 生成与行为通过的证据。各模块还须结合适用的 ECUC、BSW Module Description 模板和该模块 SWS 核对 API、类型、配置与运行行为；R24-11 已移除旧编译器抽象要求，不将 `Compiler.h` 列为必备产物。
- 每个受支持组合保存需求 ID 与适用/不适用理由、生成源文件/头文件/接口/类型/配置/内存映射及 BSWMD 对照、适用模块的 MISRA 静态分析报告或逐项有理由的偏离记录、指定目标和工具链的构建/链接记录，以及独立预期结果驱动的运行行为证据。XSD 校验与编译器警告检查仅是证据的一部分，不能替代这些检查。

依据：R24-11 [General Specification of Basic Software Modules](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)、[Specification of RTE Software](../../../docs/official/R24-11/CP/RTE/AUTOSAR_CP_SWS_RTE.pdf)、[Specification of ECU Configuration](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_ECUConfiguration.pdf)、[Basic Software Module Description Template](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf)。这四份 PDF 是用户本地提供、被 Git 忽略的参考资料，不随仓库分发。这是后续各组合的验收纪律，不是追认当前主机虚拟 C 已通过完整 AUTOSAR、MISRA 或实机硬件验证。

主机验收只证明该配置在虚拟目标成立，不代表真实芯片、CAN 电气层或中断时序通过。将来选择实机时，针对指定 MCU、板卡和工具链另做驱动与总线证据；涉及 ISO 26262、安全等级、量产可用或 AUTOSAR 官方一致性声明，都需要独立定义并取得适用证据，当前不作此类声明。公开发布前还须解决版权/许可、依赖来源及分发权利，但这不是当前本地研发的发布任务。
