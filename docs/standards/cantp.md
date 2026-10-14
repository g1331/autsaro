# CanTp

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_CANTransportLayer.pdf`（`7334d633b02c443aacbe0ca25e20c319e977104eeb4bf8e67ac0358cfea34a22`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_CanTp_00208／00211／00212／00213／00214／00215`；`00027／00168／00170／00238／00273／00202／00200`；`SWS_CanTp_00315`；`00223`／`ECUC_CanTp_00251`；`SWS_CanTp_00343`；`00205`；`00184／00280`；`00204／00298／00299`、`00075／00355`、`00057`、`00344–00349`；`00352／00229`；`00257／00260–00263`；`ECUC_CanTp_00301／00268` | Init 取 `const CanTp_ConfigType*`；OFF→ON／idle，reinit 丢连接，Shutdown 无 PduR 通知；已接纳 Tx／Rx 必有最终通知，拒绝 admission 无最终通知；收到 FC(WAIT) 重启 N_Bs，WFTmax 限制本端发出的 WAIT；lower E_NOT_OK 终止发送，与 upper CopyTxData BUSY 的 N_Cs 内重试区分；CanTp 自身 NSdu ID 与 PduR callback ID 属于不同命名空间 |
