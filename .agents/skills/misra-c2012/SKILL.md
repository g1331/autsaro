---
name: misra-c2012
description: '本仓库 C 开发时主动使用：编写或修复 C 源码、头文件、运行时模块、Rust 中的 C 生成模板及生成 C 工件。任务即使只说实现功能或推进 story，只要实际涉及这些代码也适用。按 MISRA C:2012 修订基线提供全部 221 项编码与核查指导；纯 Rust/TypeScript 改动或规格工作簿不触发。'
---

# MISRA C:2012 编码

把 MISRA 约束用于设计和实现，交付符合接口契约、可构建且经过实际检查的 C 代码。本技能补充仓库开发流程，不替代 BMad、验收规则或正式规范。

## 主动使用

根据任务的实际实现范围选择本技能，不要求用户提到 MISRA、技能名或 `$misra-c2012`。功能、story 或调试任务初期尚未确定语言时先定位实现；一旦确认会编写或修改 C 源码、头文件或 C 生成逻辑，就在设计和编辑前读取本技能及相关指导，与 BMad 开发流程一并使用。纯 Rust/TypeScript 改动、只读状态查询及规格工作簿任务不因此触发。

## 基线与依据

- 默认基线为 **MISRA C:2012 Third Edition + AMD1–AMD4 + TC1–TC2**；若使用 First Revision，辨明其中已合入的 AMD1/TC1。本仓库语言目标仍是 **C99**，修订支持 C11/C18 不构成升级编译标准的授权。
- 开始时读取 [规范基线与来源](references/standard-baseline.md)，确定本次代码角色、规范版本、可用原文和分析器覆盖范围。精确规则编号、分类、例外与判定以适用的授权规范原文为准；不凭记忆补造。
- MISRA C:2023 是 C:2012 修订链的合订版；C:2025 是后续版次。用户指定 C:2012 时保留该基线。任务明确要求迁移时，先核对版本差异和接口影响，不能只改标准名称。
- 无可用规范原文时仍完成可验证的编码、构建与行为检查；明确尚未完成逐条规范核对，不声称完整符合。不得购买、复制进仓库或再分发受许可限制的规范来补齐材料。

## 实现前定位

读取当前任务、相关接口、调用方和构建参数，及仓库的 `AGENTS.md`、`runtime/README.md`、`docs/assurance/acceptance-policy.md` 中与改动有关的约束。先确定边界，再修改代码：

- 区分 BSW、RTE、SWC 应用、生成配置与主机适配/测试代码；按角色核对要求，不把 AUTOSAR BSW 的所有义务套给全部 C 文件。
- 对生成 C 的修改同时定位 `core/src/generator.rs` 等实际生成来源、引用的运行时源码、配置输入及交付头文件。修复生成来源并验证重新生成的工件，不能只修一份输出文件。
- 保留公开 ABI、回调签名、错误语义和 C99 契约。内部实现采用更窄的可见性；不能为了扫描变绿，把外部使用的接口改成 `static` 或删除必要参数。
- 核对项目现有规则重分类计划与偏离记录；不存在时使用规范的原始类别，不自行降低约束，也不擅自追认历史代码为符合。

## 编写与修改

本技能的完整编号覆盖为 **200 Rules + 21 Directives = 221 项**，依据 [锁定目录](references/baseline-index.json) 校验。每项有有效类别、新增版次、适用条件、原创编码动作、核查方式和逐项参考链接。先逐组核对本次任务的适用性，阅读全部相关行；不能只按工具报告的条目选规则：

| 读取范围 | 入口 |
| --- | --- |
| 全项目设计、需求追踪、错误处理、资源与并发 | [全部 21 条指令](references/directives.md) |
| C 语言、未使用代码、注释、命名、位域和常量 | [R1–R7](references/language.md) |
| 声明、ABI、链接、限定符与初始化 | [R8–R9](references/declarations.md) |
| essential type、转换、表达式、求值与循环条件 | [R10–R14](references/expressions.md) |
| 分支、switch、函数、参数、返回值与调用图 | [R15–R17](references/control-flow.md) |
| 指针、数组、重叠内存与预处理 | [R18–R20](references/pointers-preprocessing.md) |
| 标准库及其参数、内部存储与同步接口 | [R21](references/libraries.md) |
| 文件、errno、线程生命周期和泛型选择 | [R22–R23](references/resources-generics.md) |

条件列只帮助定位构造，不能自动判定不适用或通过。C99 目标仍须考虑 AMD 对既有功能的更新；C11 专有构造没有使用时按源码/配置证明其不适用，不能一概跳过所有新增规则。使用 [修订与例外核对](references/revisions-and-exceptions.md) 处理容易误判的情况，并参考 [编码检查要点](references/coding-checklist.md) 中的实现示例。

完整编号覆盖与规范全文是不同层次：这些逐项动作不是受版权保护原文的翻译副本，也不完整复刻全部 amplification、例外、理由和例子。遇到边界判断时跟随该项参考，再核对授权原文；没有原文不能声称已完成全文核对。

采用 C99、四空格缩进、项目既有模块命名与类型；公开 `runtime/include/` 接口按项目 Doxygen 契约说明参数、范围、所有权与错误结果。按最小相关范围修改，避免全仓重排及无关历史问题清理。

新代码优先消除违规来源。不能用无依据的强制转换、无效初始化、吞错、关闭规则或全局 suppression 掩盖问题。必要的主机 I/O、平台 API 或标准接口限制，按下列偏离流程处理。

## 分类与偏离

- **Mandatory**：不允许偏离；消除违反或如实说明仍阻塞规范验收。
- **Required**：仅允许有依据、限定范围并经项目责任人批准的偏离。Agent 可以准备偏离草案，不能自行写成“已批准”。
- **Advisory**：尽实际可行性遵守，保留违反及理由；若项目已提升其类别，按提升后的类别处理。不要把建议一律写成无条件禁令。
- “不适用”“工具误报”“批准偏离”“尚未核实”分开记录；不适用和误报需要源码、契约或目标条件依据。
- 在仓库现有证据位置记录必要偏离：标识、标准版本、已核实规则与有效类别、文件/符号及配置范围、技术理由、替代方案、风险与限制、验证依据、批准状态。BSW 源码在实际违规处注明标识和理由，并链接记录；重复实例可引用同一份有明确范围的记录。
- suppression 只在工具确实支持、误报或偏离依据已确认时局部使用，保留理由与记录关联。注释或 suppression 本身不构成批准。标准无关的格式习惯不要标成 MISRA 规则。

## 验证与交付

目录工具从仓库根目录调用，普通查询不写文件：

```powershell
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py validate
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py query --id R10.8 --id R17.5
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py query --group libraries
```

需完整 MISRA 评估或符合性声明时，在既有任务证据位置建立/复用 221 项核查矩阵，而不是另设任务体系：

```powershell
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py matrix --output <本次证据目录中的新矩阵.json>
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py check-matrix <实际矩阵.json>
```

矩阵初始全部 `not_assessed`；对每项记录适用性、有效分类、结果、理由、规范依据、证据和必要批准。不适用需证据，Required/Mandatory 违规不可遗留为通过，Mandatory 不接受偏离。工具只校验目录/记账完整性，不验证证据内容，也不认证 C 代码；矩阵校验通过后仍须技术审查。普通局部改动按实际范围保留结论，不强制制造一次全项目审计。

使用现有后台检查路径，根据本次变更运行必要检查；不启动用户桌面的可见测试窗口：

1. 验证修改的 C 在实际目标参数下编译、链接，并运行对应成功与关键拒绝路径。生成器改动要重新生成代表性工程，使用其交付头文件及实际宏配置，构建并运行相关主机行为测试。
2. 读取当前 `scripts/quality.py`、`scripts/quality_baseline.py` 与 `scripts/verify.py` 的参数和范围后执行适用检查。增量门为 `python scripts/verify.py --scope all --base <本次起始提交>`；完整基线入口为 `python scripts/verify.py --scope baseline --generated-dir <实际生成工程目录>`。完整基线允许如实报红，不为消除历史失败扩大任务或削弱门禁。
3. 仓库 Cppcheck MISRA 扫描是**部分覆盖**，当前基线脚本只选择特定 BSW/RTE 源码与生成的 `Ecu_Config.c`；须核对本次改动是否在扫描范围内，遗漏文件补做适用分析。检查工具版本、规则支持、include 顺序、平台模型和宏配置；不把零诊断当成 AMD1–AMD4/TC1–TC2 的完整验证。
4. 人工检查工具未覆盖的适用规则/指令、跨翻译单元问题、目标相关假设及偏离限制；未检查项保持未检查。只有相应范围全部适用项与偏离批准状态有依据时，才能作限定范围的符合声明。

交付简要说明代码/生成结果变化、采用基线、实际检查范围与结果、必要偏离及未验证事项。BMad 状态和能力声明仍按现有流程管理；技能执行成功不能自动升级能力档案，也不证明实机、AUTOSAR 或安全认证。
