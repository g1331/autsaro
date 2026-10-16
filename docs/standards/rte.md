# RTE 与 SchM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_RTE.pdf`（`d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）。

| 义务区域 | 条款 |
| --- | --- |
| 契约阶段与生成阶段 | §3.1（p92 起）、§3.2 |
| 实例约束 | §4.1.4.2–4.1.4.4、`SWS_Rte_02008／02009` |
| 任务映射与位置 | §4.2.3、`ECUC_Rte_09023`；TimingEvent 绑定 `ECUC_Rte_09024`（Alarm）／`ECUC_Rte_09026`（ExpiryPoint）、`09025`（OsEvent 引用）；`CONSTR08936`／`08938`（未显式启用映射的 application／BSW 事件不得持有 task ref） |
| 标准 Read／Call 与服务调用 | §5.6.10／5.6.13；`SWS_Rte_06019`、`04515`；`SWS_Rte_07027`（USE-VOID）、`CONSTR1286`（primitive IN 限制） |
| 接收状态与初值 | `SWS_Rte_07804／07805`；§4.3.1.7／`SWS_Rte_08104`（p317，`handleTimeoutType=none` 保留）；`SWS_Rte_04501／04502`（P／R 初值冲突时 R 优先）；`06009／06010`；`SWS_Rte_07381–07383`（本地 `handleNeverReceived=true`，未实现） |
| COM 状态与通知 | `SWS_Rte_06830／07822`（仅 `COM_SERVICE_NOT_AVAILABLE` 映射 `COM_STOPPED`）；`SWS_Rte_08061／08062／08103／08104`（timeout 通知）；`SWS_Rte_91123／91127`（DRAFT，`Rte_COMCbk`／`Rte_COMCbkRxTOut`） |
| 服务器形状 | §5.7.5.6／`SWS_Rte_08913`（无 possibleErrors 的 server entry 返回 void） |
| 生命周期 | `Rte91137／02569／01309`、`91138／02570／01310`（Start／Stop）；`09035–09038` |
| 内存段 | `05088`、`07422／07424`、`04615／07710`（draft）、`07426／07427`、`07592`、`05089／05090` |
| BSWMD 与依赖 | `08404`、`05177`、`05179／05180` |
| Mode | `07255／07260`、`07256`／`constr09049`、`07261` |
| SchM | 生命周期与独占区条目在 [SchM](schm.md) 中逐条处置；`06750／06755`（`Rte_Init`／`Rte_StartTiming`）属于 RTE |
