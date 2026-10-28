# OS 标准服务的链接诊断

这些记录只处置实际公开服务的 R8.7，不宣称完成模块符合性。原始源位置按各 profile 的 source seal 保留。
授权源为 ADR 0023 原用户决定；本批记录待独立复核，尚不添加相应源码 MR12。既有已批准项保持原记录，不重复登记。

## 接口与替代边界

下表列明固定 R24-11 SpecificationsARXML 中的服务定义、Available via，或被继承的 OSEK 2.2.3 具体服务章节。保留当前真实签名和公开头绑定；所选程序没有额外 TU 调用时仍承认 R8.7 Advisory 违反，不把公开名称本身当成使用证据。
删除或 static 化会撤销该对外服务。宏／header inline 替代须同时满足原文、标准头、既有 C 链接与 ARTI／模块代码放置合同，不能仅以同名包装宣称等价；本批独立复核须检查各条的实际可用条件，条件外项不进入已完成处置。
风险为公共可见面；原有参数、状态、上下文、独占区与资源错误行为保持，以既有模块 C99 合同、OS suite 和外部 C 消费者验证。规范、配置可用性、签名、调用方或 ABI 变化时重评。

| 规则 | 精确范围 | 固定原文及公开合同 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Arti.c` 中 `Arti_GetVersionInfo` | SWS_Arti_91005；Arti.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetISRID` | SWS_Os_00511；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ControlIdle` | SWS_Os_00769；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `isOsStarted` | SWS_Os_91034；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetCoreID` | SWS_Os_00674；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetTaskID` | OSEK OS 2.2.3 §13.2.3.5；Syntax:            StatusType GetTaskID ( TaskRefType <TaskID> )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetTaskState` | OSEK OS 2.2.3 §13.2.3.6；Syntax:            StatusType GetTaskState ( TaskType <TaskID>,；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `TerminateTask` | OSEK OS 2.2.3 §13.2.3.2；Syntax:            StatusType TerminateTask ( void )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ChainTask` | OSEK OS 2.2.3 §13.2.3.3；Syntax:            StatusType ChainTask ( TaskType <TaskID> )；Conformance:                 BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetResource` | OSEK OS 2.2.3 §13.4.3.1；Syntax:           StatusType GetResource ( ResourceType <ResID> )；Conformance:         BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ReleaseResource` | OSEK OS 2.2.3 §13.4.3.2；Syntax:            StatusType ReleaseResource ( ResourceType <ResID> )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `Schedule` | OSEK OS 2.2.3 §13.2.3.4；Syntax:                       StatusType Schedule ( void )；Conformance:        BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetActiveApplicationMode` | OSEK OS 2.2.3 §13.7.2.1；；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `IncrementCounter` | SWS_Os_00399；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetAlarmBase` | OSEK OS 2.2.3 §13.6.3.1；Syntax:           StatusType GetAlarmBase (        AlarmType <AlarmID>,；Conformance:         BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetAlarm` | OSEK OS 2.2.3 §13.6.3.2；Syntax:            StatusType GetAlarm ( AlarmType <AlarmID>；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `SetRelAlarm` | OSEK OS 2.2.3 §13.6.3.3；Syntax:            StatusType SetRelAlarm ( AlarmType <AlarmID>,；Conformance:         BCC1, BCC2, ECC1, ECC2; Events only ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `SetAbsAlarm` | OSEK OS 2.2.3 §13.6.3.4；Syntax:            StatusType SetAbsAlarm (         AlarmType <AlarmID>,；Conformance:          BCC1, BCC2, ECC1, ECC2; Events only ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `CancelAlarm` | OSEK OS 2.2.3 §13.6.3.5；Syntax:            StatusType CancelAlarm ( AlarmType <AlarmID> )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `StartScheduleTableRel` | SWS_Os_00347；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `StartScheduleTableAbs` | SWS_Os_00358；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `StopScheduleTable` | SWS_Os_00006；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `NextScheduleTable` | SWS_Os_00191；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `GetScheduleTableStatus` | SWS_Os_00227；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `DisableInterruptSource` | SWS_Os_91019；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `EnableInterruptSource` | SWS_Os_91020；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ClearPendingInterrupt` | SWS_Os_91021；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `EnableAllInterrupts` | OSEK OS 2.2.3 §13.3.2.1；Syntax:             void EnableAllInterrupts ( void )；Conformance:         BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `DisableAllInterrupts` | OSEK OS 2.2.3 §13.3.2.2；Syntax:             void DisableAllInterrupts ( void )；Conformance:        BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ResumeAllInterrupts` | OSEK OS 2.2.3 §13.3.2.3；Syntax:           void ResumeAllInterrupts ( void )；Conformance:        BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `SuspendAllInterrupts` | OSEK OS 2.2.3 §13.3.2.4；Syntax:            void SuspendAllInterrupts ( void )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `ResumeOSInterrupts` | OSEK OS 2.2.3 §13.3.2.5；Syntax:          void ResumeOSInterrupts ( void )；Conformance:          BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `SuspendOSInterrupts` | OSEK OS 2.2.3 §13.3.2.6；Syntax:            void SuspendOSInterrupts ( void )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 维护者 原用户决定，ADR 0023（待独立复核） |
