# Det

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_DefaultErrorTracer.pdf`（`f5caa8cb8f2b9ef2e2578a96d940abb795fbf7dfc09a1899b0b81797c2835e1f`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_Det_00008／00010／00183／00184`；`00024`；`00014／00018／00503`；`00026／00208` | `Det_ReportRuntimeError` 恒返回 E_OK，已 Init 时调用全部配置 hooks；关闭 DevErrorDetect 不免除 runtime 报告；`Det_ReportError` 要求停止执行，与 runtime 报告不同 |
