# BSW 标准服务的链接诊断

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

## 保留形式与替代

R8.7为Advisory/System。宏／header inline或按程序裁剪是可行替代，标准服务存在不证明外部链接必需；维护者选择在本表程序范围保留当前库形式。既有独立消费者、状态／错误语义、ARTI顺序及可配置CODE段保持。可见面误用风险以原有参数、上下文、资源与独占区控制限定。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_DisableControllerInterrupts` | shape=`CAN_CODE void Can_DisableControllerInterrupts ( uint8 Controller ) {`; 范围 `All/Both/Both`；SWS_Can_00231；Can.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_EnableControllerInterrupts` | shape=`CAN_CODE void Can_EnableControllerInterrupts ( uint8 Controller ) {`; 范围 `All/Both/Both`；SWS_Can_00232；Can.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanIf.c` 中 `CanIf_GetControllerRxErrorCounter` | shape=`CANIF_CODE Std_ReturnType CanIf_GetControllerRxErrorCounter ( uint8 ControllerId , uint8 * RxErrorCounterPtr ) {`; 范围 `All/Both/Both`；SWS_CANIF_91003；CanIf.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanIf.c` 中 `CanIf_GetControllerTxErrorCounter` | shape=`CANIF_CODE Std_ReturnType CanIf_GetControllerTxErrorCounter ( uint8 ControllerId , uint8 * TxErrorCounterPtr ) {`; 范围 `All/Both/Both`；SWS_CANIF_91004；CanIf.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_GetStatus` | shape=`COM_CODE Com_StatusType Com_GetStatus ( void ) {`; 范围 `All/Both/Both`；SWS_Com_00194；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_IpduGroupStop` | shape=`COM_CODE void Com_IpduGroupStop ( Com_IpduGroupIdType IpduGroupId ) {`; 范围 `All/Both/Both`；SWS_Com_91002；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_DisableReceptionDM` | shape=`COM_CODE void Com_DisableReceptionDM ( Com_IpduGroupIdType IpduGroupId ) {`; 范围 `All/Both/Both`；SWS_Com_91003；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_TriggerIPDUSend` | shape=`COM_CODE Std_ReturnType Com_TriggerIPDUSend ( PduIdType PduId ) {`; 范围 `All/Both/Both`；SWS_Com_00348；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_GetSecurityLevel` | shape=`DCM_CODE Std_ReturnType Dcm_GetSecurityLevel ( Dcm_SecLevelType * SecLevel ) {`; 范围 `All/Both/Both`；SWS_Dcm_00338；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_GetSesCtrlType` | shape=`DCM_CODE Std_ReturnType Dcm_GetSesCtrlType ( Dcm_SesCtrlType * SesCtrlType ) {`; 范围 `All/Both/Both`；SWS_Dcm_00339；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_ResetToDefaultSession` | shape=`DCM_CODE Std_ReturnType Dcm_ResetToDefaultSession ( void ) {`; 范围 `All/Both/Both`；SWS_Dcm_00520；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_SetActiveDiagnostic` | shape=`DCM_CODE Std_ReturnType Dcm_SetActiveDiagnostic ( boolean active ) {`; 范围 `All/Both/Both`；SWS_Dcm_01068；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Det.c` 中 `Det_ReportError` | shape=`DET_CODE Std_ReturnType Det_ReportError ( uint16 ModuleId , uint8 InstanceId , uint8 ApiId , uint8 ErrorId ) {`; 范围 `All/Both/Both`；SWS_Det_00009；Det.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/LSduR.c` 中 `LSduR_GetConfigurationId` | shape=`LSDUR_CODE LSduR_PBConfigIdType LSduR_GetConfigurationId ( void ) {`; 范围 `All/Both/Both`；CP_SWS_LSduR_91008；LSduR.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/LSduR.c` 中 `LSduR_CanIfTriggerTransmit` | shape=`LSDUR_CODE Std_ReturnType LSduR_CanIfTriggerTransmit ( PduIdType TxPduId , PduInfoType * PduInfoPtr ) {`; 范围 `All/Both/Both`；CP_SWS_LSduR_91014／00022／00023；Lo=CanIf，LSduR_CanIf.h 通过包含 LSduR.h 提供。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/PduR.c` 中 `PduR_GetConfigurationId` | shape=`PDUR_CODE PduR_PBConfigIdType PduR_GetConfigurationId ( void ) {`; 范围 `All/Both/Both`；SWS_PduR_00341；PduR.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
