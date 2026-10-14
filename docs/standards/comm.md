# ComM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_COMManager.pdf`（`564560db2899fa0f8695e5fad607f3cc477a4c84abfccfe968e916117cf2ef34`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_ComM_00866／00873／00874／00861／00957`、`ECUC_ComM_00888`；`SWS_ComM_00962`；`00275`；`00893／00895`；`00619／00552／91002`；`00669`；ECUC `00563`；`00783`；`01022／01023` | 所选 host CDD channel（`COMM_BUS_TYPE_CDD`，`Ecu_HostBusSM`，NM NONE）为合法 provider；选择 `COMM_BUS_TYPE_CAN` 须真正 CanSM；诊断 active 为 FULL 需求；NM NONE 仍需 NETWORK_REQUESTED／READY_SLEEP 与最小 Full duration；同步唤醒为 false 时的唤醒请求状态 |
