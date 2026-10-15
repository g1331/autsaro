# 标准与规范基线

本目录记录项目固定的标准版次、官方资料在本地的位置、测试预期如何从标准独立定义，以及各模块的义务索引与条款处置。它是查找依据的入口，不是符合性报告：列出的条款表示已定位并用于设计或验收，**不表示所有适用条款已逐条核完**，也不表示实现已经符合。架构与所有权约束见 [architecture/](../architecture/README.md)，决定理由见 [ADR 0002](../adr/0002-fixed-standard-baseline-r24-11.md)、[ADR 0008](../adr/0008-misra-baseline-and-no-deviation.md)。

## 1. 固定基线

| 范围 | 基线 | 说明 |
| --- | --- | --- |
| AUTOSAR | **CP／FO R24-11** | 文件身份以官方 [CP](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256)／[FO](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256) `TR_SpecificationHashes.sha256` 为准；Schema 为 `AUTOSAR_00053.xsd`。R25-11 只作后续升级参考，不混用 |
| ECUC 定义 | `AUTOSAR_CP_MOD_ECUConfigurationParameters.zip` | SHA-256 `df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`（`core/resources/official.json`） |
| XML Schema | `AUTOSAR_FO_MMOD_XMLSchema.zip` | SHA-256 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`（同上） |
| C 语言 | **C99** | MISRA 修订支持 C11／C18 不构成升级编译标准的授权 |
| 编码规范 | **MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2** | 200 Rules＋21 Directives＝221 项；当前无已批准偏离 |
| OSEK OS | OSEK/VDX OS 2.2.3 | R24-11 SWS_OS §7.1 引用的 OSEK 基础义务须一并纳入。已核对 NXP 托管副本（SHA-256 `2ec77d3c4259797f4d0190f938131b641d415d8e0a85ce40a72e4201a9114746`）；ISO 17356-3 全文与后续勘误未核对 |

R24-11 中标记为 DRAFT 的条款（例如 LSduR 全文，`SWS_Com_91019／91020`、`SWS_Rte_91123／91127`、`SWS_COMTYPE_91001`、CanIf `91002`）按固定版本原文实现并如实标注，不宣称为成熟标准条款。

## 2. 官方资料与本地位置

- 官方 PDF、ZIP 与哈希清单只在本地查阅，由 `.gitignore` 排除，不随仓库或产品再分发。目录与核验结果见 [docs/official/README.md](../official/README.md) 与 [docs/official/R24-11/README.md](../official/R24-11/README.md)。
- 下载与核验：`python scripts/collect_official_specs.py R24-11`，按两份官方清单逐项校验，结果写入 `download-status.json`。校验失败或官网缺项不算已收集。
- 开发对照档案的路径与环境变量登记在 `core/resources/official.json`：XSD／MOD（`AUTOSAR_XSD_ARCHIVE`、`AUTOSAR_MOD_ARCHIVE`）供 `official-oracles` 测试检验内置规则；`AUTOSAR_CP_MOD_SpecificationsARXML.zip`（`AUTOSAR_SPECIFICATIONS_ARCHIVE`）是 [§5](#5-条款处置与核算) 条款核算的条目来源。普通配置与源码生成不需要这些档案。
- 官方单份文件的统一入口为 `https://www.autosar.org/fileadmin/standards/R24-11/CP/<文件名>`。
- MISRA 正文需授权取得，不在仓库中。编码指导与 221 项编号目录见项目技能 `.agents/skills/misra-c2012/`（`references/baseline-index.json`）；它是编码辅助，不替代正文或人工评估。
- 合法正文缺失时，只阻塞依赖该正文的标准裁决；记录缺口与解除条件，不凭记忆、旧版本或现有实现补齐。

## 3. 预期从标准独立定义

1. 测试预期从固定版次的官方原文与独立向量定义；不从现有源码、ABI 或资产清单或生成器输出自动生成唯一预期。
2. 对标准的核对与对实现内部一致性的检查分开记录；后者不能证明前者。
3. 公开接口用独立 C consumer 编译、链接并调用固定函数指针，覆盖成功、拒绝、关键状态变化与失败后输出保护；不以函数桩、源码文本断言或纯编译代替行为。
4. 对外使用标准名称的接口须满足标准签名与语义；改名、typedef 或空壳接口不算对齐。
5. 主机行为、XSD 校验、MISRA 检查都不能代替 AUTOSAR 模块符合性；ECUC metadata 数值范围通过也不等于配置语义合法。
6. 修改或扩展 BSW、RTE、OS、配置模型和 C 生成逻辑前，核对所涉模块在所选配置下的公开接口、类型、配置、状态机、错误与回调义务及上下层接缝；超出当前范围或受目标条件阻塞的差距写明依据、影响与退出条件，不只写“主机专属”。
7. MISRA：工具可用、扫描完整、源码诊断与人工评估分别证明；原始诊断保留，采用代码不自动排除；申请偏离前核对适用的 amendment／TC 与实际调用路径，偏离只能由维护者批准。

## 4. 证据分层

以下结论各自需要独立证据，不能由另一项推定：主机接口与逻辑行为；Windows 与 Linux 各自的执行；原生 Tauri／IPC 与安装包；离线接收者复验；MCU 实时性、真实 ISR 与内存段放置；第三方模块互操作；完整规范义务；MISRA 符合性；功能安全与认证。未运行记为未执行，失败记为失败。

## 5. 条款处置与核算

各模块篇是条款处置在文档一侧的记录（[ADR 0017](../adr/0017-requirement-disposition-in-tests-and-standards.md)）。模块整改时，该模块文档在 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 中的全部条目（含 `ECUC_`、`CONSTR`、`NA` 前缀）都须处置。只有紧跟显式标记、且写在对应来源中的编号才计入处置。测试在注释中写 `Verifies: <编号>`。模块篇中写以下几种：`已核对（<核对方式>）：` 记录审阅确认、在所选配置下已满足的条目，核对方式写出具体证据（函数、类型、测试或生成样本对象），尚未实现或移交的部分不能写成已核对；`不适用（配置：<ECUC 参数编号>[=<值>]）：` 记录产品不支持的配置下才适用的条目，理由中的每个参数都须有带 `Rejects: <ECUC 参数编号>` 注释的拒绝测试；`不适用（归属：<承担者>）：` 记录属于其他模块、集成方或配置工具的条目；`差距 #<Issue>：` 记录差距。分组处置可写成表格，但每个编号单元格须以标记开头。`SWS_BSWGeneral` 的条款对每个模块各自成立，其标记在冒号前写模块缩写，如 `Verifies[CanIf]:`、`差距 #59[CanIf]：`；覆盖全部已交付模块的检查或对所有模块成立的结论写 `[*]`。其他模块的标记和不带限定的标记都不替本模块处置。核算用下面的命令现算，结果不入库：

```powershell
uv run --locked python -m autosar_tooling requirements SWS_CANInterface
```

参数为文档名（`AUTOSAR_CP_<文档名>.pdf`）。命令输出各类别的条目数，并列出未处置条目（标注 draft／obsolete）、缺少拒绝测试的配置条件与失效引用。失效引用指带有该文档的前缀、但 R24-11 中没有的编号；未加标记的编号也会检查。三者任一非空即失败；`--stale-only` 只检查失效引用。模块整改完成后，运行 `requirements --update <文档>` 把该文档的编号清单写入 `core/resources/requirements/` 并提交，必需检查随后按清单核算（不带文档参数的 `requirements`，不需要档案）。`SWS_BSWGeneral` 按模块核算：`requirements SWS_BSWGeneral --module <模块缩写>`，整改完成后用 `requirements --update SWS_BSWGeneral --module <模块缩写>` 把模块登记到该文档的清单。一个标记后可列多个完整编号，用 `、`、`,`、`／` 或空格分隔，编号可加反引号；同前缀的后续编号可用 `／`、`,` 续写，或用 `–` 表示范围，如 `不适用（<理由>）：SWS_<Mod>_00001／00003、SWS_<Mod>_00907–00910`（示例中的 `<Mod>` 不构成引用）。

尚未按此整改的行沿用原核定记录：模块上下文中的简写（如“Dcm 00806”“RTE 05088”）指该模块 SWS 的同号条款，不计入处置，其中的完整编号也只是定位记录，整改时改为带标记的完整编号；`ECUC_*`、`CONSTR`／`constr_*`、`TPS_*` 等前缀保持原样。SHA-256 为 2026-10-09 实际取得、与 CP 官方清单匹配的文件身份。

## 6. 模块义务索引

每个模块一篇，记录所用文档的文件身份、已定位条款、义务要点、条款处置和该模块的已知差距。

| 篇 | 范围 |
| --- | --- |
| [方法论、输入角色与模板](methodology-and-templates.md) | TR_Methodology、System／SWC／BSWMD 模板与 ECU Configuration |
| [BSW General](bsw-general/README.md) | 对全部 BSW 模块成立的通用条款 |
| [类型](types.md) | StandardTypes、PlatformTypes、CommunicationStackTypes |
| [MemMap](memmap.md) | 内存段映射 |
| [RTE 与 SchM](rte.md) | RTE 契约、映射、生命周期与 SchM |
| [OS](os.md) | AUTOSAR OS 与 OSEK/VDX OS 2.2.3 |
| [COM](com.md)、[PduR](pdur.md)、[LSduR](lsdur.md) | 信号与 PDU 路由 |
| [CanIf](canif.md)、[Can](can.md)、[CanTp](cantp.md) | CAN 接口、驱动与传输层 |
| [Dcm](dcm/README.md) | 诊断通信 |
| [ComM](comm.md)、[CanSM](cansm.md)、[BswM](bswm.md) | 通信与模式管理 |
| [Det](det.md) | 开发错误跟踪 |
| [多组件剖面](multi-component-profile.md) | 主线剖面的首批支持配置与拒绝边界 |

## 7. 已知未核查项与差距

以下跨模块事项不能由现有记录推定为已完成，也不能以“主机专属”为由延期；各项退出条件是取得合法正文、完成对应模块的全文核对与独立验收。

- **逐条全文审计未完成。** 除 Can、CanIf、LSduR、PduR 与 Com 已逐条处置外，各模块篇中的条款是已定位条款，不是完整适用矩阵；各模块全部必需 API、状态、错误、回调、BSWMD、配置语义与可选条件须在模块整改时闭合，未处置条目以 [§5](#5-条款处置与核算) 的核算命令为准。
- **Dem／NvM／Security**：不在主线程序中，已随旧链路删除（[ADR 0013](../adr/0013-remove-frame-signal-host-chain.md)）；以后加入主线时须先核定标准公共面、存储链与 Security 的项目责任。
- **MISRA**：221 项 assessment 当前均为 `not_assessed`，生成工程源码分析 `passed=false`；固定 addon 的已知限制（R21.8 仍报告 `getenv`，终止函数清单缺 `_Exit`／`quick_exit`，errno／mmap 模型差异）只作核查线索。
- **资料可用性**：部分工作树只有官方资料索引而无原件；R24-11 `TR_ChangeDocumentation.zip` 不在官方清单内且未取得；`AUTOSAR_CP_TR_ModelingShowCases.pdf` 已引用（跨源 ARPackage 片段合法）但未加入已匹配 SHA-256 的文件身份声明。

单个模块的差距写在该模块篇的“已知差距”中。
