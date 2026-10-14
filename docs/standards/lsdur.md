# LSduR

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_LSduRouter.pdf`（`2cd776bc9d54dd8d7f12fa300f24bbf4b790560f9fe7cc9702d20c76b24fdd3a`，R24-11 DRAFT）。

| 条款 | 义务要点 |
| --- | --- |
| `91006`（Init）、`91009`（upper transmit）、`91011`（Rx）、`91013`（confirmation）、`91014`（pull）；`00008／00012／00015–19`；`00024／00025`（zero cost）；`00035` | 一对一、同 partition、无 metadata 的 IF 路由翻译真实 PDU ID 并原样传播 lower 返回值；LSduR 无 Rx／Tx 长度检查或 Tx buffering；未选 ZeroCostOperation |
