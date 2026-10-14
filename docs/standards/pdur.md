# PduR

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_PDURouter.pdf`（`3f97e95f6b36c876daeddc1bd90db7d0b24ebd0aa652078349d323ad6664ad6f`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_PduR_00334`（Init／PBConfigType）；`00406`（upper transmit）；`00507／00518／00375`；`ECUC_PduR_00311／00322`；`00241` | `PduR_<User:Up>Transmit` 为 `Std_ReturnType(PduIdType,const PduInfoType*)`；lower Rx 为 `void(PduIdType,const PduInfoType*)`；TP StartOfReception／CopyRxData／CopyTxData 的 BufReq 与 buffer-size 闭包；PBConfigType 由模块所属 `PduR_PBcfg.c` 生产 |
