/** @file Internal implementation and trace declarations used by the typed OS service entries. */
#ifndef AUTOSAR_OS_SERVICE_H
#define AUTOSAR_OS_SERVICE_H
#include "Os_Version.h"
#include "Os_Hooks.h"
#include "Arti.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(ARTI_AR_RELEASE_MAJOR_VERSION) || !defined(ARTI_AR_RELEASE_MINOR_VERSION)
#error "Os_Service.h -> Arti.h: missing AUTOSAR release"
#elif (ARTI_AR_RELEASE_MAJOR_VERSION != 4U) || (ARTI_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Service.h -> Arti.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** @brief Emit an application service event with an already captured payload.
 * @param[in] service Captured OS service identifier.
 * @param[in] parameter Already captured scalar service event payload.
 * @param[in] returning Zero for entry, nonzero for return.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; arguments are already captured for the current
 * logical caller. Context: service wrapper entry/return on its Task, ISR, Hook or initialized-C
 * caller, including refused calls; no native critical section is required, outside BSW areas.
 * Reentrancy: reentrant across native actors; logical caller scopes preserve TLS side data, and
 * atomic slot reservation/publication protects the tool buffer. Ownership: event payloads are
 * copied; native addresses are recorded as bits without retaining or dereferencing pointed-to
 * storage.
 */
OS_MEMMAP void Os_ArtiService(OSServiceIdType service, uint32 parameter, sint32 returning);

/** @brief Preserve the complete native pointer alongside its standard uint32 token.
 * @param[in] address Native AlarmBase result address, captured as bits without dereferencing.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; arguments are already captured for the current
 * logical caller. Context: successful GetAlarmBase wrapper return after the implementation has
 * returned; no native critical section is guaranteed, outside BSW areas. Reentrancy: reentrant
 * across native actors; logical caller scopes preserve TLS side data, and atomic slot
 * reservation/publication protects the tool buffer. Ownership: event payloads are copied; native
 * addresses are recorded as bits without retaining or dereferencing pointed-to storage.
 */
OS_MEMMAP void Os_ArtiAlarmBaseReturn(AlarmBaseRefType address);

/** Enter a service scope and reset its captured access result.
 * @return Nonzero only for an outermost application-origin service call.
 * Preconditions: service nesting depth remains below UINT32_MAX.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP sint32 Os_ArtiServiceEnter(void);

/** Leave the matching service scope.
 * @par Return
 * None.
 * Preconditions: an unmatched leave asserts; service depth must be nonzero.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiServiceLeave(void);

/** Test whether the last captured service access was accepted.
 * @return Nonzero for E_OK; zero for a refused access.
 * Preconditions: current logical service access result has been captured.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP sint32 Os_ArtiAccessValid(void);

/** Read the last captured service access result.
 * @return Current logical caller access status.
 * Preconditions: current logical service access result has been captured.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP StatusType Os_ArtiAccessStatus(void);

OS_MEMMAP ISRType Os_Implementation_GetISRID(void);

OS_MEMMAP StatusType Os_Implementation_ControlIdle(CoreIdType CoreID, IdleModeType IdleMode);

OS_MEMMAP CoreIdType Os_Implementation_GetCoreID(void);

OS_MEMMAP StatusType Os_Implementation_GetTaskID(TaskRefType TaskID);

OS_MEMMAP StatusType Os_Implementation_GetTaskState(TaskType TaskID, TaskStateRefType State);

OS_MEMMAP StatusType Os_Implementation_ActivateTask(TaskType TaskID);

OS_MEMMAP StatusType Os_Implementation_ChainTask(TaskType TaskID);

OS_MEMMAP StatusType Os_Implementation_GetResource(ResourceType ResID);

OS_MEMMAP StatusType Os_Implementation_ReleaseResource(ResourceType ResID);

OS_MEMMAP StatusType Os_Implementation_Schedule(void);

OS_MEMMAP StatusType Os_Implementation_WaitEvent(EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_ClearEvent(EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_SetEvent(TaskType TaskID, EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_GetEvent(TaskType TaskID, EventMaskRefType Event);

OS_MEMMAP void Os_Implementation_ShutdownOS(StatusType Error);

OS_MEMMAP void Os_Implementation_StartOS(AppModeType Mode);

OS_MEMMAP AppModeType Os_Implementation_GetActiveApplicationMode(void);

OS_MEMMAP StatusType Os_Implementation_IncrementCounter(CounterType CounterID);

OS_MEMMAP StatusType Os_Implementation_GetCounterValue(CounterType CounterID, TickRefType Value);

OS_MEMMAP StatusType Os_Implementation_GetElapsedValue(CounterType CounterID,
                                                       TickRefType Value,
                                                       TickRefType ElapsedValue);

OS_MEMMAP StatusType Os_Implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info);

OS_MEMMAP StatusType Os_Implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick);

OS_MEMMAP StatusType Os_Implementation_SetRelAlarm(AlarmType AlarmID,
                                                   TickType Increment,
                                                   TickType Cycle);

OS_MEMMAP StatusType Os_Implementation_SetAbsAlarm(AlarmType AlarmID,
                                                   TickType Start,
                                                   TickType Cycle);

OS_MEMMAP StatusType Os_Implementation_CancelAlarm(AlarmType AlarmID);

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                             TickType Offset);

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                             TickType Start);

OS_MEMMAP StatusType Os_Implementation_StopScheduleTable(ScheduleTableType ScheduleTableID);

OS_MEMMAP StatusType Os_Implementation_GetScheduleTableStatus(
    ScheduleTableType ScheduleTableID, ScheduleTableStatusRefType ScheduleStatus);

OS_MEMMAP StatusType Os_Implementation_DisableInterruptSource(ISRType ISRID);

OS_MEMMAP StatusType Os_Implementation_EnableInterruptSource(ISRType ISRID, boolean ClearPending);

OS_MEMMAP StatusType Os_Implementation_ClearPendingInterrupt(ISRType ISRID);

OS_MEMMAP void Os_Implementation_EnableAllInterrupts(void);

OS_MEMMAP void Os_Implementation_DisableAllInterrupts(void);

OS_MEMMAP void Os_Implementation_ResumeAllInterrupts(void);

OS_MEMMAP void Os_Implementation_SuspendAllInterrupts(void);

OS_MEMMAP void Os_Implementation_ResumeOSInterrupts(void);

OS_MEMMAP void Os_Implementation_SuspendOSInterrupts(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Service.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Service.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
