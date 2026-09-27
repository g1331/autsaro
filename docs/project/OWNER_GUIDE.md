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

当前长期方向与阶段需求在 `_bmad-output/planning-artifacts/` 的 product brief 和 PRD，近期工作拆在 epics/stories，任务状态在 `_bmad-output/implementation-artifacts/sprint-status.yaml`。学习层目前是 product brief 中的待规划方向，不会仅因旧议题存在就进入开发队列。早期产品地图、决策和研究已归档到 `docs/project/archive/` 与 `docs/research/autosar-platform/`，不再需要你维护第二份产品计划。旧任务卡和证据仍留在 `docs/workflow/` 供追溯；能力是否有资格标为“内部支持”看 `docs/assurance/acceptance-policy.md` 和 `docs/assurance/capabilities.json` 的独立证据门。Story 完成不等于某个 AUTOSAR 模块已完整实现。

Agent 应把规范研究放进具体功能任务，不用一轮轮独立审计替代产品开发。当前做什么须同时核对委托目标、BMad 规划与状态及实际证据，不以本说明中的静态示例为准。

项目目前由你主动唤起 Agent，不会在你离开后后台自动开发或发送通知。Agent 可作日常技术判断；新标准版次、真实芯片、重大兼容性或费用、对外推送与发布，应带着推荐和影响请你决定。
