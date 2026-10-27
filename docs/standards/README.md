# 标准与规范基线

本目录记录项目固定的标准版次、官方资料在本地的位置、测试预期如何从标准独立定义，以及各模块的义务索引与条款处置。它是查找依据的入口，不是符合性报告：列出的条款表示已定位并用于设计或验收，**不表示所有适用条款已逐条核完**，也不表示实现已经符合。架构与所有权约束见 [architecture/](../architecture/README.md)，决定理由见 [ADR 0002](../adr/0002-fixed-standard-baseline-r24-11.md)、[ADR 0008](../adr/0008-misra-baseline-and-no-deviation.md)。

## 1. 固定基线

| 范围 | 基线 | 说明 |
| --- | --- | --- |
| AUTOSAR | **CP／FO R24-11** | 文件身份以官方 [CP](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256)／[FO](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256) `TR_SpecificationHashes.sha256` 为准；Schema 为 `AUTOSAR_00053.xsd`。R25-11 只作后续升级参考，不混用 |
| ECUC 定义 | `AUTOSAR_CP_MOD_ECUConfigurationParameters.zip` | SHA-256 `df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`（`core/resources/official.json`） |
| XML Schema | `AUTOSAR_FO_MMOD_XMLSchema.zip` | SHA-256 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`（同上） |
| C 语言 | **C99** | MISRA 修订支持 C11／C18 不构成升级编译标准的授权 |
| 编码规范 | **MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2** | 200 Rules＋21 Directives＝221 项；既有具体批准见 [C 编码规范](../development/c-coding-standard.md#misra-偏离注释)，有界冲突见 [ADR 0023](../adr/0023-autosar-required-conflict-disposition.md)，适用分层、类别与 GRP 见 [ADR 0025](../adr/0025-misra-layered-scope.md) |
| OSEK OS | OSEK/VDX OS 2.2.3 | R24-11 SWS_OS §7.1 引用的 OSEK 基础义务须一并纳入。已核对 NXP 托管副本（SHA-256 `2ec77d3c4259797f4d0190f938131b641d415d8e0a85ce40a72e4201a9114746`）；ISO 17356-3 全文与后续勘误未核对 |

R24-11 中标记为 DRAFT 的条款（例如 LSduR 全文，`SWS_Com_91019／91020`、`SWS_Rte_91123／91127`、`SWS_COMTYPE_91001`、CanIf `91002`）按固定版本原文实现并如实标注，不宣称为成熟标准条款。

## 2. 官方资料与本地位置

- 规范原件保存在仓库外的持久目录，例如 `../autsaro-official-specs`，按标准和版次隔离，并保留来源、官方清单及收到字节摘要。原件与提取全文不提交或随产品分发；临时目录不作为唯一原件位置。目录与核验方式见[官方资料入口](../official/README.md)。
- 官网收集使用 `python scripts/collect_official_specs.py R24-11`，默认输出到 `docs/official/R24-11/` 并按两份官方清单校验。有私有资料库访问权限的维护者也可用该库的 `specs.py restore` 和 `specs.py verify`；校验失败或缺项不算已取得。
- 开发对照档案的默认路径和环境变量见 `core/resources/official.json`：`AUTOSAR_XSD_ARCHIVE`、`AUTOSAR_MOD_ARCHIVE`、`AUTOSAR_SAMPLE_ARCHIVE` 供官方对照测试引用，`AUTOSAR_SPECIFICATIONS_ARCHIVE` 指定条款核算的 SpecificationsARXML ZIP。引用其他位置的副本时使用这些变量；示例持久目录不会自动替代现有默认路径。普通配置、源码生成和无关开发检查不需要先准备这些档案。
- 官方单份文件的入口为 `https://www.autosar.org/fileadmin/standards/R24-11/CP/<文件名>`。
- MISRA 原文使用固定的 C:2012 Third Edition 与 AMD1–AMD4、TC1–TC2 组合，按版次保存并记录来源和字节身份，在评估时引用具体正文位置。项目技能 `.agents/skills/misra-c2012/` 的编码指导及221项目录只作辅助，不能替代完整授权原文、修订合成或逐项人工评估。
- 合法正文缺失时，只阻塞依赖该正文的标准裁决；记录缺口与解除条件，不凭记忆、旧版本或现有实现补齐。

## 3. 预期从标准独立定义

1. 测试预期从固定版次的官方原文与独立向量定义；不从现有源码、ABI 或资产清单或生成器输出自动生成唯一预期。
2. 对标准的核对与对实现内部一致性的检查分开记录；后者不能证明前者。
3. 公开接口用独立 C consumer 编译、链接并调用固定函数指针，覆盖成功、拒绝、关键状态变化与失败后输出保护；不以函数桩、源码文本断言或纯编译代替行为。
4. 对外使用标准名称的接口须满足标准签名与语义，标准允许实现自定的部分按模块职责设计；改名、typedef 或空壳接口不算对齐。主机注入、时钟、输出和测试控制通过明确的适配接口提供，不侵入标准公开契约。标准整改同步修改生成来源、配置、调用方和交付工件，兼容性变更按 Issue 范围或另行授权处理，不用资产摘要更新追认。
5. 主机行为、XSD 校验、MISRA 检查都不能代替 AUTOSAR 模块符合性；ECUC metadata 数值范围通过也不等于配置语义合法：接纳 controller／HOH 等身份前核对连续编号及所选实例数量约束，把非等 ID 机制测试与合法 source profile 验收分开。生成器对未消费参数的拒绝按定义成立：同一定义只在部分对象上生效时（如只对 Rx 信号生效的 `ComTimeout`），须对其余对象显式拒绝，并用注入该参数的拒绝测试确认。
6. 修改或扩展 BSW、RTE、OS、配置模型和 C 生成逻辑前，核对所涉模块在所选配置下的公开接口、类型、配置、状态机、错误与回调义务及上下层接缝；超出当前范围或受目标条件阻塞的差距写明依据、影响与退出条件，不只写“主机专属”。
7. MISRA：先按 [ADR 0025](../adr/0025-misra-layered-scope.md)的四层、类别记录与 R15.5 GRP 分配责任；类别内引用共用依据，不逐处重写必要性。工具可用、扫描完整、源码诊断与人工评估分别证明；原始诊断保留，采用代码不自动排除；按 [ADR 0023](../adr/0023-autosar-required-conflict-disposition.md)执行用户已有有界冲突授权：逐项核对规则／文件／源码行、实际调用闭包、固定 SWS 全文必要性、合法 MISRA 版次及 amendment／TC、风险控制和独立复核，沿模块记录与 SWS_BSW_00115 注释路径处置。项目授权不改变 MISRA 类别、允许条件或符合性；缺原文（#57）或必要人工评估（#58）时保持未核查，不把旧 checker 编号当规范依据。范围外新例外仍由用户决定，既有具体批准的来源和范围不变。

## 4. 证据分层

以下结论各自需要独立证据，不能由另一项推定：主机接口与逻辑行为；Windows 与 Linux 各自的执行；原生 Tauri／IPC 与安装包；离线接收者复验；MCU 实时性、真实 ISR 与内存段放置；第三方模块互操作；完整规范义务；MISRA 符合性；功能安全与认证。未运行记为未执行，失败记为失败。

## 5. 条款处置与核算

各模块篇是条款处置在文档一侧的记录（[ADR 0017](../adr/0017-requirement-disposition-in-tests-and-standards.md)）。模块整改时，该模块文档在 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 中的全部条目（含 `ECUC_`、`CONSTR`、`NA` 前缀）都须处置。只有紧跟显式标记、且写在对应来源中的编号才计入处置。测试在注释中写 `Verifies: <编号>`。模块篇中写以下几种：`已核对（<核对方式>）：` 记录审阅确认、在所选配置下已满足的条目，核对方式写出具体证据（函数、类型、测试或生成样本对象），尚未实现或移交的部分不能写成已核对；`不适用（配置：<ECUC 参数编号>[=<值>]）：` 记录产品不支持的配置下才适用的条目，理由中的每个参数都须有带 `Rejects: <ECUC 参数编号>` 注释的拒绝测试；`不适用（归属：<承担者>）：` 记录属于其他模块、集成方或配置工具的条目；`差距 #<Issue>：` 记录差距。分组处置可写成表格，但每个编号单元格须以标记开头。`SWS_BSWGeneral` 的条款对每个模块各自成立，其标记在冒号前写模块缩写，如 `Verifies[CanIf]:`、`差距 #59[CanIf]：`；覆盖全部已交付模块的检查或对所有模块成立的结论写 `[*]`。其他模块的标记和不带限定的标记都不替本模块处置。核算用下面的命令现算，结果不入库：

```powershell
uv run --locked python -m autosar_tooling requirements SWS_CANInterface
```

参数为文档名（`AUTOSAR_CP_<文档名>.pdf`）。命令输出各类别的条目数，并列出未处置条目（标注 draft／obsolete）、缺少拒绝测试的配置条件与失效引用。失效引用指带有该文档的前缀、但 R24-11 中没有的编号；未加标记的编号也会检查。三者任一非空即失败；`--stale-only` 只检查失效引用。模块整改完成后，运行 `requirements --update <文档>` 把该文档的编号清单写入 `core/resources/requirements/` 并提交，必需检查随后按清单核算（不带文档参数的 `requirements`，不需要档案）。`SWS_BSWGeneral` 按模块核算：`requirements SWS_BSWGeneral --module <模块缩写>`，整改完成后用 `requirements --update SWS_BSWGeneral --module <模块缩写>` 把模块登记到该文档的清单。一个标记后可列多个完整编号，用 `、`、`,`、`／` 或空格分隔，编号可加反引号；同前缀的后续编号可用 `／`、`,` 续写，或用 `–` 表示范围，如 `不适用（<理由>）：SWS_<Mod>_00001／00003、SWS_<Mod>_00907–00910`（示例中的 `<Mod>` 不构成引用）。一个标记只连续计入同一文档、仅以分隔符相连的编号：说明写进标记括号（`已核对（源码：…）：`），引用另一文档时另起标记。格式化工具会把超长注释行折到下一行，使编号脱离 `Verifies:` 标记、不再计入；标记行保持在列宽以内，格式化测试文件后重新核算。

`python -m autosar_tooling coverage --output <目录>` 用同一核算为每个已交付模块计数，生成 README 与[路线图](../roadmap/README.md#module-map)中的架构图（`architecture.svg`、`architecture-full.svg`）；它要求模块表与档案中的 SWS 文档逐份一致。master 上由 `Architecture diagrams` 工作流生成并发布到 GitHub Pages，结果不提交。

尚未按此整改的行沿用原核定记录：模块上下文中的简写（如“Dcm 00806”“RTE 05088”）指该模块 SWS 的同号条款，不计入处置，其中的完整编号也只是定位记录，整改时改为带标记的完整编号；`ECUC_*`、`CONSTR`／`constr_*`、`TPS_*` 等前缀保持原样。SHA-256 为 2026-10-09 实际取得、与 CP 官方清单匹配的文件身份。

## 6. 模块义务索引

每个模块一篇，记录所用文档的文件身份、已定位条款、义务要点、条款处置和该模块的已知差距。

| 篇 | 范围 |
| --- | --- |
| [方法论、输入角色与模板](methodology-and-templates.md) | TR_Methodology、System／SWC／BSWMD 模板与 ECU Configuration |
| [BSW General](bsw-general/README.md) | 对全部 BSW 模块成立的通用条款 |
| [类型](types.md) | StandardTypes、PlatformTypes、CommunicationStackTypes |
| [MemMap](memmap/README.md) | 内存段映射 |
| [RTE 与 SchM](rte.md) | RTE 契约、映射、生命周期与 SchM |
| [SchM](schm.md) | BSW Scheduler 生命周期与独占区的逐条处置 |
| [OS](os/README.md) | AUTOSAR OS 与 OSEK/VDX OS 2.2.3 |
| [COM](com.md)、[PduR](pdur.md)、[LSduR](lsdur.md) | 信号与 PDU 路由 |
| [CanIf](canif.md)、[Can](can.md)、[CanTp](cantp.md) | CAN 接口、驱动与传输层 |
| [Dcm](dcm/README.md) | 诊断通信 |
| [ComM](comm/README.md)、[CanSM](cansm.md)、[BswM](bswm/README.md) | 通信与模式管理 |
| [Det](det.md) | 开发错误跟踪 |
| [多组件剖面](multi-component-profile.md) | 主线剖面的首批支持配置与拒绝边界 |

## 7. 已知未核查项与差距

以下跨模块事项不能由现有记录推定为已完成，也不能以“主机专属”为由延期；各项退出条件是取得合法正文、完成对应模块的全文核对与独立验收。

- **逐条全文审计未完成。** 除 Can、CanIf、CanTp、Dcm、LSduR、PduR、Com、ComM 与 BswM 已逐条处置外，各模块篇中的条款是已定位条款，不是完整适用矩阵；各模块全部必需 API、状态、错误、回调、BSWMD、配置语义与可选条件须在模块整改时闭合，未处置条目以 [§5](#5-条款处置与核算) 的核算命令为准。
- **Dem／NvM／Security**：不在主线程序中，已随旧链路删除（[ADR 0013](../adr/0013-remove-frame-signal-host-chain.md)）；以后加入主线时须先核定标准公共面、存储链与 Security 的项目责任。
- **MISRA**：221 项 assessment 当前均为 `not_assessed`，生成工程源码分析 `passed=false`；固定 addon 的已知限制（R21.8 仍报告 `getenv`，终止函数清单缺 `_Exit`／`quick_exit`，errno／mmap 模型差异）只作核查线索。
- **资料可用性**：部分工作树只有官方资料索引而无原件；R24-11 `TR_ChangeDocumentation.zip` 不在官方清单内且未取得；`AUTOSAR_CP_TR_ModelingShowCases.pdf` 已引用（跨源 ARPackage 片段合法）但未加入已匹配 SHA-256 的文件身份声明。

单个模块的差距写在该模块篇的“已知差距”中。
