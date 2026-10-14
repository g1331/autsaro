# BswM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_BSWModeManager.pdf`（`a6ce38abd020c3aef91bc90ebc35243b888d05b899c36eba33a2305bfbe08ab3`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_BswM_00047／00078／00091`；`00880／00883`；`00955`；`00935`；`ECUC_BswM_00843`；`00115／00116`、`00066`；`00214／00216`；`00053`；`00075` | IMMEDIATE ComM mode input 及其规则；`BswM_ModeType`／`UserType` 为 uint16；`BswM_MainFunction` 在 `SchM_BswM.h`；无 DEFERRED 时无需周期评价但入口须存在；UserCallout 为完整固定参数调用 |
