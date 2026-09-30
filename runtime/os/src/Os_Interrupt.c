#include "Os_Backend.h"
#include <limits.h>

typedef struct {
    uint32_t all_depth;
    uint32_t os_depth;
    uint8_t disabled;
} InterruptState;
/* One cell per physical Task/boot actor, and separate logical ISR cells on
 * the native ISR thread. This preserves ownership across interrupted actors. */
static __thread InterruptState states[33u * ((unsigned)OS_HOOK_ALARM + 1u)];
static volatile LONG all_owners;
static volatile LONG os_owners;

static InterruptState *owner_state(void) {
    const Os_NativeStack *stack = Os_StackCurrent();
    const unsigned interrupt = Os_BackendCurrentInterrupt();
    const Os_HookPhase phase = Os_HookContext();
    unsigned index = (stack != NULL && stack->role == 'S' && interrupt < 32u) ? interrupt : 32u;
    if (phase >= OS_HOOK_ERROR && phase <= OS_HOOK_ALARM) {
        /* Each Hook has its own mask ownership, including its logical ISR.
         * Queries in ErrorHook do not inherit the failed caller's mask. */
        index += 33u * (unsigned)phase;
    }
    return &states[index];
}
int Os_InterruptDisabled(void) {
    const InterruptState *state = owner_state();
    return state->disabled != 0u || state->all_depth != 0u || state->os_depth != 0u;
}
int Os_InterruptAllows(unsigned interrupt) {
    int allowed = 0;
    if ((interrupt < 32u) && (InterlockedCompareExchange(&all_owners, 0, 0) == 0)) {
        allowed = (InterlockedCompareExchange(&os_owners, 0, 0) == 0) ||
                  ((Os_Config != NULL) &&
                   ((Os_Config->category1_isrs & (UINT32_C(1) << interrupt)) != 0u));
    }
    return allowed;
}
static int permitted(unsigned operation) {
    const Os_HookPhase phase = Os_HookContext();
    const Os_NativeStack *stack = Os_StackCurrent();
    const unsigned group = operation / 2u;
    int allowed = 0;
    if (phase >= OS_HOOK_ERROR && phase <= OS_HOOK_ALARM) {
        /* R24-11 Table 7.1 permits all six primitives from every supported
         * global Hook and Alarm callback, extending the older OSEK table. */
        allowed = 1;
    } else if (phase == OS_HOOK_NONE && group != 2u &&
               (Os_BackendStarted() == 0 || InterlockedCompareExchange(&Os_Closing, 0, 0) != 0)) {
        allowed = 1;
    } else if (phase == OS_HOOK_NONE && stack != NULL) {
        allowed = stack->role == 'T' || (stack->role == 'S' && Os_BackendCurrentInterrupt() < 32u);
    } else {
        /* A registered nonautomotive actor is not a Task or active ISR. */
    }
    return allowed;
}
static void change(unsigned operation, OSServiceIdType service) {
    InterruptState *state = owner_state();
    StatusType status = E_OK;
    const Os_ErrorParameters arguments = {.service_TerminateTask = {0u}};
    const int running =
        Os_BackendStarted() != 0 && InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
    Os_StackCheck();
    Os_BackendGuardService();
    if (permitted(operation) == 0) {
        status = E_OS_CALLEVEL;
    } else {
        if (running != 0) {
            taskENTER_CRITICAL();
        }
        switch (operation) {
        case 0u:
            if (state->disabled != 0u) {
                state->disabled = 0u;
                InterlockedDecrement(&all_owners);
            }
            break;
        case 1u:
            if (state->disabled == 0u) {
                state->disabled = 1u;
                InterlockedIncrement(&all_owners);
            }
            break;
        case 2u:
            if (state->all_depth != 0u) {
                --state->all_depth;
                if (state->all_depth == 0u) {
                    InterlockedDecrement(&all_owners);
                }
            }
            break;
        case 3u:
            if (state->all_depth == UINT32_MAX) {
                status = E_OS_LIMIT;
            } else {
                if (state->all_depth == 0u) {
                    InterlockedIncrement(&all_owners);
                }
                ++state->all_depth;
            }
            break;
        case 4u:
            if (state->os_depth != 0u) {
                --state->os_depth;
                if (state->os_depth == 0u) {
                    InterlockedDecrement(&os_owners);
                }
            }
            break;
        case 5u:
            if (state->os_depth == UINT32_MAX) {
                status = E_OS_LIMIT;
            } else {
                if (state->os_depth == 0u) {
                    InterlockedIncrement(&os_owners);
                }
                ++state->os_depth;
            }
            break;
        default:
            status = E_OS_ID;
            break;
        }
        if (running != 0) {
            taskEXIT_CRITICAL();
        }
    }
    if (Os_HookContext() != OS_HOOK_ERROR) {
        (void)Os_ErrorResult(service, status, &arguments);
    }
}
void Os_Implementation_EnableAllInterrupts(void) { change(0u, OSServiceId_EnableAllInterrupts); }
void Os_Implementation_DisableAllInterrupts(void) { change(1u, OSServiceId_DisableAllInterrupts); }
void Os_Implementation_ResumeAllInterrupts(void) { change(2u, OSServiceId_ResumeAllInterrupts); }
void Os_Implementation_SuspendAllInterrupts(void) { change(3u, OSServiceId_SuspendAllInterrupts); }
void Os_Implementation_ResumeOSInterrupts(void) { change(4u, OSServiceId_ResumeOSInterrupts); }
void Os_Implementation_SuspendOSInterrupts(void) { change(5u, OSServiceId_SuspendOSInterrupts); }
void Os_InterruptRestoreOwner(void) {
    InterruptState *state = owner_state();
    /* Internal exit cleanup is called under the real OS completion critical
     * section, so no ISR can observe partial restoration. */
    if (state->disabled != 0u) {
        InterlockedDecrement(&all_owners);
    }
    if (state->all_depth != 0u) {
        InterlockedDecrement(&all_owners);
    }
    if (state->os_depth != 0u) {
        InterlockedDecrement(&os_owners);
    }
    state->disabled = 0u;
    state->all_depth = 0u;
    state->os_depth = 0u;
}
static StatusType source_control(ISRType interrupt, unsigned operation, boolean clear) {
    StatusType result = E_OK;
    const Os_NativeStack *stack = Os_StackCurrent();
    const unsigned current = Os_BackendCurrentInterrupt();
    Os_StackCheck();
    Os_BackendGuardService();
    /* Source operations are interrupt services, so application All/OS masks
     * do not reject them. Hooks and Category 1 callers cannot change sources. */
    if ((Os_HookContext() != OS_HOOK_NONE) || (Os_BackendServiceContext() == 0) ||
        ((stack != NULL) && (stack->role == 'S') &&
         ((current >= 32u) || (current == OS_KERNEL_YIELD_INTERRUPT)))) {
        result = E_OS_CALLEVEL;
    } else if ((interrupt >= 32u) || (interrupt == OS_KERNEL_YIELD_INTERRUPT) ||
               (interrupt == OS_CONTROLLED_TICK_INTERRUPT) ||
               ((Os_Config->category1_isrs & (UINT32_C(1) << interrupt)) != 0u)) {
        result = E_OS_ID;
    } else {
        taskENTER_CRITICAL();
        Os_BackendGuardService();
        const int source_result = Os_PortInterruptSourceControl(interrupt, operation, clear);
        if (source_result == 0) {
            result = E_OS_ID;
        } else if (source_result == 2) {
            result = E_OS_NOFUNC;
        }
        taskEXIT_CRITICAL();
        Os_BackendGuardService();
    }
    return result;
}
StatusType Os_Implementation_DisableInterruptSource(ISRType ISRID) {
    const Os_ErrorParameters arguments = {.service_DisableInterruptSource = {ISRID}};
    const StatusType status = source_control(ISRID, 0u, FALSE);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_DisableInterruptSource, status, &arguments);
}
StatusType Os_Implementation_EnableInterruptSource(ISRType ISRID, boolean ClearPending) {
    const Os_ErrorParameters arguments = {.service_EnableInterruptSource = {ISRID, ClearPending}};
    const StatusType status = source_control(ISRID, 1u, ClearPending);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_EnableInterruptSource, status, &arguments);
}
StatusType Os_Implementation_ClearPendingInterrupt(ISRType ISRID) {
    const Os_ErrorParameters arguments = {.service_ClearPendingInterrupt = {ISRID}};
    const StatusType status = source_control(ISRID, 2u, FALSE);
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_ClearPendingInterrupt, status, &arguments);
}
