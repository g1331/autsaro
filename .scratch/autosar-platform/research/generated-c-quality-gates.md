# AUTOSAR R24-11 质量门：生成代码与交付证据

本笔记基于 Classic Platform R24-11 与配套 Foundation R24-11 官方规范，为项目质量门提供可追溯依据，不代表 AUTOSAR 定义了完整的软件保证或认证流程。下表页码均为 **PDF 印刷页码**。规范 PDF 是用户提供、被 Git 忽略的本地文件；未复制或重新分发。所有来源均使用从本目录可解析的相对链接。

## 生成代码与配置工件

| 来源主张（规范项；印刷页） | 项目门禁含义 | 适用范围／边界 |
|---|---|---|
| 以 C 实现的 BSW **shall** 符合 MISRA C:2012；仅在技术上合理的例外情况下允许偏离，且须在源代码违规处标识并说明。[`SWS_BSW_00115`]，SWS BSW General，p.37（[本地 PDF](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)）。 | 对适用的 BSW C 执行 MISRA 分析，并维护受控偏离记录。 | 只适用于以 C 编写的 BSW，不是项目所有 C 文件的 AUTOSAR 通用要求。p.38 说明规范示例可能为易读而省略通用要求；示例不构成放宽。 |
| BSW 外部接口绑定 **shall** 符合 ISO/IEC 9899:1999（C99）。[`SWS_BSW_00234`]，SWS BSW General，p.37（同上）。 | 按 C99 检查 BSW 公共接口，并用实际目标工具链编译。 | 条文针对外部接口绑定，本身不是完整的生成源码风格规则；不可擅自扩展为所有应用自有 C 翻译单元。 |
| BSW 实现源文件与公开头文件分别按 `<Mip>[_<Ie>].c` 和 `<Mip>[_<Ie>].h` 命名，至少提供 `<Mip>.h`；模块实现还须随附功能、源码、集成、配置及规范/要求偏离等文档。[`SWS_BSW_00004`] p.21、[`SWS_BSW_00020`] p.26、[`SWS_BSW_00002`] p.19（同上）。 | 对声称标准 BSW 的模块核对文件清单、公开声明及模块文档；缺项不得靠主机构建成功补证。 | 这些条款针对 BSW 模块，不自动约束工具代码或主机专属文件；所查通用条款没有规定统一缩进、括号布局或普通注释的语言。 |
| RTE 对 MISRA 的符合性追溯到 RTE 要求追踪表中的 [`SWS_Rte_01168`] / [`SRS_BSW_00007`]，§1.6，p.41（表续 p.42）。Appendix C 说明生成与非生成的 RTE 代码均应符合 MISRA，并列出生成 RTE 预期例外：无需在生成代码中逐处标为不符合，pp.1336–1337（[本地 PDF](../../../docs/official/R24-11/CP/RTE/AUTOSAR_CP_SWS_RTE.pdf)）。Appendix C 按 §1.1，pp.30–31 明确为信息性内容。 | 将生成 RTE 与模块源代码分开分析；只应用列出的预期例外，避免宽泛豁免掩盖其他违规。 | 适用于 **RTE 实现代码**，包括生成的 RTE；不等同于项目编写的 SWC 应用代码，也不把该例外清单套用到 SWC C。 |
| RTE 规范区分 RTE 与 SWC：RTE 实现 SWC 间 VFB；规范面向 RTE 生成器／具体实现，具体实现细节属厂商特定。`[SWS_Rte_01266]` 规定 RTE 模块定义 SWC 调用 RTE API 时所调用的生成函数。SWS RTE §1.1，p.30；`[SWS_Rte_01266]` §5.3.9.3.2，p.643（同上）。 | 分开界定生成 RTE C、BSW C、SWC 实现 C 的分析对象与责任，不把 SWC 源码归到 RTE 生成器。 | 已查来源未证实对 SWC C 有一项普遍适用的 MISRA 要求；项目可另行采纳，但须标为项目策略。RTE 规范也不定义具体生成器实现细节。 |
| BSW 实现须追溯到通用 BSW 要求及对应模块 SWS 中适用的要求；适用性按各条款条件判断。SWS BSW General §§1.1、6，pp.9、31–36（[本地 PDF](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)）。 | 按模块建立实现／生成行为与相应模块 SWS、通用 SWS 的追溯；明确记录不适用项。 | 不是一份适用于所有模块的生成物检查清单。SWS BSW General 是模块 SWS 的补充，不能单独作为模块规范（§5，p.15）。 |
| BSW 模块 **shall** 提供符合 BSWMD 模板的描述 ARXML。[`SWS_BSW_00001`]，SWS BSW General，p.18（同上）。对经 AUTOSAR 接口访问的 BSW，除 BSWMD 外还 **shall** 有定义 `AtomicSwComponentType` 与 `SwcInternalBehavior` 的 SWCT 工件，供 RTE 生成使用。[`TPS_BSWMDT_04000`]，BSW Module Description Template，p.23（[本地 PDF](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf)）。 | 校验规定的工件是否存在、引用是否连通，并核对 RTE 输入与对应 BSWMD／SWCT 描述。 | 额外 SWCT 条件仅适用于通过 AUTOSAR 接口暴露的 BSW，不是每个模块都必须有。 |
| ECUC 定义、Schema、值描述属于不同模型／文件角色；值描述需符合其模板 Schema，StMD／VSMD 定义需符合 Schema。ECU Configuration §§2.1–2.2，pp.21–25；VSMD 派生自 StMD 时需引用 StMD，[`TPS_ECUC_06076`]，p.36（[本地 PDF](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_ECUConfiguration.pdf)）。 | 分别校验 ARXML Schema、引用及 StMD／VSMD 关系；区分参数定义与 ECU 实例值。 | Schema 校验不证明跨模块语义或生成 C 正确。模型构建期间约束可暂时不满足，但须在规定工作流节点执行（§1，p.19）。 |
| ECUC 支持 pre-compile、link-time、post-build 配置类／变体。模块定义声明支持的变体 [`TPS_ECUC_02096`]，p.37；容器 multiplicity 可随 binding time 改变 [`TPS_ECUC_08000`]，p.39；post-build 变体的 multiplicity 由 [`TPS_ECUC_08013`] 描述，p.40（同上）。 | 对项目声明支持的各 binding time／变体校验配置与引用，并编译／链接计划交付的配置。 | 一个默认配置不能证明所有变体正确。规范不要求项目支持所有变体类别；门禁范围应是实际声明支持的类别。 |
| RTE 生成器须拒绝违反指定模型约束的配置。RTE 规范 §1.6，pp.41–45 将 `SRS_Rte_00018`（拒绝无效配置）追溯到多项要求；例如 [`SWS_Rte_06769`] / [`SWS_Rte_CONSTR_09063`]，p.1287（RTE [本地 PDF](../../../docs/official/R24-11/CP/RTE/AUTOSAR_CP_SWS_RTE.pdf)）。 | 对适用的 RTE 输入约束执行拒绝检查；保留诊断并覆盖无效配置场景。 | 规范要求限于 RTE 生成器及明文定义的约束，不代表 AUTOSAR 规定了适用于所有第三方 ECUC、BSWMD、SWCD、编译器或集成错误的通用验证器。 |
| Memory Mapping 规定段关键字约定 [`SWS_MemMap_00022`]，p.23；BSW 与 SWC **should** 支持列出的 section type [`SWS_MemMap_00038`]，pp.19–20；模块／SWC 专用映射头文件要求见 [`SWS_MemMap_00032`]、[`SWS_MemMap_00029`]，p.14（[本地 PDF](../../../docs/official/R24-11/CP/Memory/AUTOSAR_CP_SWS_MemoryMapping.pdf)）。 | 检查 MemMap 起止配对、配置段名、对象分类／段放置及所选配置的目标链接映射。 | `SWS_MemMap_00038` 用词为 **should**，且允许添加模块特定段。目标相关 pragma／链接行为需目标证据。规范变更历史记载 compiler abstraction 已弃用（MemMap p.2；SWS BSW General p.1–2）；不要把旧式 `Compiler.h` 当成 AUTOSAR 必需项。 |

## 非代码风险与交付证据

以下保留代码以外的交叉风险。凡未由规范明文要求的证据形式，均标注为**项目门禁**，不是 AUTOSAR SHALL。

| 规范主张（条款；印刷页） | 项目证据／门禁建议 | 边界 |
|---|---|---|
| BSW 模块实现 **shall** 提供配置规则与约束，以便在 ECU 配置阶段尽可能进行合理性检查。[`SWS_BSW_00061`]，p.20；模块文档配置说明要求见 [`SWS_BSW_00002`]，p.19（[本地 PDF](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)）。 | **项目门禁：**保留模块文档并执行现有合理性检查；将配置缺陷追溯到模块 SWS／参数约束。 | “where possible”有限定；规范未要求特定验证器、覆盖率或所有约束均可机器执行。 |
| BSW 模块文档 **shall** 说明 scheduled functions 的执行顺序／序列要求。[`SWS_BSW_00054`]，p.20（同上）。 | **项目门禁：**对照模块时序和顺序假设审查调度／任务映射；保留 ECU 目标上的时序分析与测量。 | 规范要求记录执行顺序，不规定通用时限预算或测量方法。 |
| RAM／ROM／stack 占用及配置、平台、编译器和选项信息 **may** 写入 BSW 模块文档。[`SWS_BSW_00002`]，p.19；若实现依赖 MCU，则厂商、系列、型号、相关 stepping 和工具链名称／版本／选项 **shall** 记录。[`SWS_BSW_00003`]，p.20（同上）。 | **项目门禁：**按目标与配置保留实际编译器／链接器版本、选项、目标身份、内存映射／链接输出及资源结果。 | 占用详情是 **may**，不是统一 **shall**。不代表要求 ASIL 认证，也不表示 AUTOSAR 已独立认可项目目标。 |
| ECUC 约束应在适当工作流／binding time 执行；未完成模型在开发期间可暂时不满足约束。ECU Configuration §1，pp.18–19（[本地 PDF](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TPS_ECUConfiguration.pdf)）。该规范说明值描述供生成器使用并最终进入 ECU executable；实际 executable 生成不在其范围内，§1，p.18。 | **项目门禁：**在相应 binding／生成节点分阶段验证；固定工具与输入基线，并要求由相同配置和生成工件构建目标。 | 端到端证据链是项目策略；ECUC 规范明确将工具策略及 executable 生成排除在范围外。 |
| BSW 实现对外部头文件执行 inter-module major／minor AUTOSAR release 版本检查，不一致时 **shall** 报错。[`SWS_BSW_00036`]，SWS BSW General，p.30（[本地 PDF](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf)）。 | **项目门禁：**对每个支持配置和模块组合要求基线版本检查及集成／构建证据。 | 这是 BSW 版本检查要求，不是完整供应链来源证明或签名策略。 |
| AUTOSAR Methodology 涵盖从 VFB 到 ECU executable 的主要步骤，但明确不是完整流程，也不规定组织内具体流程顺序；它是可裁剪的 work-product flow。[`TR_METH_01003`]，p.22；[`TR_METH_01004`]、[`TR_METH_01005`]，p.23（[本地 PDF](../../../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_TR_Methodology.pdf)）。 | **项目门禁：**建立发布清单，将批准的模型／配置、生成源码、构建设置、目标证据与测试结果关联。 | 可复现发布、工件来源记录、干净构建、CI 门槛与签署均属项目选择；上述条款没有把它们规定为具体 AUTOSAR SHALL。 |

## 建议的项目门禁分类（非 AUTOSAR SHALL）

1. **模型／工件：**按 binding time 校验 ARXML Schema、ECUC 约束、BSWMD／SWCT／RTE 关系及受支持变体引用。
2. **生成／代码：**固定生成器与输入；编译各支持配置；分开分析 BSW、RTE、SWC；MISRA 偏离只依据适用条款或明确采纳的项目政策。
3. **集成／目标：**用固定目标编译器／链接器构建；按目标与配置检查版本、MemMap／链接放置及资源预算。
4. **行为／交付：**依据模块 SWS、调度／错误行为及项目安全／安保要求测试；随发布保留配置、构建、测试和目标证据。

已查规范支持上述模块与配置层面的依据，但不能据此宣称通过静态分析或 host build 即已获得“AUTOSAR 认证”、ASIL 合规、安保、完全可移植或可复现发布。项目门禁必须标清来源与适用目标，不应倒写成 AUTOSAR SHALL。
