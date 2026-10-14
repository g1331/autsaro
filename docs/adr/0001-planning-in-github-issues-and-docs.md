# 0001. 规划与任务跟踪从 BMad 迁到 GitHub Issues 与 docs

## 背景

项目此前以 `_bmad-output/` 下的 BMad 工件（PRD、epics、各期架构 spine、规格、story 与 `sprint-status.yaml`）作为规格与任务状态来源。这些文件混合了仍然有效的架构约束、已完成工作的实施流水、历史验收结果和以 Epic／Story 编号表达的阶段身份。接手者需要读完大量历史记录才能分辨当前约束；编号还渗入了产品代码、测试名和交付身份。普通开发不应依赖代理工具或其工作流才能理解项目。

## 决策

1. 当前架构与不变量写在 `docs/architecture.md`，一份随代码维护的文档，只描述当前状态和明确标注的实施中方向。
2. 重大决定按 Nygard 格式写入 `docs/adr/NNNN-*.md`，一个决定一个文件；被取代时新增 ADR 并更新旧 ADR 的状态，不改写历史正文。
3. AUTOSAR／MISRA 基线、官方资料位置与模块义务索引写在 `docs/standards/`。
4. 待办、缺陷、实施计划与验收讨论使用 GitHub Issues 和 PR；修复证据放在 PR，不另建报告或台账。
5. `_bmad-output/` 下的原工件以 Git 标签 `planning-archive-2026-10` 保存后从主线移除；新文档以 `planning-archive-2026-10:_bmad-output/<path>` 引用原文，不复制实施流水。
6. BMad 工具本身（`_bmad/` 与 `.agents/skills/bmad-*`）一并移除；`AGENTS.md` 与 `docs/project/OWNER_GUIDE.md` 改为以 Issues、Milestones 和上述文档为准。长期方向与候选写在 `docs/roadmap.md`，只为已承诺的阶段建立 Milestone。

## 后果

- 接手者从 README、CONTRIBUTING、`docs/architecture.md`、ADR 和 Issues 即可理解与推进项目，无需安装代理工具。
- 历史完成证据、验收数字和复盘仍可通过归档标签追溯，但不再作为当前状态；当前状态以代码、CI 与 Issues 为准。
- 只有一个状态来源：Issues、Milestones 与已合并 PR。
- 原规划中的约束只有迁入 `docs/` 或 ADR 的才继续有效；未迁移内容及理由记录在迁移 PR 的描述中。

## 状态

已接受（2026-10），随规划资料迁移 PR 合并生效。

来源：本次迁移决定；被取代的工作方式见 `planning-archive-2026-10:_bmad-output/implementation-artifacts/spec-bmad-workflow-cleanup.md`、`planning-archive-2026-10:_bmad-output/implementation-artifacts/sprint-status.yaml`。
