# BSW 标准服务的链接诊断

这些记录只处置实际公开服务的 R8.7，不宣称完成模块符合性。原始源位置按各 profile 的 source seal 保留。
授权源为 ADR 0023 原用户决定；本批记录待独立复核，尚不添加相应源码 MR12。既有已批准项保持原记录，不重复登记。

## 接口与替代边界

下表列明固定 R24-11 SpecificationsARXML 中的服务定义、Available via，或被继承的 OSEK 2.2.3 具体服务章节。保留当前真实签名和公开头绑定；所选程序没有额外 TU 调用时仍承认 R8.7 Advisory 违反，不把公开名称本身当成使用证据。
删除或 static 化会撤销该对外服务。宏／header inline 替代须同时满足原文、标准头、既有 C 链接与 ARTI／模块代码放置合同，不能仅以同名包装宣称等价；本批独立复核须检查各条的实际可用条件，条件外项不进入已完成处置。
风险为公共可见面；原有参数、状态、上下文、独占区与资源错误行为保持，以既有模块 C99 合同、OS suite 和外部 C 消费者验证。规范、配置可用性、签名、调用方或 ABI 变化时重评。

| 规则 | 精确范围 | 固定原文及公开合同 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_DisableControllerInterrupts` | SWS_Can_00231；Can.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_EnableControllerInterrupts` | SWS_Can_00232；Can.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_CheckWakeup` | SWS_Can_00360；Can.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanIf.c` 中 `CanIf_GetControllerRxErrorCounter` | SWS_CANIF_91003；CanIf.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/CanIf.c` 中 `CanIf_GetControllerTxErrorCounter` | SWS_CANIF_91004；CanIf.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_GetStatus` | SWS_Com_00194；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_IpduGroupStop` | SWS_Com_91002；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_DisableReceptionDM` | SWS_Com_91003；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Com.c` 中 `Com_TriggerIPDUSend` | SWS_Com_00348；Com.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_GetSecurityLevel` | SWS_Dcm_00338；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_GetSesCtrlType` | SWS_Dcm_00339；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_ResetToDefaultSession` | SWS_Dcm_00520；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Dcm.c` 中 `Dcm_SetActiveDiagnostic` | SWS_Dcm_01068；Dcm.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Det.c` 中 `Det_ReportError` | SWS_Det_00009；Det.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/LSduR.c` 中 `LSduR_GetConfigurationId` | CP_SWS_LSduR_91008；LSduR.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/LSduR.c` 中 `LSduR_CanIfTriggerTransmit` | CP_SWS_LSduR_91014／00022／00023；Lo=CanIf，LSduR_CanIf.h 通过包含 LSduR.h 提供。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/PduR.c` 中 `PduR_GetConfigurationId` | SWS_PduR_00341；PduR.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
