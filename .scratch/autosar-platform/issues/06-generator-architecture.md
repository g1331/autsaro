Type: grilling
Status: resolved
Blocked by: 01, 02, 05

## 问题

什么配置模型与确定性生成架构，能从首个受支持范围扩展至多个模块、多款 MCU，同时输出独立的 C 源码、配置与集成产物？提出至少两种具体接口方案，比较 Schema/版本处理、模块契约、通用运行实现、硬件适配及生成结果，并决定各自的接口位置和配置拒绝规则。

## 待讨论的两种接口形状

**方案 A：直接从 ARXML 渲染各模块模板。** 初期文件少，但跨模块引用、变型、版本兼容与目标平台差异会散落在模板里；规模扩展后难以判断配置何时有效。

**方案 B：ARXML 与软件配置界面共用版本化的配置模型。** 读取/编辑 → 引用与跨模块语义校验 → 生成计划 → 输出通用 C 实现、项目配置、RTE/集成代码及构建工程。模板只负责输出，硬件差异由具体 MCAL 实现承担。未支持的参数在校验时明确报错，不产生半成品。该方案允许在软件内直接创建配置，不依赖另一款外部配置器。

决策问题：首个交付是否允许在本软件内从空项目创建并编辑 ECU/CAN 配置，而不要求先从别的工具取得 ARXML。

## 技术栈对比（决策前）

- **Python 内核路线**：Python 3.12＋`lxml` 解析/验证 R24-11 ARXML 与 `AUTOSAR_00053.xsd`，构建配置模型、校验并生成 C/H。搭配 Electron＋React＋TypeScript 时，Python 是随应用打包的辅助进程；优点是成熟的 XML 工具链与快速迭代，代价是进程接口、Python 运行环境及打包维护。
- **Rust 内核路线**：Tauri＋React＋TypeScript，Rust 命令直接承载配置模型、校验与生成；C99 仍是目标 ECU 的运行代码。无 Python sidecar。XSD 校验可借助 libxml2 的原生接口，但要在真实 R24-11 ARXML、`AUTOSAR_00053.xsd` 和 Windows 打包上验证可行性；不能仅凭“Rust 能解析 XML”就宣称规范语义已覆盖。
- **共同原则**：从空项目配置与 ARXML 导入共用一份模型，显式校验跨模块约束，确定性生成。首版需交付可用的配置界面和主机虚拟 ECU 闭环；不以命令行或 demo 替代。正式栈一经选择不预设日后重写。

界面目标是现代 Web 风格，并避免不必要的额外后台进程。一体化交付更适合 **Tauri＋Rust 内核**；现成 XML 工具与较快迭代则倾向 **Electron＋Python 内核**。Qt/QML 也可构建现代界面，但不是此处技术栈选择的优先方案。

## 答案

选定 **Tauri＋React/TypeScript 界面、Rust 配置与生成内核、C99 目标端运行代码**；不引入 Python sidecar，也不安排先用 Qt/Electron 再重写的过渡阶段。工具在本机从空项目创建与编辑 ECU/CAN 配置，同时导入、导出与首版 CP/FO R24-11 匹配的 ARXML；不能要求先用另一款配置软件准备输入。

采用方案 B：UI 与 ARXML 文件经同一版本化配置模型，先校验 Schema、引用与跨模块语义，再形成生成计划并确定性输出完整工程。生成过程不得由模板各自解释 ARXML，也不调用 AI 临场补写目标代码。每个已支持目标提供可运行的 C99 实现与所需配置；虚拟目标的 CAN/时间行为和将来的实机 MCAL 分处明确接口位置。未支持的配置、版本或目标必须报告具体错误，不产出假装完整的工程。

**验证要求**：用真实 R24-11 ARXML 和 `AUTOSAR_00053.xsd` 验证 Rust 解析、XSD 校验、导入/导出及 Windows 本地运行；后续打包交付须另行验证。XSD 校验不替代跨模块规则，更不构成 AUTOSAR 符合性声明；技术选型本身不等于验收。

## 2026-09-26 生成闭包复核（保留上述方案 B 与技术栈决策）

生成计划应是**受支持配置与目标的工件所有权契约**，不是“源文件能编译”的清单。输入侧明确 CP/FO 版次与配置文件集合、ECUC 定义/值、必要的系统/SWC/BSW 描述、引用与选定变体；计划侧明确每个模块的配置类和消费方；输出侧逐一列出通用运行实现、生成源/头/配置、RTE/BSW 集成描述、回调与公开接口、类型、适用的 MemMap、目标适配、构建/链接入口及主机专属工件。某项只对特定模块或配置适用时记录条件与不适用理由，不为满足表格而输出空壳；共用运行源码和项目专属生成代码分别标注版本与所有者。实际规范适用性见[生成代码质量门研究](../research/generated-c-quality-gates.md)，其中链接的官方 PDF 仅是本地参考材料。

生成前对**引用、声明与定义、符号可见性及精确函数签名、类型、配置绑定及构建输入**作闭包检查；按选定 ECUC 条件逐项核对读/写/例程等回调，而不是仅检查名称或生成 C 能否由本项目的内部指针调用。ECUC 参数所指回调必须由生成代码或已声明、已验证的集成提供方以匹配接口提供。跨模块、跨文件与变体选择由共用模型/生成计划统一解析，不能让模板或运行时代码各自补猜；缺依赖、冲突或未支持目标时定位并拒绝该组合的生成，不能产生看似完整的工程。生成后用完整输出目录而非单个配置文件比较确定性，并将生成器、运行源码、输入、Schema/目标工具链来源关联至[逐组合验收](07-assurance-gates.md)。

**已观察到的 P0 闭包缺口（不是主机 CAN 冒烟失败的结论）：**[`arxml_render.rs`](../../../core/src/arxml_render.rs) 的诊断 ECUC 将 `DcmDspDataReadFnc` 设为 `Ecu_DcmRead_<index>`，可选 `DcmDspDataWriteFnc` 设为 `Ecu_DcmWrite_<index>`（284–290 行）；配置例程时 `DcmDspStartRoutineFnc=Ecu_DcmRestoreDid` 且选择 `ROUTINE_FNC_NORMAL`（321–335 行）。[`generator.rs`](../../../core/src/generator.rs) 未定义读符号，却把写函数生成为 `static EcuStatus(const uint8_t[4])`、例程函数生成为 `static EcuStatus(void)`（65–95 行）；主机 [`Dcm.c`](../../../runtime/src/Dcm.c) 直接从 `Rte_ReadSignal` 读值，且主机 [`Ecu_Config.h`](../../../runtime/include/Ecu_Config.h) 使用自己的函数指针类型。所选 R24-11 Dcm 对固定长度同步写回调的 [`SWS_Dcm_00794`](../../../docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) 要求 `Std_ReturnType(const uint8* Data, Dcm_NegativeResponseCodeType* ErrorCode)`；对 `ROUTINE_FNC_NORMAL` 的 `SWS_Dcm_01203` 要求含 `OpStatus`、`ErrorCode`，其余记录参数依配置而定。须逐个确认实际选择的读/写/Start 回调的**提供者、外部可见性及适用的精确类型/参数/返回值**；补一个同名读函数也不能解决写/Start 签名和可见性问题。当前只能将对应诊断组合的 ECUC 集成记为未验证，不推断现有主机行为失败，也不预定修复方式。上述 Dcm PDF 是用户本地提供、被 Git 忽略的参考材料，不随仓库分发。

**适用性边界：**同一用户本地 R24-11 [Dcm SWS](../../../docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) 的 `[ECUC_Dcm_01215]`（提取文本 25099–25126 行）把 **`DcmDspRoutineFncSignature` 参数及 `ROUTINE_FNC_NORMAL` 选项**标为 DRAFT；这不等于整个 Dcm 模块、RoutineControl 服务或 `SWS_Dcm_01203` 的 normal 回调签名都标为 DRAFT。现有可选 0x31 主机行为仍可如实描述，但不能因其运行便把该选项计入当前有效的成熟 ECUC 配置能力或宣称已完成 AUTOSAR ECUC 集成；只有明确兼容用途、适用范围并取得该组合的闭包与行为证据后，才能单列这类兼容档案，不预断实现方案。
