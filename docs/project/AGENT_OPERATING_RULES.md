# BMad 在本仓库的使用约定

## 由 BMad 决定任务流转

本仓库固定使用 BMad 6.12.0。新会话先核对 Git、源码、`_bmad-output/planning-artifacts/` 和对应 story 文件，再用 `bmad-sprint-planning` 的 status 操作读取 `_bmad-output/implementation-artifacts/sprint-status.yaml`。通常执行它推荐的下一项：恢复进行中 story、处理 review、开始就绪或 backlog story，或在没有待开发 story 时复盘。一次 `bmad-build` 处理一个目标或 story；开始另一项时使用新会话。不另设连续开发数量、自定义领题排序或独立于 BMad 的任务状态机。

“按项目计划继续推进”和“按 BMad 工作流推进下一步”都表示执行当前最合适的**一个 BMad 工作单元**。“状态如何”只读，不领取任务或修改仓库。若 sprint 状态与 story、Git 或源码证据不一致，先核实，再使用 BMad 的 sprint-status 修复流程，不凭聊天记录或手工改 YAML 猜测完成度。

BMad 的 product brief、PRD、architecture 和 epics 记录当前产品方向与计划；`docs/assurance/acceptance-policy.md` 及 `docs/assurance/capabilities.json` 分别记录内部验收规则、能力声明和证据门，不参与 BMad 任务排序，也不能由 story `done` 自动升级。早期产品决定与调查分别在 `docs/project/archive/autosar-platform/`、`docs/research/autosar-platform/` 供溯源，不形成另一套当前规划。历史 `docs/workflow/` 只用于溯源。重大需求或跨 epic 调整走 BMad PRD、架构、epic/story 或 `bmad-correct-course`，不另建项目管理状态。

## 使用 BMad 开发与收尾

对明确的 story 使用 `bmad-build`：让它调查意图、选择规划深度、实施、复核、修复并记录结果。Build 已包含代码复核；`bmad-code-review` 用于额外检查或 sprint 中确实待审的变更，不对每项交付机械重复。Story 2.1 的 Dem/Dcm/NvM 语义关注项是 Build 的输入，编码前必须查明适用规范与配置边界；若 Build 无法安全收口重要意图缺口，再用 `bmad-spec`、更新 PRD 或修订 story，不把 Spec 固定为所有 story 的前置步骤。

每项记录起始提交、实现与拒绝路径、实际输入输出、验证命令、规范出处和未验证维度。运行 `python scripts/verify.py --scope all --base <story起始提交>` 与适用的主机测试；全量旧债用 `--scope baseline` 单独报告，不弱化规则。若 BMad 技能要求对重大意图空缺或不可逆操作作选择，给出具体推荐；常规技术判断自行完成。

`bmad-retrospective` 用于审视已完成 epic 的合并结果和行动项。Epic 结束时可建议复盘；BMad 状态推荐或使用者要求时运行，但它不自动挡住下一项 backlog story。若发现跨 story 的实际风险，说明证据并建议复盘。旧工作流反馈 `WF-007` 已作为 sprint action item 保留，只有后续相关桌面验收提供观察结果时才更新，不因换流程而虚假关闭。单次本机问题只写入对应 story。

## AUTOSAR 声明与交付

每项新运行能力须界定 CP/FO 版次、模块/配置/变体、目标/工具链、标准 BSW 与主机专属代码边界；列出适用 SWS/MOD 条款及不适用理由、输入与生成物闭包、正反向独立预期。`docs/assurance/capabilities.json` 的六门按组合逐一复核；未通过时保持 `documented_behavior`。主机成功不得表述为实机、第三方互操作、完整 MISRA、功能安全或官方一致性。PDF/ZIP 和 XSD 保持本地，不入库。

验证桌面界面只能在真正隔离的桌面会话或虚拟机进行；不可用则标“原生 UI 未验证”，不占用当前用户桌面。对使用者报告实际运行增量、操作/交付改进、可复现证据、拒绝场景、仍未验证、下一推荐项及需要决定的事。只做本地提交；push、PR、发布及重要产品范围变化按授权边界处理。
