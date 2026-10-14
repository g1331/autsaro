# COM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_COM.pdf`（`8addd33ad1e625b4e9d01afc938131d424f7161f18075f44d2a5fcfa925c5190`）。

| 条款 | 义务要点 |
| --- | --- |
| §7.3／8.3／8.4；`SWS_Com_00432`（Init）、`00130`（DeInit）、`00194`（GetStatus）、`00197`（SendSignal）、`00198`（ReceiveSignal）、`00123`（RxIndication）、`00001`（TriggerTransmit）；`SWS_Com_00840`；`SWS_Com_00772`；`00300／00301`；`00348／00861／00388`；`SWS_Com_91019／91020`（DRAFT）；ECUC `00181`（MDT）、`00387`（ComIPduCallout） | 标准 callback 形状为 `void Com_RxIndication(PduIdType,const PduInfoType*)` 与 `Std_ReturnType Com_TriggerTransmit(PduIdType,PduInfoType*)`（pull-copy）；未分组 Tx PDU 在 Com_Init 隐式启动且不可停止；只有属于启用 Rx DM 的 group 的 PDU 才启用 monitoring；IMMEDIATE 与 DEFERRED 语义不同；TriggerIPDUSend 手动请求与零 minimum delay |
