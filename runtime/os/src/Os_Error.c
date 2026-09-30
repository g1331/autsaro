#include "Os_Backend.h"

/* The fixed GCC/Win64 target uses the same native PE TLS ABI as Os_Stack.c.
 * Each actor keeps its own callback phase and typed snapshot; a failed call
 * from inside ErrorHook never recursively enters or overwrites that snapshot. */
static __thread Os_HookPhase phase;
static __thread OSServiceIdType service_id;
static __thread Os_ErrorParameters parameters;
static __thread unsigned hook_depth;
static __thread int error_active;
static const Os_ErrorParameters empty_parameters = {.service_TerminateTask = {0u}};
int Os_HookBlocksCategory2(void) { return hook_depth != 0u; }

void Os_ErrorContextSave(Os_ErrorContext *context) {
    context->phase = phase;
    context->service = service_id;
    context->parameters = parameters;
    context->arti = Os_ArtiCallerSave();
    phase = OS_HOOK_NONE;
    service_id = OSServiceId_Unknown;
    parameters = empty_parameters;
}
void Os_ErrorContextRestore(const Os_ErrorContext *context) {
    phase = context->phase;
    service_id = context->service;
    parameters = context->parameters;
    Os_ArtiCallerRestore(context->arti);
}

Os_HookPhase Os_HookContext(void) { return phase; }
int Os_HookQueryContext(void) {
    return (phase == OS_HOOK_ERROR) || (phase == OS_HOOK_PRE) || (phase == OS_HOOK_POST);
}
int Os_HookServiceAllowed(OSServiceIdType service) {
    if (phase == OS_HOOK_NONE) {
        return 1;
    }
    if (Os_HookQueryContext() != 0) {
        return (service == OSServiceId_GetTaskID) || (service == OSServiceId_GetTaskState) ||
               (service == OSServiceId_GetEvent) || (service == OSServiceId_GetAlarmBase) ||
               (service == OSServiceId_GetAlarm) ||
               (service == OSServiceId_GetActiveApplicationMode) ||
               ((phase == OS_HOOK_ERROR) &&
                ((service == OSServiceId_GetISRID) || (service == OSServiceId_ShutdownOS)));
    }
    if (phase == OS_HOOK_STARTUP) {
        return (service == OSServiceId_GetActiveApplicationMode) ||
               (service == OSServiceId_ShutdownOS);
    }
    if (phase == OS_HOOK_SHUTDOWN) {
        return service == OSServiceId_GetActiveApplicationMode;
    }
    return 0;
}
int Os_ErrorHookConfigured(void) {
    return (Os_Config != NULL) && (Os_Config->hooks != NULL) && (Os_Config->hooks->error != NULL);
}
StatusType Os_ServiceAccessStatus(OSServiceIdType service) {
    StatusType result = E_OK;
    const Os_NativeStack *stack = Os_StackCurrent();
    const unsigned interrupt = Os_BackendCurrentInterrupt();
    if (Os_InterruptDisabled() != 0) {
        result = E_OS_DISABLEDINT;
    } else if (service == OSServiceId_isOsStarted) {
        /* The draft initialized-C-environment query is available in all contexts. */
    } else if (Os_HookServiceAllowed(service) == 0) {
        result = E_OS_CALLEVEL;
    } else if ((phase == OS_HOOK_NONE) && (stack != NULL) && (stack->role == 'S') &&
               ((interrupt >= 32u) || (interrupt == OS_KERNEL_YIELD_INTERRUPT))) {
        result = E_OS_CALLEVEL;
    } else {
        /* The current logical caller owns this mask. ErrorHook uses a separate
         * cell and may query a masked parent until it masks itself. */
    }
    Os_ArtiAccessResult(result);
    return result;
}
const Os_ErrorParameters *Os_ErrorParametersCurrent(void) {
    return (phase == OS_HOOK_ERROR) ? &parameters : &empty_parameters;
}
OSServiceIdType Os_ErrorServiceId(void) {
    return (phase == OS_HOOK_ERROR) ? service_id : OSServiceId_Unknown;
}
StatusType Os_ErrorResult(OSServiceIdType service, StatusType status,
                          const Os_ErrorParameters *arguments) {
    if ((status != E_OK) && (error_active == 0) && (Os_ErrorHookConfigured() != 0) &&
        (Os_StackCurrent() != NULL) && (InterlockedCompareExchange(&Os_Closing, 0, 0) == 0)) {
        const Os_HookPhase previous = phase;
        const Os_NativeStack *stack = Os_StackCurrent();
        const int isr = (stack->role == 'S') && (Os_BackendCurrentInterrupt() < 32u);
        /* The native ISR dispatcher already owns the interrupt mutex. Do not
         * introduce an extra kernel critical level around an ISR ErrorHook.
         * The Hook gate excludes Cat2; higher priority Cat1 may interrupt it. */
        if (isr == 0) {
            taskENTER_CRITICAL();
        }
        Os_BackendGuardService();
        parameters = *arguments;
        service_id = service;
        phase = OS_HOOK_ERROR;
        ++hook_depth;
        error_active = 1;
        const Os_ArtiCaller arti = Os_ArtiCallerSave();
        Os_ArtiHook(OS_HOOK_ERROR, status, 0);
        Os_Config->hooks->error(status);
        Os_ArtiHook(OS_HOOK_ERROR, status, 1);
        Os_ArtiCallerRestore(arti);
        error_active = 0;
        --hook_depth;
        phase = previous;
        service_id = OSServiceId_Unknown;
        parameters = empty_parameters;
        if (isr == 0) {
            taskEXIT_CRITICAL();
        } else {
            Os_PortDispatchNested();
        }
        Os_BackendGuardService();
    }
    return status;
}
void Os_HookInvoke(void (*hook)(void), Os_HookPhase selected) {
    if (hook != NULL) {
        const Os_HookPhase previous = phase;
        taskENTER_CRITICAL();
        Os_BackendGuardService();
        phase = selected;
        ++hook_depth;
        const Os_ArtiCaller arti = Os_ArtiCallerSave();
        Os_ArtiHook(selected, E_OK, 0);
        hook();
        Os_ArtiHook(selected, E_OK, 1);
        Os_ArtiCallerRestore(arti);
        --hook_depth;
        phase = previous;
        taskEXIT_CRITICAL();
        Os_BackendGuardService();
    }
}
void Os_ShutdownHookInvoke(StatusType error) {
    const Os_HookPhase previous = phase;
    /* Closing runs on the established healthy controller. Never acquire the
     * ordinary port mutex: a suspended or damaged automotive actor may own it. */
    phase = OS_HOOK_SHUTDOWN;
    const Os_ArtiCaller arti = Os_ArtiCallerSave();
    Os_ArtiHook(OS_HOOK_SHUTDOWN, error, 0);
    ShutdownHook(error);
    Os_ArtiHook(OS_HOOK_SHUTDOWN, error, 1);
    Os_ArtiCallerRestore(arti);
    phase = previous;
}
