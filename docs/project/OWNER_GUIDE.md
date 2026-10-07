# 可选的 Agent 与 BMad 协作

普通开发从 [贡献指南](../../CONTRIBUTING.md) 开始，人和 Agent 使用同一套命令。Codex、BMad 和工作簿技能是可选协作方式，不是安装、修复或提交 PR 的前置条件。

## 委托任务

可以描述明确的问题与预期行为，或显式委托“按 BMad 完成 Story/Epic”。Agent 先核对当前 Git、源码、规划和实际验证范围，再选择所需技能。普通修复按贡献流程执行；已有 story 的结果记回对应工件，不另建台账。

| 任务 | 示例 |
| --- | --- |
| 看进度 | 使用 BMad 显示当前状态，并核对实际可运行的能力；只读 |
| 推进规划任务 | 按 BMad 完成 Story【编号】或 Epic【编号】并验证组合结果 |
| 调整目标 | 用户需要【操作】和【结果】，判断现有规划需怎样调整 |
| 看成果 | 展示本次交付用途、验证方法和未验范围 |
| 复盘 | 基于提交、实施工件和运行记录复盘 Epic |

规划保留在 _bmad-output/planning-artifacts，详细实施状态保留在 _bmad-output/implementation-artifacts。历史研究用于追溯，不直接变成开发队列；本文不复制动态完成状态。BMad 安装信息见 _bmad/_config/manifest.yaml，更新时固定版本并审阅团队定制差异。

## 共同入口

- [开发环境](../development/environment.md)：依赖、版本、本地配置及资源路径。
- [测试指南](../development/testing.md)：基础、官方对照、进程、原生桌面及发行检查。
- [资产维护](../maintainers/assets.md)：显式清单更新与行尾规则。
- [汽车工程技能](automotive-skills.md)：可选规格工作簿、来源与限制。
- [Agent 规则](../../AGENTS.md)：后台验收和 C 编码技能等额外约束。

普通配置链和内置规则不依赖官方 XSD/MOD；兼容/官方对照与原生运行分别准备所需资源和工具。具体能力以 README、模块说明和适用实现为准，不从 story 完成数量推定覆盖或标准符合性。

技能变化可由 Codex 自动发现；无 Agent 的开发者遵循贡献指南，不需要安装这些技能。技能模板或工作簿评分不产生额外质量门。
