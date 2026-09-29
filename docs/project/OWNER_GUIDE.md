# 怎样用 BMad 推进 Autosar

在仓库中打开 Codex，直接说明要达成的结果，例如“按 BMad 完成 Epic 2”或“继续当前 story”。Agent 应核对现有规划、状态与实际证据，并按委托范围使用 BMad 的规划、开发和复核技能。你无需逐项分派函数或重复上次聊天；如果需求或验收范围还不够明确，先得到具体的缺口和建议。

| 你想做什么 | 直接说 | 你会得到什么 |
| --- | --- | --- |
| 看进度 | “使用 `bmad-sprint-planning` 显示当前状态，并告诉我实际能运行什么。” | 只读的在途、待办、风险和推荐动作；能力声明另行核对。 |
| 推进 story | “按 BMad 完成 Story【编号】。” | 该项开发、验证与复核的实际结果，或明确阻断。 |
| 完成 epic | “按 BMad 完成 Epic【编号】，包括跨 story 集成验收。” | 逐项推进和最终的 epic 验收结论；若规划尚未就绪，先说明缺口与建议。 |
| 不指定任务 | “按 BMad 判断当前最值得推进的工作并执行。” | 基于现有规划和证据选择工作，不把状态文件的下一条机械当成产品优先级。 |
| 改目标 | “我希望用户能【操作】并得到【结果】。请用 BMad 判断现有计划需怎样调整。” | 影响范围、推荐方案和确需你决定的取舍。 |
| 看成果 | “使用 `bmad-walkthrough` 带我看这次交付。” | 用途、值得检查的地方及复现方式。 |
| 阶段复盘 | “使用 `bmad-retrospective` 复盘 Epic【编号】。” | 基于 story、提交和运行证据的结论与行动项。 |

不知道下一步时直接调用 `bmad-help`。`bmad-build` 处理一个明确的开发目标或 story；委托整个 epic 时，Agent 应按 BMad 规划逐项推进，并核对组合结果，而不是把 epic 当成单次 Build。关键意图不清时使用适用的 BMad 规划技能；`bmad-code-review` 可作额外审查，`bmad-retrospective` 用于阶段复盘。这些技能各有工作范围，但仓库不再规定每次委托只能推进一个工作单元或必须在下一项前开新会话。

当前长期方向与阶段需求在 `_bmad-output/planning-artifacts/` 的 product brief 和 PRD，近期工作拆在 epics/stories，任务状态在 `_bmad-output/implementation-artifacts/sprint-status.yaml`。学习层目前是 product brief 中的待规划方向，不会仅因旧议题存在就进入开发队列。早期产品地图、决策和研究已归档到 `docs/project/archive/` 与 `docs/research/autosar-platform/`，不再需要你维护第二份产品计划。旧任务卡和反馈保存在 `docs/workflow/archive/`，历史运行证据保存在 `docs/assurance/evidence/`；能力是否有资格标为“内部支持”看 `docs/assurance/acceptance-policy.md` 和 `docs/assurance/capabilities.json` 的独立证据门。Story 完成不等于某个 AUTOSAR 模块已完整实现。

Agent 应把规范研究放进具体功能任务，不用一轮轮独立审计替代产品开发。当前做什么须同时核对委托目标、BMad 规划与状态及实际证据，不以本说明中的静态示例为准。

项目目前由你主动唤起 Agent，不会在你离开后后台自动开发或发送通知。Agent 可作日常技术判断；新标准版次、真实芯片、重大兼容性或费用、对外推送与发布，应带着推荐和影响请你决定。

## 汽车工程技能怎么用

仓库提供 12 组、共 24 个汽车工程规格与审阅技能，具体清单、依赖、来源和限制见[技能说明](automotive-skills.md)。你仍然可以直接说“按 BMad 完成 Epic 3”或“为 Epic 4 设计验证方案”，Agent 根据实际需要选技能；无需你维护 Excel 或逐个调用技能。没有规格交付需求时，普通开发继续使用 BMad。

| 你想得到的结果 | 可以直接发送的提示词 |
| --- | --- |
| Epic 验证计划 | “使用 `$verification-plan-builder`，依据当前 Epic 3 和 story 验收条件编制验证计划，再使用 `$verification-plan-checklist-reviewer` 审阅。区分文档检查、实际测试和未验证项，不改变任务状态。” |
| 需求与测试的关联 | “使用 `$traceability-matrix-builder`，把当前明确纳入范围的需求、规范依据、设计和测试结果整理成追踪矩阵，再用 `$traceability-matrix-checklist-reviewer` 找出遗漏。注明统计范围，不推算整个 AUTOSAR 的完成率。” |
| 诊断规格 | “使用 `$uds-services-builder` 整理当前主机诊断的实际服务、DID、会话和拒绝契约，再用 `$uds-services-checklist-reviewer` 审阅。依据源码和测试区分已实现、待核实和未支持，不用模板补齐支持声明。” |
| Classic 配置设计 | “依据 Epic 4 当前架构，使用 `$autosar-swc-builder` 和 `$autosar-rte-mapping-builder` 整理参考应用及映射规格，并用对应 checklist-reviewer 审阅；对照 R24-11 标注版次差异和待核实项。” |

这些技能生成规格工作簿和文档审阅报告；报告中可能包含上游固定模板、建议阈值和待确认判断。它们不能单独证明生成代码正确或满足完整 AUTOSAR/MISRA 要求，Agent 应说明实际证据。Agent 根据任务选择适用技能；`$技能名` 是可选的显式指定方式。Codex 自动发现技能变化，若列表未更新可重启 Codex。

## C 编码技能如何使用

直接提出功能或修复需求即可，例如“修复 CanTp 的长度校验”“完成当前 OS story”或“修改生成的 C 回调”。Agent 定位到 C 源码、头文件或 Rust 中的 C 生成模板后，应在设计和编辑前主动读取 [misra-c2012](../../.agents/skills/misra-c2012/SKILL.md)，无需你每次点名或输入 `$misra-c2012`。它与 BMad 开发流程配合使用，纯 Rust/TypeScript 改动及只读状态查询不会因此触发。

该技能已显式允许自动调用；完整性与符合性仍按技能的实际核查范围及规范依据判断，不能把自动使用技能当作 MISRA 验证通过。

## 标准 ECU 输入的检查与保存

在“导入 ARXML”中选择同一输入集合的多份文件，也可展开“直接填写来源路径”，每行填写一份 ARXML 的完整路径。含 ECU_EXTRACT 的集合自动进入“标准输入”；不完整集合也可打开此页查看缺失关系的诊断。原有主机目标仍使用原来的配置、生成和运行入口。

“标准输入”显示同一核心计划实际识别的来源角色和原字节摘要。当前可编辑所选 S/R 接收/发送 CAN ID，以及应用、Com 发送和对应 Alarm 的共同周期；诊断参数、类型、端口及引用保持只读。应用修改后会重新校验整个计划，冲突时恢复原配置。

点击“预览保存”查看受影响文件及保存前后原文，再确认保存；“重开来源”重新读取并检查磁盘输入。未修改文件保持原字节，外部编辑、失效预览及待恢复备份会阻止保存。输入已校验或已保存，都不表示运行工程已经生成。XSD/MOD 仍须在本机合法提供，不属于交付输入包。

后台原生复验使用 `python scripts/epic4_desktop.py --binary <本次桌面构建路径> --evidence docs/assurance/evidence/epic4/standard-input-roundtrip.json`。该脚本创建独立 Windows Desktop，通过 `STARTUPINFO.lpDesktop` 指定子进程归属，核对实际窗口不在输入桌面，不调用 SwitchDesktop。WebView 使用私有数据目录，真实 UI 与原生 Rust IPC 完成编辑、预览、保存、重开及拒绝路径；测试进程由专属 job 关闭。原始截图和桌面句柄记录保留在忽略的 `.scratch/epic4/`，共享证据记录来源摘要与实际验证范围。
