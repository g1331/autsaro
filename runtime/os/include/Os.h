/** @file Public AUTOSAR OS API for the single-core target, with OSEK compatibility macros. */
#ifndef AUTOSAR_OS_H
#define AUTOSAR_OS_H
#include "Os_Version.h"
#include "Os_Types.h"
#include "Rte_Os_Type.h"
#include "Os_Hooks.h"
#include "Os_Service.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "Os.h -> Rte_Os_Type.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os.h -> Rte_Os_Type.h: incompatible AUTOSAR release"
#endif
/* OSEK source compatibility only (R24-11 section 7.1.2.1). The unused tag
 * deliberately consumes a legacy file-scope semicolon in strict C99. It has
 * no object, function, configuration or argument-evaluation effect. R2.4
 * Advisory: this incomplete tag exists solely for the required interface. */
/* MR12 RULE 2.5 VIOLATION: OS 7.1.2.1 requires this no-effect compatibility construction macro. */
/** Declare the standard Task spelling (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured Task C identifier; not evaluated.
 * @return An incomplete compatibility tag declaration, with no runtime object or evaluation.
 * Preconditions: file-scope use followed by a semicolon; generated configuration supplies the
 * object binding. Context: C preprocessing/declaration only; no runtime context. Reentrancy: not
 * applicable; no runtime operation. Ownership: declares no storage and retains no pointer.
 */
#define DeclareTask(identifier) struct Os_DeclarationCompatibility
/* MR12 RULE 2.5 VIOLATION: OS 7.1.2.1 requires this no-effect compatibility construction macro. */
/** Declare the standard resource spelling (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured resource C identifier; not evaluated.
 * @return An incomplete compatibility tag declaration, with no runtime object or evaluation.
 * Preconditions: file-scope use followed by a semicolon; generated configuration supplies the
 * object binding. Context: C preprocessing/declaration only; no runtime context. Reentrancy: not
 * applicable; no runtime operation. Ownership: declares no storage and retains no pointer.
 */
#define DeclareResource(identifier) struct Os_DeclarationCompatibility
/* MR12 RULE 2.5 VIOLATION: OS 7.1.2.1 requires this no-effect compatibility construction macro. */
/** Declare the standard event spelling (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured event C identifier; not evaluated.
 * @return An incomplete compatibility tag declaration, with no runtime object or evaluation.
 * Preconditions: file-scope use followed by a semicolon; generated configuration supplies the
 * object binding. Context: C preprocessing/declaration only; no runtime context. Reentrancy: not
 * applicable; no runtime operation. Ownership: declares no storage and retains no pointer.
 */
#define DeclareEvent(identifier) struct Os_DeclarationCompatibility
/* MR12 RULE 2.5 VIOLATION: OS 7.1.2.1 requires this no-effect compatibility construction macro. */
/** Declare the standard Alarm spelling (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured Alarm C identifier; not evaluated.
 * @return An incomplete compatibility tag declaration, with no runtime object or evaluation.
 * Preconditions: file-scope use followed by a semicolon; generated configuration supplies the
 * object binding. Context: C preprocessing/declaration only; no runtime context. Reentrancy: not
 * applicable; no runtime operation. Ownership: declares no storage and retains no pointer.
 */
#define DeclareAlarm(identifier) struct Os_DeclarationCompatibility
/* Standard OSEK entry bodies. Token pasting keeps the object spelling even
 * when its generated identifier is a numeric macro (R20.10 Advisory: source
 * compatibility requires the same identifier for the body and its binding).
 * A preceding prototype also covers the normal definition-only use. */
/** Define a standard Task entry body (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured entry C identifier; not evaluated at runtime.
 * @return A void entry prototype and definition head; follow it with the function body.
 * Preconditions: file-scope definition for a configured entry; use the matching generated binding.
 * Context: C preprocessing only; the resulting body follows the Task calling rules.
 * Reentrancy: macro expansion has no state; entry reentrancy follows OS scheduling.
 * Ownership: no runtime argument or storage transfer.
 */
#define TASK(identifier)                                                                           \
    OS_CODE void Os_TaskEntry_##identifier(void);                                                  \
    OS_CODE void Os_TaskEntry_##identifier(void)
/** Refer to the native body for a configured Task.
 * @param[in] identifier Same configured C identifier used by the entry body macro.
 * @return Function designator for the corresponding native entry.
 * Preconditions: matching entry body is declared/defined.
 * Context: C expression/configuration initializer; no runtime call.
 * Reentrancy: no state; repeatable expansion.
 * Ownership: does not invoke the body or transfer pointer ownership.
 */
#define OS_TASK_ENTRY(identifier) Os_TaskEntry_##identifier
/* MR12 RULE 2.5 VIOLATION: OSEK 13.3.3 requires the ISR definition macro; its mapping remains
 * intact. */
/** Define a standard Category 2 ISR entry body (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured entry C identifier; not evaluated at runtime.
 * @return A void entry prototype and definition head; follow it with the function body.
 * Preconditions: file-scope definition for a configured entry; use the matching generated binding.
 * Context: C preprocessing only; the resulting body follows the Category 2 ISR calling rules.
 * Reentrancy: macro expansion has no state; entry reentrancy follows OS scheduling.
 * Ownership: no runtime argument or storage transfer.
 */
#define ISR(identifier)                                                                            \
    OS_CODE void Os_IsrEntry_##identifier(void);                                                   \
    OS_CODE void Os_IsrEntry_##identifier(void)
/** Refer to the native body for a configured ISR.
 * @param[in] identifier Same configured C identifier used by the entry body macro.
 * @return Function designator for the corresponding native entry.
 * Preconditions: matching entry body is declared/defined.
 * Context: C expression/configuration initializer; no runtime call.
 * Reentrancy: no state; repeatable expansion.
 * Ownership: does not invoke the body or transfer pointer ownership.
 */
#define OS_ISR_ENTRY(identifier) Os_IsrEntry_##identifier
/* MR12 RULE 2.5 VIOLATION: OSEK 13.6.5 requires this alarm-callback definition macro. */
/** Define a standard Alarm callback entry body (OSEK OS 2.2.3 chapter 13).
 * @param[in] identifier Configured entry C identifier; not evaluated at runtime.
 * @return A void entry prototype and definition head; follow it with the function body.
 * Preconditions: file-scope definition for a configured entry; use the matching generated binding.
 * Context: C preprocessing only; the resulting body follows the Alarm callback calling rules.
 * Reentrancy: macro expansion has no state; entry reentrancy follows OS scheduling.
 * Ownership: no runtime argument or storage transfer.
 */
#define ALARMCALLBACK(identifier)                                                                  \
    OS_CODE void identifier(void);                                                                 \
    OS_CODE void identifier(void)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** @brief Select the supported virtual-core idle mode.
 * @param[in] CoreID Ignored by this single-core target, including foreign values.
 * @param[in] IdleMode IDLE_NO_HALT; other modes return E_OS_ID without an effect.
 * @return E_OK, or the standard context/disabled-interrupt/invalid-mode error.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: non-reentrant (SWS_Os_00769); callers serialize mode selection.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType ControlIdle(CoreIdType CoreID, IdleModeType IdleMode)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ControlIdle, (uint32)IdleMode, 0);
    }
    const StatusType result = Os_Implementation_ControlIdle(CoreID, IdleMode);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ControlIdle, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** @brief R24-11 DRAFT query of whether StartOS has been called.
 * @return TRUE after entry to StartOS; FALSE before it or when the call is ignored.
 * @note The draft API can be used before startup in an initialized C environment.
 * Preconditions: initialized C environment; masked calls return FALSE.
 * Context: any Task, ISR or supported hook, and before StartOS; outside BSW exclusive areas.
 * Reentrancy: reentrant; the startup latch is atomic.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP boolean isOsStarted(void);

/** @brief Read the logical CoreID of the calling core (SWS_Os_00674, 00675).
 * @return OS_CORE_ID_0, the EcucCoreId of the only core of this target.
 * Preconditions: none; callable before StartOS (SWS_Os_00625) and with interrupts disabled.
 * Context: any Task, ISR, Hook or Alarm callback, and initialized C code before StartOS; outside
 * BSW exclusive areas.
 * Reentrancy: reentrant.
 * Ownership: none.
 */
OS_MEMMAP static inline CoreIdType GetCoreID(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetCoreID, (uint32)0u, 0);
    }
    const CoreIdType result = Os_Implementation_GetCoreID();
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)E_OK);
        Os_ArtiService(OSServiceId_GetCoreID, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Restore the caller's non-nested DisableAllInterrupts state; unmatched call has no effect.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. The All pair is also available in initialized startup/closing C code. Reentrancy:
 * different logical callers have separate nesting; same-caller pairs must remain balanced.
 * Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void EnableAllInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_EnableAllInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Mask all application interrupts. This pair does not support nesting.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. The All pair is also available in initialized startup/closing C code. Reentrancy:
 * different logical callers have separate nesting; same-caller pairs must remain balanced.
 * Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void DisableAllInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_DisableAllInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Restore one nesting level of the caller's all-interrupt suspension.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. The All pair is also available in initialized startup/closing C code. Reentrancy:
 * different logical callers have separate nesting; same-caller pairs must remain balanced.
 * Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void ResumeAllInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_ResumeAllInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Mask all application interrupts; nesting with the All/OS suspend pairs is supported.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. The All pair is also available in initialized startup/closing C code. Reentrancy:
 * different logical callers have separate nesting; same-caller pairs must remain balanced.
 * Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void SuspendAllInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_SuspendAllInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Restore one nesting level of the caller's Category 2 interrupt suspension.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. Reentrancy: different logical callers have separate nesting; same-caller pairs must remain
 * balanced. Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void ResumeOSInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_ResumeOSInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Mask Category 2 application interrupts, preserving Category 1 delivery.
 * @par Return
 * None.
 * Preconditions: initialized C environment; unmatched restores and refused calls have no effect.
 * Context: Task, ISR or supported hook/AlarmCallback (R24-11 Table 7.1); caller pairs its own
 * masks. Reentrancy: different logical callers have separate nesting; same-caller pairs must remain
 * balanced. Ownership: no pointer arguments; mask ownership stays with the logical caller.
 */
OS_MEMMAP static inline void SuspendOSInterrupts(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_SuspendOSInterrupts();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

/** Read the active logical Category 2 ISR, including its ErrorHook.
 * @return ISR vector identifier, or INVALID_ISR outside a Category 2 ISR.
 * Preconditions: caller interrupts enabled; masked or non-ISR calls return INVALID_ISR.
 * Context: Task, Category 2 ISR or ErrorHook; outside BSW exclusive areas.
 * Reentrancy: reentrant; observes the active logical ISR.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline ISRType GetISRID(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetISRID, (uint32)0u, 0);
    }
    const ISRType result = Os_Implementation_GetISRID();
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
        Os_ArtiService(
            OSServiceId_GetISRID, (Os_ArtiAccessValid() != 0) ? (uint32)result : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Enable a known Category 2 source; optionally clear its real pending bit.
 * @param[in] ISRID Logical source/vector identifier.
 * @param[in] ClearPending TRUE to consume the pending edge before enabling.
 * @return E_OK, E_OS_ID, E_OS_CALLEVEL, or E_OS_NOFUNC if already enabled.
 * @note Rejected calls preserve source state and pending, including ClearPending=TRUE.
 * Preconditions: configured Category 2 source; invalid identifiers and contexts return the
 * documented error. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Existing All/OS masks do not reject source-control calls. Reentrancy: reentrant across Tasks and
 * Category 2 ISRs; mutations use the OS critical section. Ownership: scalar arguments are copied;
 * no application storage is retained.
 */
OS_MEMMAP static inline StatusType EnableInterruptSource(ISRType ISRID, boolean ClearPending)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_EnableInterruptSource(ISRID, ClearPending);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Disable a known Category 2 source, retaining any real pending edge.
 * @param[in] ISRID Logical source/vector identifier.
 * @return E_OK, E_OS_ID, E_OS_CALLEVEL, or E_OS_NOFUNC if already disabled.
 * Preconditions: configured Category 2 source; invalid identifiers and contexts return the
 * documented error. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Existing All/OS masks do not reject source-control calls. Reentrancy: reentrant across Tasks and
 * Category 2 ISRs; mutations use the OS critical section. Ownership: scalar arguments are copied;
 * no application storage is retained.
 */
OS_MEMMAP static inline StatusType DisableInterruptSource(ISRType ISRID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_DisableInterruptSource(ISRID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Clear a known Category 2 source's real pending bit without enabling it.
 * @param[in] ISRID Logical source/vector identifier.
 * @return E_OK, E_OS_ID or E_OS_CALLEVEL.
 * Preconditions: configured Category 2 source; invalid identifiers and contexts return the
 * documented error. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Existing All/OS masks do not reject source-control calls. Reentrancy: reentrant across Tasks and
 * Category 2 ISRs; mutations use the OS critical section. Ownership: scalar arguments are copied;
 * no application storage is retained.
 */
OS_MEMMAP static inline StatusType ClearPendingInterrupt(ISRType ISRID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_ClearPendingInterrupt(ISRID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Start a NONE-synchronized table at a positive relative delay.
 * @param[in] ScheduleTableID Configured table identifier.
 * @param[in] Offset Delay; Offset plus initial offset must fit counter maximum.
 * @return E_OK or E_OS_ID/VALUE/STATE/CALLEVEL; refusal preserves state.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                         TickType Offset)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableRel(ScheduleTableID, Offset);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Start at the next occurrence of an absolute counter value.
 * @param[in] ScheduleTableID Configured table identifier.
 * @param[in] Start Value within counter maximum; equality waits a full wrap.
 * @return E_OK or E_OS_ID/VALUE/STATE/CALLEVEL; refusal preserves state.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                         TickType Start)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableAbs(ScheduleTableID, Start);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Stop a running or reserved NEXT table and detach its successor.
 * @param[in] ScheduleTableID Configured table identifier.
 * @return E_OK or E_OS_ID/NOFUNC/CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType StopScheduleTable(ScheduleTableType ScheduleTableID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StopScheduleTable(ScheduleTableID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Reserve a stopped table to follow the running source's final delay.
 * @param[in] ScheduleTableID_From Running source table.
 * @param[in] ScheduleTableID_To Stopped destination on the same counter/strategy.
 * @return E_OK or E_OS_ID/NOFUNC/STATE/CALLEVEL; refusal preserves both.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP StatusType NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                       ScheduleTableType ScheduleTableID_To);

/** Copy the table's current state within the existing OS critical boundary.
 * @param[in] ScheduleTableID Configured table identifier.
 * @param[out] ScheduleStatus Nonnull caller-owned output, unchanged on refusal.
 * @return E_OK or E_OS_ID/ILLEGAL_ADDRESS/CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: caller retains output storage, borrowed until return and written only on success.
 */
OS_MEMMAP static inline StatusType GetScheduleTableStatus(ScheduleTableType ScheduleTableID,
                                                          ScheduleTableStatusRefType ScheduleStatus)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetScheduleTableStatus, (uint32)ScheduleTableID, 0);
    }
    const StatusType result =
        Os_Implementation_GetScheduleTableStatus(ScheduleTableID, ScheduleStatus);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetScheduleTableStatus,
                       (result == E_OK) ? (uint32)*ScheduleStatus : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Start the configured OS in mode; successful startup never returns.
 * @param[in] Mode Static application mode (1 or 2).
 * @par Return
 * None.
 * Preconditions: successful Os_TargetPrepare, valid application mode and first startup; invalid
 * first startup shuts down. Context: initialized startup thread before scheduling, outside
 * exclusive areas; hook/repeated calls are ignored. Reentrancy: non-reentrant; the process has one
 * OS startup. Ownership: retains the configuration already published by Os_TargetPrepare; scalar
 * Mode is copied.
 */
OS_MEMMAP static inline void StartOS(AppModeType Mode)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_StartOS, (uint32)Mode, 0);
    }
    Os_Implementation_StartOS(Mode);
    Os_ArtiServiceLeave();
}

/** Close irreversibly; invoke ShutdownHook on the established control stack,
 * or on the startup caller before control resources exist.
 * @param[in] Error Standard shutdown reason.
 * @par Return
 * None.
 * Preconditions: service access permitted and caller interrupts enabled; a refused call is ignored.
 * Context: Task, Category 2 ISR, StartupHook or ErrorHook, outside BSW exclusive areas.
 * Reentrancy: shutdown admission is atomic; the first accepted close wins.
 * Ownership: no caller pointer is retained; accepted shutdown does not return.
 */
OS_MEMMAP static inline void ShutdownOS(StatusType Error)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ShutdownOS, (uint32)Error, 0);
    }
    Os_Implementation_ShutdownOS(Error);
    Os_ArtiServiceLeave();
}

/** Read the automotive task selected by the actual kernel.
 * @param[out] TaskID Nonnull caller-owned output; INVALID_TASK if none is selected.
 * @return E_OK, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS; refusal preserves output.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task, Category 2 ISR, ErrorHook, PreTaskHook or PostTaskHook; outside
 * BSW exclusive areas. Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS
 * critical section. Ownership: caller retains writable output storage; borrowed only until return
 * and unchanged on refusal.
 */
OS_MEMMAP static inline StatusType GetTaskID(TaskRefType TaskID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetTaskID, (uint32)0u, 0);
    }
    const StatusType result = Os_Implementation_GetTaskID(TaskID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetTaskID, (result == E_OK) ? (uint32)*TaskID : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Read the application mode selected by StartOS, including in global hooks.
 * @return Selected mode; zero before StartOS has selected a mode.
 * Preconditions: enabled caller interrupts; refused access returns zero.
 * Context: Task, Category 2 ISR or supported global hook, outside BSW exclusive areas.
 * Reentrancy: reentrant; reads the selected startup mode.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline AppModeType GetActiveApplicationMode(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetActiveApplicationMode, (uint32)0u, 0);
    }
    const AppModeType result = Os_Implementation_GetActiveApplicationMode();
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
        Os_ArtiService(OSServiceId_GetActiveApplicationMode,
                       (Os_ArtiAccessValid() != 0) ? (uint32)result : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Query actual backend task state.
 * @param[in] TaskID Configured task identifier.
 * @param[out] State Output state, must not be null.
 * @return E_OK, E_OS_ID, E_OS_CALLEVEL, E_OS_ILLEGAL_ADDRESS or E_OS_STATE.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task, Category 2 ISR, ErrorHook, PreTaskHook or PostTaskHook; outside
 * BSW exclusive areas. Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS
 * critical section. Ownership: caller retains writable output storage; borrowed only until return
 * and unchanged on refusal.
 */
OS_MEMMAP static inline StatusType GetTaskState(TaskType TaskID, TaskStateRefType State)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetTaskState, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetTaskState(TaskID, State);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetTaskState, (result == E_OK) ? (uint32)*State : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Accept one activation request in Task or Category 2 ISR context.
 * @param[in] TaskID Configured task identifier.
 * @return E_OK, E_OS_ID, E_OS_LIMIT or E_OS_CALLEVEL; rejection preserves requests.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType ActivateTask(TaskType TaskID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ActivateTask, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ActivateTask(TaskID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ActivateTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Complete the current activation. Success never returns to the application.
 * @return E_OS_CALLEVEL, E_OS_RESOURCE or E_OS_STATE on rejection; success never returns.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Caller meets the task/resource restrictions stated above. Context: current Task
 * only, outside BSW exclusive areas; blocking/termination requires no external resources.
 * Reentrancy: per-Task operation; independent Tasks may call concurrently, serialized by the OS.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP StatusType TerminateTask(void);

/** Complete this activation and accept a sequential successor atomically.
 * @param[in] TaskID Configured successor; self-chain does not consume extra capacity.
 * @return E_OS_ID, E_OS_LIMIT, E_OS_RESOURCE or E_OS_CALLEVEL on rejection.
 * Successful calls never return to the completed application frame.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Caller meets the task/resource restrictions stated above. Context: current Task
 * only, outside BSW exclusive areas; blocking/termination requires no external resources.
 * Reentrancy: per-Task operation; independent Tasks may call concurrently, serialized by the OS.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType ChainTask(TaskType TaskID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ChainTask, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ChainTask(TaskID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ChainTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Acquire a configured external resource using its static priority ceiling.
 * @param[in] ResID Resource accessible to this logical Task/ISR caller.
 * @return E_OK, E_OS_ID, E_OS_ACCESS or E_OS_CALLEVEL; no blocking acquisition.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Resource is configured for this caller; release follows LIFO ownership.
 * Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: resource ownership belongs to the logical Task/ISR until matching release; no pointer
 * retained.
 */
OS_MEMMAP static inline StatusType GetResource(ResourceType ResID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetResource, (uint32)ResID, 0);
    }
    const StatusType result = Os_Implementation_GetResource(ResID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetResource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Release the last external resource acquired by this logical Task/ISR caller.
 * @param[in] ResID Last owned resource, in LIFO order.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Resource is configured for this caller; release follows LIFO ownership.
 * Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: resource ownership belongs to the logical Task/ISR until matching release; no pointer
 * retained.
 */
OS_MEMMAP static inline StatusType ReleaseResource(ResourceType ResID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32)ResID, 0);
    }
    const StatusType result = Os_Implementation_ReleaseResource(ResID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Explicitly release/reacquire the current task's internal resource.
 * @return E_OK, E_OS_CALLEVEL or E_OS_RESOURCE; no effect for a FULL task without an internal
 * resource.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Caller meets the task/resource restrictions stated above. Context: current Task
 * only, outside BSW exclusive areas; blocking/termination requires no external resources.
 * Reentrancy: per-Task operation; independent Tasks may call concurrently, serialized by the OS.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType Schedule(void)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_Schedule, (uint32)0u, 0);
    }
    const StatusType result = Os_Implementation_Schedule();
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_Schedule, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Wait until any requested stored event is set; the current instance is retained.
 * @param[in] Mask Event predicate, including zero for an indefinite wait.
 * @return E_OK, E_OS_ACCESS, E_OS_RESOURCE or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Caller meets the task/resource restrictions stated above. Context: current Task
 * only, outside BSW exclusive areas; blocking/termination requires no external resources.
 * Reentrancy: per-Task operation; independent Tasks may call concurrently, serialized by the OS.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType WaitEvent(EventMaskType Mask)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_WaitEvent, (uint32)Mask, 0);
    }
    const StatusType result = Os_Implementation_WaitEvent(Mask);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_WaitEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Clear stored bits owned by the current Extended task.
 * @param[in] Mask Bits to clear.
 * @return E_OK, E_OS_ACCESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Caller meets the task/resource restrictions stated above. Context: current Task
 * only, outside BSW exclusive areas; blocking/termination requires no external resources.
 * Reentrancy: per-Task operation; independent Tasks may call concurrently, serialized by the OS.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType ClearEvent(EventMaskType Mask)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_ClearEvent, (uint32)Mask, 0);
    }
    const StatusType result = Os_Implementation_ClearEvent(Mask);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_ClearEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define SetEvent Os_SetEvent /**< Standard name bound to the host-safe OS entry. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Set stored event bits of an active Extended task; repeated setting merges bits.
 * @param[in] TaskID Extended task identifier.
 * @param[in] Mask Bits to publish.
 * @return E_OK, E_OS_ID, E_OS_ACCESS, E_OS_STATE or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType Os_SetEvent(TaskType TaskID, EventMaskType Mask)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SetEvent, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_SetEvent(TaskID, Mask);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_SetEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}
/** Read the stored events of an active Extended task.
 * @param[in] TaskID Extended task identifier.
 * @param[out] Event Caller-owned non-null output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ACCESS, E_OS_STATE, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task, Category 2 ISR, ErrorHook, PreTaskHook or PostTaskHook; outside
 * BSW exclusive areas. Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS
 * critical section. Ownership: caller retains writable output storage; borrowed only until return
 * and unchanged on refusal.
 */
OS_MEMMAP static inline StatusType GetEvent(TaskType TaskID, EventMaskRefType Event)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetEvent, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetEvent(TaskID, Event);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetEvent, (result == E_OK) ? (uint32)*Event : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Increment a software Counter and execute due Alarm actions.
 * @param[in] CounterID Configured software Counter.
 * @return E_OK, E_OS_ID or E_OS_CALLEVEL; action errors invoke configured ErrorHook.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType IncrementCounter(CounterType CounterID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_IncrementCounter(CounterID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Read a Counter's current modular value.
 * @param[in] CounterID Configured Counter.
 * @param[out] Value Non-null output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: caller retains result storage; borrowed until return, updated only on success.
 */
OS_MEMMAP static inline StatusType GetCounterValue(CounterType CounterID, TickRefType Value)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetCounterValue, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetCounterValue(CounterID, Value);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(
            OSServiceId_GetCounterValue, (result == E_OK) ? (uint32)*Value : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Read a modular elapsed value and update the prior sample.
 * @param[in] CounterID Configured Counter.
 * @param[in,out] Value Prior sample on input, current value on success.
 * @param[out] ElapsedValue Difference within one modulus; multiple wraps are not detectable.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: caller retains result storage; borrowed until return, updated only on success.
 */
OS_MEMMAP static inline StatusType GetElapsedValue(CounterType CounterID,
                                                   TickRefType Value,
                                                   TickRefType ElapsedValue)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetElapsedValue, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetElapsedValue(CounterID, Value, ElapsedValue);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(
            OSServiceId_GetElapsedValue, (result == E_OK) ? (uint32)*ElapsedValue : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Read immutable Alarm Counter properties.
 * @param[in] AlarmID Configured Alarm.
 * @param[out] Info Non-null output.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task, Category 2 ISR, ErrorHook, PreTaskHook or PostTaskHook; outside
 * BSW exclusive areas. Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS
 * critical section. Ownership: caller retains writable output storage; borrowed only until return
 * and unchanged on refusal.
 */
OS_MEMMAP static inline StatusType GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetAlarmBase, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarmBase(AlarmID, Info);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        if (result == E_OK) {
            Os_ArtiAlarmBaseReturn(Info);
        } else {
            Os_ArtiService(OSServiceId_GetAlarmBase, UINT32_MAX, 1);
        }
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Read ticks remaining until the active Alarm expires.
 * @param[in] AlarmID Configured Alarm.
 * @param[out] Tick Non-null output, unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task, Category 2 ISR, ErrorHook, PreTaskHook or PostTaskHook; outside
 * BSW exclusive areas. Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS
 * critical section. Ownership: caller retains writable output storage; borrowed only until return
 * and unchanged on refusal.
 */
OS_MEMMAP static inline StatusType GetAlarm(AlarmType AlarmID, TickRefType Tick)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_GetAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarm(AlarmID, Tick);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_GetAlarm, (result == E_OK) ? (uint32)*Tick : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Start a relative Alarm.
 * @param[in] AlarmID Configured Alarm.
 * @param[in] Increment Positive distance within the Counter maximum.
 * @param[in] Cycle Zero for one-shot, or a valid cycle.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_STATE or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType SetRelAlarm(AlarmType AlarmID,
                                               TickType Increment,
                                               TickType Cycle)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetRelAlarm(AlarmID, Increment, Cycle);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Start an absolute Alarm; the current value expires only after a complete modulus.
 * @param[in] AlarmID Configured Alarm.
 * @param[in] Start Absolute value within the Counter maximum.
 * @param[in] Cycle Zero for one-shot, or a valid cycle.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_STATE or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetAbsAlarm(AlarmID, Start, Cycle);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Cancel an active Alarm.
 * @param[in] AlarmID Configured Alarm.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC or E_OS_CALLEVEL.
 * Preconditions: configured OS and enabled caller interrupts; masked access returns
 * E_OS_DISABLEDINT. Context: Task or Category 2 ISR, outside BSW exclusive areas; not from hooks.
 * Reentrancy: reentrant across Tasks and Category 2 ISRs; mutations use the OS critical section.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP static inline StatusType CancelAlarm(AlarmType AlarmID)
{
    const sint32 service_scope = Os_ArtiServiceEnter();
    if (service_scope != 0) {
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_CancelAlarm(AlarmID);
    if (service_scope != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

/** Initialize application services during OS startup (OSEK OS 2.2.3 13.8.2.4).
 * @par Return
 * None.
 * Preconditions: invoked only by the OS in its selected hook phase.
 * Context: OS bootstrap before readiness and periodic resource release, outside BSW exclusive
 * areas. Reentrancy: non-reentrant; one startup/shutdown per process. Ownership: scalar arguments
 * are copied; no application storage is retained.
 */
OS_CODE void StartupHook(void);
/** Retire application services during irreversible shutdown (OSEK OS 2.2.3 13.8.2.5).
 * @param[in] Error Shutdown reason selected by the accepted close.
 * @par Return
 * None.
 * Preconditions: invoked only by the OS in its selected hook phase.
 * Context: healthy control actor, or startup caller before controllers exist; no ordinary port
 * mutex is acquired. Reentrancy: non-reentrant; one startup/shutdown per process. Ownership: scalar
 * arguments are copied; no application storage is retained.
 */
OS_CODE void ShutdownHook(StatusType Error);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
