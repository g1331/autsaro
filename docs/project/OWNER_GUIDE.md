# 规划与 Agent 协作

开发环境、检查命令和提交方式见 [贡献指南](../../CONTRIBUTING.md)。本文说明项目怎样规划和跟踪工作，以及使用 Codex、Claude Code 协作时的常用方式，可按任务需要选用。

## 规划与跟踪

| 内容 | 位置 |
| --- | --- |
| 方向、近期顺序、长期候选与待决问题 | [路线图](../roadmap/README.md) |
| 当前架构、所有权与状态不变量 | [架构](../architecture/README.md) |
| 重要决定及其理由 | [ADR](../adr/) |
| 标准版次、官方资料位置与模块义务索引 | [标准基线](../standards/README.md) |
| 已承诺的阶段 | GitHub Milestones |
| 具体任务、已知差距与问题 | GitHub Issues |
| 实现、验证证据与审阅 | PR（用 `Closes #N` 关联 Issue） |

- 只为已承诺开工的阶段建立 Milestone；长期候选只在路线图中保持成果级定义，开工时再建 Milestone 并拆分 Issue。
- Issue 写明触发问题、依据的规范条款、独立的成功／拒绝／恢复预期和验证命令；超出本次范围的差距写明依据、影响与承接位置。
- 改变路线顺序、产品契约或支持边界的决定先写 ADR，再更新路线图和架构文档。被取代的 ADR 只改状态并链接新 ADR，不改写正文。
- 状态以 Issues、Milestones 和已合并 PR 为准，不另建状态文件、报告或台账。
- 2026-10 之前的产品简述、PRD、Epic／Story 规格、回顾与验收记录保存在归档标签 `planning-archive-2026-10` 的 `_bmad-output/` 下，可用 `git show planning-archive-2026-10:<路径>` 或在 GitHub 上按该标签浏览，仅供追溯。

## 委托任务

可以描述明确的问题与预期行为，或直接给出 Issue 编号。Agent 先核对当前 Git、源码、对应 Issue 和实际验证范围，再按贡献流程实现和验证。

| 任务 | 示例 |
| --- | --- |
| 看进度 | 汇总 Milestone【名称】的 Issue 状态，并核对实际可运行的能力；只读 |
| 推进任务 | 按 Issue #【编号】的验收标准完成实现与验证，提交 PR |
| 调整目标 | 用户需要【操作】和【结果】，判断路线图与现有 Issue 需怎样调整 |
| 看成果 | 展示本次交付用途、验证方法和未验范围 |
| 复盘 | 基于已合并 PR、Milestone 与运行记录复盘阶段成果 |

## 共同入口

- [开发环境](../development/environment.md)：依赖、版本、本地配置及资源路径。
- [测试指南](../development/testing.md)：基础、官方对照、进程、原生桌面及发行检查。
- [资产维护](../maintainers/assets.md)：显式清单更新与行尾规则。
- [Agent 规则](../../AGENTS.md)：后台验收和 C 编码技能等额外约束。

日常配置和内置规则无需官方 XSD/MOD。官方对照测试及原生运行需分别准备对应资源和工具。功能覆盖与支持范围见 README、模块说明和相应实现，标准符合性需按适用规范单独核查。

## 项目技能

项目技能只维护在 `.agents/skills`（`impeccable`、`misra-c2012`），Codex 直接读取；Claude Code 通过 `.claude/skills` 软链接读取同一目录，两者都能自动发现技能变化。新增或更新技能只改 `.agents/skills`。`c-check` 读取 `misra-c2012/references/baseline-index.json`，该技能不能随意移除。手工开发按贡献指南操作即可。

Windows 上使用 Claude Code 时，需要让 Git 检出真正的软链接，否则 `.claude/skills` 会成为普通文件，Claude Code 找不到技能，其他开发不受影响。每台电脑设置一次：开启“开发者模式”，执行 `git config --global core.symlinks true`（或在 Git for Windows 安装时勾选 Enable symbolic links）。之后的克隆会自动生效；已有克隆删除 `.claude/skills` 后执行 `git checkout -- .claude/skills` 重新检出。
