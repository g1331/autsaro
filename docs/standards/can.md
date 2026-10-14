# Can

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_CANDriver.pdf`（`d04d35b80d8f445b66f960fd9b5be4f02f324e73cc4be239887e8c1235b172aa`）。

| 条款 | 义务要点 |
| --- | --- |
| `00233`（typed Can_Write）；`00212–214／00275`；`00282`；`00370／00373`；`00218`；`SWS_Can_00227`、`00109`、`ECUC_Can_00314`、`00183`；`SWS_Can_00368`、`00369`；`SWS_Can_00234`、`SWS_Can_00279`、`SWS_CAN_00496`；`ECUC_Can_00316`、`ECUC_Can_00326`；`ECUC_Can_00437／00438／00484`、`00355`、`00486`；`SWS_Can_00441／00442` | 实际 BUSY 与已复制帧；STOP 取消；polling mode indication；BusOff polling 由 `Can_MainFunction_BusOff`；driver 发出的 Mode／BusOff 使用 abstract CanIf ID；driver controller ID 从 0 无间隔，HRH／HTH 从 0 无间隔 |

## 已知差距

- **Can published information**：项目尚无已分配的 AUTOSAR vendor ID，也无经核对的 R24-11 数字发布宏映射；`CAN_VERSION_INFO_API` 默认关闭，启用即拒绝。取得正式身份后才能实现 GetVersionInfo 与外部版本交叉检查（见 [runtime/README.md](../../runtime/README.md)）。
