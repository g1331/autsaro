# Agent 怎样记录工作流反馈

[`active.json`](active.json)只放仍需处理或正在试行的反馈；[`HISTORY.md`](HISTORY.md)保存关闭结论。每个 Agent 在执行任务时遇到具体摩擦，就以 `WF-001` 等 ID 记录一次观察。同一问题再次出现时追加 `observations`；不要猜测次数，也不要为没有出现过的问题预填反馈。只读的“状态如何”不写反馈。

新反馈的结构如下。`task_id` 填当时的任务 ID；问题发生在任务之外时可填 `null`。`source` 是可选的仓库相对路径，指向能帮助下个 Agent 核对现象的文件；命令及输出摘要写在 `detail` 中，不要把临时日志路径当成持久证据。

```json
{
  "id": "WF-001",
  "status": "observed",
  "severity": "normal",
  "problem": "同一任务交接后，下一位 Agent 找不到失败输入",
  "impact": "独立复核无法复现，任务停留在 review",
  "observations": [
    {
      "task_id": "AUDIT-001",
      "detail": "证据只写了测试名，没有列出输入文件和命令"
    }
  ]
}
```

`severity` 只用 `normal` 或 `critical`。虚假支持声明、真实数据风险和门禁绕过属于 `critical`，应立即处理对应缺陷。`observed` 表示待复盘；需要用户决定产品或授权边界时改为 `needs_decision`，同时填写 `decision_needed` 和 `recommendation`，不反复提示自动复盘。

Agent 实施流程改动后，将反馈改为 `trial`，填写 `change`（改了什么、如何撤回）、`success_condition`（后续怎样判断好转）、`change_files`（仓库内的改动文件）和 `baseline_done_task_ids`（试行开始时所有已完成任务 ID）。两张**新完成的任务**之后检查同类场景是否改善；若任务没有触及同类情形，继续试行。若仍有问题，在原记录追加观察，调整或撤回改动并重设试行基线。确认解决或确认无需修改时，把反馈移到 `HISTORY.md`，包含 ID、日期、原现象、改动/不改的理由、后续任务与实际结果。历史不充当能力验收证据。

`reviewed_done_task_ids` 保存上次工作流复盘时已经完成的全部任务 ID。每次复盘完成后在 `HISTORY.md` 记录日期、检查的任务 ID 与结论，即使本次无需调整，也更新该基线；达到三张新完成任务时再复盘。运行 `python scripts/workflow.py check` 检查字段与引用，`python scripts/workflow.py status` 查看只读的复盘触发原因。字段检查只能发现缺项，不能替代对反馈真实性和改动效果的判断。
