# CanSM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_CANStateManager.pdf`（`9f215f2f532e94a9d8f2bbbf4ba4208d31117d1b82f5277792638f3b6f0fcceb`）。

| 条款 | 义务要点 |
| --- | --- |
| — | 仅核对 CAN BusSM 义务与 host CDD 边界；**未实现、不宣称**完整 CanSM |

## 已知差距

- **网络管理**：完整 Nm、CanNm、CanSM 未实现；当前 host CDD BusSM 的模式行为不代表 CAN BusSM 符合性。
