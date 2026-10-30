# 未使用的库服务与提供runnable

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

R8.7为Advisory/System；这组仅覆盖ND的16个CanTp／Dcm入口及Ingress_ReadData。当前支持能力与模块签名／CODE映射保持；应用runnable源是手写fixture，不归emitter。按调用方裁剪或头内重构仍是可行替代，项目选择保留库式交付。参数检查、状态／错误报告及原独占区控制不变。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `core/tests/fixtures/component-pipeline/applications/Ingress.c` 中 `Ingress_ReadData` | shape=`Ingress_CODE Std_ReturnType Ingress_ReadData ( Dcm_DataElement_ApplicationValueType Data ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_Init` | shape=`CANTP_CODE void CanTp_Init ( const CanTp_ConfigType * CfgPtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_Shutdown` | shape=`CANTP_CODE void CanTp_Shutdown ( void ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_Transmit` | shape=`CANTP_CODE Std_ReturnType CanTp_Transmit ( PduIdType TxPduId , const PduInfoType * PduInfoPtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_RxIndication` | shape=`CANTP_CODE void CanTp_RxIndication ( PduIdType RxPduId , const PduInfoType * PduInfoPtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_TxConfirmation` | shape=`CANTP_CODE void CanTp_TxConfirmation ( PduIdType TxPduId , Std_ReturnType result ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanTp.c` 中 `CanTp_MainFunction` | shape=`CANTP_CODE void CanTp_MainFunction ( void ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_Init` | shape=`DCM_CODE void Dcm_Init ( const Dcm_ConfigType * ConfigPtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_StartOfReception` | shape=`DCM_CODE BufReq_ReturnType Dcm_StartOfReception ( PduIdType id , const PduInfoType * info , PduLengthType TpSduLength , PduLengthType * bufferSizePtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_CopyRxData` | shape=`DCM_CODE BufReq_ReturnType Dcm_CopyRxData ( PduIdType id , const PduInfoType * info , PduLengthType * bufferSizePtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_TpRxIndication` | shape=`DCM_CODE void Dcm_TpRxIndication ( PduIdType id , Std_ReturnType result ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_MainFunction` | shape=`DCM_CODE void Dcm_MainFunction ( void ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_CopyTxData` | shape=`DCM_CODE BufReq_ReturnType Dcm_CopyTxData ( PduIdType id , const PduInfoType * info , const RetryInfoType * retry , PduLengthType * availableDataPtr ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_TpTxConfirmation` | shape=`DCM_CODE void Dcm_TpTxConfirmation ( PduIdType id , Std_ReturnType result ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_ComM_NoComModeEntered` | shape=`DCM_CODE void Dcm_ComM_NoComModeEntered ( uint8 NetworkId ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_ComM_SilentComModeEntered` | shape=`DCM_CODE void Dcm_ComM_SilentComModeEntered ( uint8 NetworkId ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_ComM_FullComModeEntered` | shape=`DCM_CODE void Dcm_ComM_FullComModeEntered ( uint8 NetworkId ) {`; 范围 `ND/Both/Both`；保留no-diagnostic交付中的库服务或手写提供runnable，未使用报告保持真实。 | 维护者 2026-10-30精确124键决定 |
