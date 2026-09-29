#include "Os_Backend.h"

/* The fixed GCC/Win64 target uses the same native PE TLS ABI as Os_Stack.c.
 * Each actor keeps its own callback phase and typed snapshot; a failed call
 * from inside ErrorHook never recursively enters or overwrites that snapshot. */
static __thread Os_HookPhase phase;
static __thread OSServiceIdType service_id;
static __thread Os_ErrorParameters parameters;
static const Os_ErrorParameters empty_parameters = {.service_TerminateTask = {0u}};

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
               (service == OSServiceId_GetAlarm);
    }
    return 0;
}
int Os_ErrorHookConfigured(void) {
    return (Os_Config != NULL) && (Os_Config->hooks != NULL) && (Os_Config->hooks->error != NULL);
}
const Os_ErrorParameters *Os_ErrorParametersCurrent(void) {
    return (phase == OS_HOOK_ERROR) ? &parameters : &empty_parameters;
}
OSServiceIdType Os_ErrorServiceId(void) {
    return (phase == OS_HOOK_ERROR) ? service_id : OSServiceId_Unknown;
}
StatusType Os_ErrorResult(OSServiceIdType service, StatusType status,
                          const Os_ErrorParameters *arguments) {
    if ((status != E_OK) && (phase != OS_HOOK_ERROR) && (Os_ErrorHookConfigured() != 0) &&
        (Os_StackCurrent() != NULL) && (InterlockedCompareExchange(&Os_Closing, 0, 0) == 0)) {
        const Os_HookPhase previous = phase;
        taskENTER_CRITICAL();
        Os_BackendGuardService();
        parameters = *arguments;
        service_id = service;
        phase = OS_HOOK_ERROR;
        Os_Config->hooks->error(status);
        phase = previous;
        service_id = OSServiceId_Unknown;
        parameters = empty_parameters;
        taskEXIT_CRITICAL();
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
        hook();
        phase = previous;
        taskEXIT_CRITICAL();
        Os_BackendGuardService();
    }
}
