# BSW General

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。`SWS_BSWGeneral` 的义务对每个 BSW 模块成立，按模块分篇处置，标记带模块限定。LSduR、PduR、Com、Can、CanIf、CanTp、Dcm、ComM、BswM、Os、Det 与 Rte 已逐条处置；其余模块下表列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_BSWGeneral.pdf`（`4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_BSW_00006`、`SWS_BSW_00115`、`[SWS_BSW_00103]`、`[SWS_BSW_00170]`／`[SWS_BSW_00171]`、`SWS_BSW_00036`；附录 A.2 模块 ID | 实现源包含 `<Mip>_MemMap.h`；文件名区分大小写且不能只靠大小写区分；跨模块头文件版本检查；模块 ID：Can 80、CanIf 60、CanTp 35、Com 50、ComM 12、Dcm 53、PduR 51、Rte 2、Os 1、BswM 42、Det 15、CDD 255，LSduR 132 |

## 按模块处置

| 模块 | 处置 |
| --- | --- |
| [LSduR 与 PduR](lsdur-pdur.md) | 全部 150 条，标记带 `[LSduR]`／`[PduR]` |
| [Com](com.md) | 全部 150 条，标记带 `[Com]` |
| [Can](can.md) | 全部 150 条，标记带 `[Can]` |
| [CanIf](canif.md) | 全部 150 条，标记带 `[CanIf]` |
| [CanTp](cantp.md) | 全部 150 条，标记带 `[CanTp]` |
| [Dcm](dcm.md) | 全部 150 条，标记带 `[Dcm]` |
| [ComM](comm.md) | 全部 150 条，标记带 `[ComM]` |
| [BswM](bswm.md) | 全部 150 条，标记带 `[BswM]` |
| [Os](os.md) | 全部 150 条，标记带 `[Os]` |
| [Det](det.md) | 全部 150 条，标记带 `[Det]` |
| [Rte](rte.md) | 全部 150 条，标记带 `[Rte]` |

## 已知差距

- **`SWS_BSW_00036` 跨模块头文件版本检查**：已读 CanIf 头与多组件模块无对应版本宏或预处理检查，归 #59。
- **当前交付说明**：选定实现的 Host_Implementation.arxml 从实际交付公开头和不可变计划描述入口、独占区及调度；MODULES.md 导航逐模块手册、API 与同源配置说明。服务端口支持边界仍由 #187／#195 跟踪，版本身份仍由 #59 跟踪。
- **Symbolic Name Value**：提供方的配置头按 TPS_ECUC_02108 唯一导出所选句柄，生成配置包含提供方头并使用这些符号；LSduR 保持由路由计划合成的配置边界。

## Det 报告身份

`runtime/bsw/include/` 的各模块主头提供 `<MIP>_MODULE_ID`，取值按 R24-11 `SWS_BSW_00256` 与附录 A.2 独立核对。当前报告路径为 Can、CanIf 与 CanTp；服务 ID 在各自主头按原 API 表命名：

| 模块／路径 | 服务宏后缀和值 | 原文 API 表 |
| --- | --- | --- |
| Can 丢帧轮询 | `SID_MAIN_FUNCTION_READ=0x08` | `SWS_Can_00226` |
| CanIf 发送拒绝 | `SID_TRANSMIT=0x49` | `SWS_CANIF_00005` |
| CanIf 接收 DLC 拒绝 | `SID_RX_INDICATION=0x14` | `SWS_CANIF_00006` |
| CanTp 定时处理 | `SID_MAIN_FUNCTION=0x06` | `SWS_CanTp_00213` |
| CanTp 接收处理 | `SID_RX_INDICATION=0x42` | `SWS_CanTp_00214` |
| CanTp 取消发送／接收 | `SID_CANCEL_TRANSMIT=0x4a`／`SID_CANCEL_RECEIVE=0x4c` | `SWS_CanTp_00246`／`SWS_CanTp_00257` |

CanTp 的内部 `CanTp_Report` 保留服务参数与零错误过滤；Det 入口签名、通知与终止行为不变。`bsw-contracts` 消费者按原文数值断言头宏与实际通知，预期不引用生产宏。宏为增量公开信息，不改变函数 ABI、配置或类型布局；重新生成交付包以取得新头。完整 published information、vendor/release 身份与跨模块检查仍由 #59 承接。
