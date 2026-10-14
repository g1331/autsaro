# Dcm

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf`（`cf5aeee78fda6e5a25f982f04cb146ce4a75586de7ba41913d46b0cea1cc2407`）。

| 条款 | 义务要点 |
| --- | --- |
| §7.4.2（0x22）、`SWS_Dcm_01335`／`ECUC_Dcm_00638`、`SWS_Dcm_00433–00437`、`ECUC_Dcm_00713／00669`；§8.7.3.2.1、`SWS_Dcm_00793`（p320）、§8.8.3.2（p415）；`SWS_Dcm_00037／00094／00556／00093／00092／00351／00053`；`SWS_Dcm_01069`、`01142`、`01377`、`ECUC_Dcm_00952`、`00148–00162／01324–01326`、`SWS_Dcm_01068`；`00356／00358／00360`；`00111／00241`；`01069–01071／01142／01373–01378`；`00338`、`00339`、`00520`；`00977／00978`；`01062`、`00311`、`01670`、`91019`；`00085`（F186）；`00200／00201／00204`、`00238／00240`；`00806`；`01327／00777／00807／00781`、`91021／23／27／31`、`91034／35／37／39`、`91022／24／28／32`；`01477／01478／01480`、`91067／91074／91075`；`ECUC_Dcm_00864` | `USE_DATA_SYNCH_CLIENT_SERVER`／`UINT8_N` 同步 ReadData；先核对 DID／会话／长度再调用数据访问；Dcm 拥有 request／response buffer，到 confirmation 前不复用；`Dcm_SetActiveDiagnostic` 恒 E_OK；必需 `DcmDslProtocolComMChannelRef`；NoCom／Silent／Full 按 channel 门控；内部 DID F186 读取当前会话；DID 数超过所配置 `DcmDspMaxDidToRead` 回 NRC 0x13；SPRMIB；每个 connection 的 authentication state |
