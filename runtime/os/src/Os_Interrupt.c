/** @file Interrupt masking, suspend/resume and interrupt source control services. */
#include "Os_Backend.h"
#include <limits.h>

typedef struct {
    uint32 all_depth;
    uint32 os_depth;
    uint8 disabled;
} Os_InterruptStateType;
/* One cell per physical Task/boot actor, and separate logical ISR cells on
 * the native ISR thread. This preserves ownership across interrupted actors. */
#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL Os_InterruptStateType
    states[33u * ((uint32)OS_HOOK_ALARM +
                  1u)]; /**< Actor TLS logical-caller mask depths; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Interrupt.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Interrupt.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    all_owners; /**< Atomic outer All-mask owner count; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    os_owners; /**< Atomic outer OS-mask owner count; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static Os_InterruptStateType *owner_state(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 interrupt = Os_BackendCurrentInterrupt();
    const Os_HookPhaseType phase = Os_HookContext();
    uint32 index = ((stack != NULL) && (stack->role == 'S') && (interrupt < 32u)) ? interrupt : 32u;
    if ((phase >= OS_HOOK_ERROR) && (phase <= OS_HOOK_ALARM)) {
        /* Each Hook has its own mask ownership, including its logical ISR.
         * Queries in ErrorHook do not inherit the failed caller's mask. */
        index += 33u * (uint32)phase;
    }
    return &states[index];
}

OS_MEMMAP sint32 Os_InterruptDisabled(void)
{
    const Os_InterruptStateType *state = owner_state();
    return ((state->disabled != 0u) || (state->all_depth != 0u) || (state->os_depth != 0u)) ? 1 : 0;
}

OS_MEMMAP sint32 Os_InterruptAllows(uint32 interrupt)
{
    sint32 allowed = 0;
    if (interrupt < 32u) {
        if (InterlockedCompareExchange(&all_owners, 0, 0) == 0) {
            allowed = ((InterlockedCompareExchange(&os_owners, 0, 0) == 0) ||
                       ((Os_Config != NULL) &&
                        ((Os_Config->category1_isrs & ((uint32)1u << interrupt)) != 0u)))
                          ? 1
                          : 0;
        }
    }
    return allowed;
}

OS_MEMMAP static sint32 permitted(uint32 operation)
{
    const Os_HookPhaseType phase = Os_HookContext();
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 group = operation / 2u;
    sint32 allowed = 0;
    if ((phase >= OS_HOOK_ERROR) && (phase <= OS_HOOK_ALARM)) {
        /* R24-11 Table 7.1 permits all six primitives from every supported
         * global Hook and Alarm callback, extending the older OSEK table. */
        allowed = 1;
    } else if (phase == OS_HOOK_NONE) {
        if (group != 2u) {
            allowed = (Os_BackendStarted() == 0) ? 1 : 0;
            if (allowed == 0) {
                allowed = (InterlockedCompareExchange(&Os_Closing, 0, 0) != 0) ? 1 : 0;
            }
        }
        if ((allowed == 0) && (stack != NULL)) {
            allowed = ((stack->role == 'T') ||
                       ((stack->role == 'S') && (Os_BackendCurrentInterrupt() < 32u)))
                          ? 1
                          : 0;
        }
    } else {
        /* A registered nonautomotive actor is not a Task or active ISR. */
    }
    return allowed;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_Os_00367: these services return no StatusType, so a refused call
 * (wrong context, or a saturated nesting counter) is ignored without ErrorHook. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void update_owner_mask(Os_InterruptStateType *state, uint32 operation)
{
    switch (operation) {
        case 0u:
            if (state->disabled != 0u) {
                state->disabled = 0u;
                (void)InterlockedDecrement(&all_owners);
            }
            break;
        case 1u:
            if (state->disabled == 0u) {
                state->disabled = 1u;
                (void)InterlockedIncrement(&all_owners);
            }
            break;
        case 2u:
            if (state->all_depth != 0u) {
                --state->all_depth;
                if (state->all_depth == 0u) {
                    (void)InterlockedDecrement(&all_owners);
                }
            }
            break;
        case 3u:
            if (state->all_depth != UINT32_MAX) {
                if (state->all_depth == 0u) {
                    (void)InterlockedIncrement(&all_owners);
                }
                ++state->all_depth;
            }
            break;
        case 4u:
            if (state->os_depth != 0u) {
                --state->os_depth;
                if (state->os_depth == 0u) {
                    (void)InterlockedDecrement(&os_owners);
                }
            }
            break;
        default:
            if (state->os_depth != UINT32_MAX) {
                if (state->os_depth == 0u) {
                    (void)InterlockedIncrement(&os_owners);
                }
                ++state->os_depth;
            }
            break;
    }
}

OS_MEMMAP static void change(uint32 operation)
{
    Os_InterruptStateType *state = owner_state();
    boolean running = FALSE;
    if (Os_BackendStarted() != 0) {
        running = InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
    }
    Os_StackCheck();
    Os_BackendGuardService();
    if (permitted(operation) != 0) {
        if (running) {
            taskENTER_CRITICAL();
        }
        update_owner_mask(state, operation);
        if (running) {
            taskEXIT_CRITICAL();
        }
    }
}

OS_MEMMAP void Os_Implementation_EnableAllInterrupts(void) { change(0u); }

OS_MEMMAP void Os_Implementation_DisableAllInterrupts(void) { change(1u); }

OS_MEMMAP void Os_Implementation_ResumeAllInterrupts(void) { change(2u); }

OS_MEMMAP void Os_Implementation_SuspendAllInterrupts(void) { change(3u); }

OS_MEMMAP void Os_Implementation_ResumeOSInterrupts(void) { change(4u); }

OS_MEMMAP void Os_Implementation_SuspendOSInterrupts(void) { change(5u); }

OS_MEMMAP void Os_InterruptRestoreOwner(void)
{
    Os_InterruptStateType *state = owner_state();
    /* Internal exit cleanup is called under the real OS completion critical
     * section, so no ISR can observe partial restoration. */
    if (state->disabled != 0u) {
        (void)InterlockedDecrement(&all_owners);
    }
    if (state->all_depth != 0u) {
        (void)InterlockedDecrement(&all_owners);
    }
    if (state->os_depth != 0u) {
        (void)InterlockedDecrement(&os_owners);
    }
    state->disabled = 0u;
    state->all_depth = 0u;
    state->os_depth = 0u;
}

OS_MEMMAP static StatusType source_control(ISRType interrupt, uint32 operation, boolean clear)
{
    StatusType result = E_OK;
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 current = Os_BackendCurrentInterrupt();
    Os_StackCheck();
    Os_BackendGuardService();
    /* Source operations are interrupt services, so application All/OS masks
     * do not reject them. Hooks and Category 1 callers cannot change sources. */
    sint32 serving = 0;
    if (Os_HookContext() == OS_HOOK_NONE) {
        serving = Os_BackendServiceContext();
    }
    if ((serving == 0) || ((stack != NULL) && (stack->role == 'S') &&
                           ((current >= 32u) || (current == OS_KERNEL_YIELD_INTERRUPT)))) {
        result = E_OS_CALLEVEL;
    } else if ((interrupt >= 32u) || (interrupt == OS_KERNEL_YIELD_INTERRUPT) ||
               (interrupt == OS_CONTROLLED_TICK_INTERRUPT) ||
               ((Os_Config->category1_isrs & ((uint32)1u << interrupt)) != 0u)) {
        result = E_OS_ID;
    } else {
        taskENTER_CRITICAL();
        Os_BackendGuardService();
        const sint32 source_result = Os_PortInterruptSourceControl(interrupt, operation, clear);
        if (source_result == 0) {
            result = E_OS_ID;
        } else if (source_result == 2) {
            result = E_OS_NOFUNC;
        } else {
            /* Successful source control retains E_OK. */
        }
        taskEXIT_CRITICAL();
        Os_BackendGuardService();
    }
    return result;
}

OS_MEMMAP StatusType Os_Implementation_DisableInterruptSource(ISRType ISRID)
{
    const Os_ErrorParametersType arguments = {.service_DisableInterruptSource = {ISRID}};
    const StatusType status = source_control(ISRID, 0u, FALSE);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_DisableInterruptSource, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_EnableInterruptSource(ISRType ISRID, boolean ClearPending)
{
    const Os_ErrorParametersType arguments = {
        .service_EnableInterruptSource = {ISRID, ClearPending}};
    const StatusType status = source_control(ISRID, 1u, ClearPending);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_EnableInterruptSource, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_ClearPendingInterrupt(ISRType ISRID)
{
    const Os_ErrorParametersType arguments = {.service_ClearPendingInterrupt = {ISRID}};
    const StatusType status = source_control(ISRID, 2u, FALSE);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_ClearPendingInterrupt, status, &arguments);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
