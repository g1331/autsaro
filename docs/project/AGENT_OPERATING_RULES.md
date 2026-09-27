# BMad 项目推进规则

## 恢复项目状态

本仓库固定使用 BMad 6.12.0，Codex 技能在 `.agents/skills/`，共用配置在 `_bmad/`。新会话先核对 Git 状态、当前源码、`_bmad-output/planning-artifacts/` 的 PRD/架构/epic、`_bmad-output/implementation-artifacts/sprint-status.yaml` 及对应 story 文件；可用 `bmad-help` 与 `bmad-sprint-planning` 的 status/validate 意图辅助导航。BMad 状态是任务进度唯一来源。产品地图 `.scratch/autosar-platform/map.md` 记录产品决定，`docs/assurance/capabilities.json` 记录能力声明和六道证据门；两者都不能由 story 状态自动推断。历史 `docs/workflow/` 只用于溯源。

“状态如何”只读：对照状态、Git、源码与能力档案，简述可运行行为、已完成和在途 story、下一功能项、未通过的门及阻断，不构建、不修改状态。`sprint-status.yaml` 的推荐动作只是索引；文件与现实冲突时先核实，再走 BMad 修复状态流程。

## 一次推进最多两个 story

“按项目计划继续推进”时，先恢复在途 story；若无在途项，按依赖和产品价值选择就绪 story。优先解决明确阻断当前切片的回归，随后推进能增加可观察 ECU 行为的 story。证据调查并入其所属的产品 story，除非有真实缺陷或支持声明前置门，不连续领取独立审计项。Story 2.1 的规范适用性在就绪检查中列为关注项；开发前先用 `bmad-spec` 结合本地 R24-11 Dem/Dcm/NvM 文档收口，再运行 Build，不猜测规范语义。

主 Agent 对每个 story 执行 `bmad-build`，独立审查仅作只读复核，由主 Agent 修复并核对结论；当前不使用 `bmad-build-auto` 或 BMad Loop。每项记录起始提交、实现和拒绝路径、实际输入与输出、验证命令、规范出处、未验证维度。运行 `python scripts/verify.py --scope all --base <story起始提交>` 与适用的实际主机测试；全量旧债用 `--scope baseline` 单独报告，不弱化规则。若 BMad 技能要求对重大意图空缺或不可逆操作作选择，提供具体推荐并停在该边界；常规技术判断自行完成。

一轮最多完成两个 story；到 epic 边界、重大产品决定或真实阻断即停，并交接已完成内容与下一项。Epic 结束时对合并行为进行端到端验收并运行 `bmad-retrospective`。复盘行动项进入 BMad sprint 文件或后续 story；旧工作流反馈 `WF-007` 已迁入 sprint action item，必须在后续实际桌面验收中观察后再关闭。单次本机问题只写入对应 story，不形成共享任务。

## AUTOSAR 声明与交付

每项新运行能力须界定 CP/FO 版次、模块/配置/变体、目标/工具链、标准 BSW 与主机专属代码边界；列出适用 SWS/MOD 条款及不适用理由、输入与生成物闭包、正反向独立预期。`docs/assurance/capabilities.json` 的六门按组合逐一复核；未通过时保持 `documented_behavior`。主机成功不得表述为实机、第三方互操作、完整 MISRA、功能安全或官方一致性。PDF/ZIP 和 XSD 保持本地，不入库。

验证桌面界面只能在真正隔离的桌面会话或虚拟机进行；不可用则标“原生 UI 未验证”，不占用当前用户桌面。对使用者报告实际运行增量、操作/交付改进、可复现证据、拒绝场景、仍未验证、下一推荐项及需要决定的事。只做本地提交；push、PR、发布及重要产品范围变化按授权边界处理。
