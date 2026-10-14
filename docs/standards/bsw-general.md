# BSW General

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_BSWGeneral.pdf`（`4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_BSW_00006`、`SWS_BSW_00115`、`[SWS_BSW_00103]`、`[SWS_BSW_00170]`／`[SWS_BSW_00171]`、`SWS_BSW_00036`；表 13.2 模块 ID | 实现源包含 `<Mip>_MemMap.h`；文件名区分大小写且不能只靠大小写区分；跨模块头文件版本检查；模块 ID：Can 80、CanIf 60、CanTp 35、Com 50、ComM 12、Dcm 53、PduR 51、Rte 2、Os 1、BswM 42、CDD 255，LSduR 132 |

## 已知差距

- **`SWS_BSW_00036` 跨模块头文件版本检查**：已读 CanIf 头与多组件模块无对应版本宏或预处理检查，归 #59。
