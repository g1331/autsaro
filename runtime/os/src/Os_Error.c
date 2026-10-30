/** @file Hook invocation and ErrorHook context kept separately for each actor. */
#include "Os_Backend.h"

/* The fixed GCC/Win64 target uses the same native PE TLS ABI as Os_Stack.c.
 * Each actor keeps its own callback phase and typed snapshot; a failed call
 * from inside ErrorHook never recursively enters or overwrites that snapshot. */
#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL Os_HookPhaseType
    phase; /**< Actor TLS hook phase; context save/restore, no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Error.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Error.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_TLS_8
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL OSServiceIdType
    service_id; /**< Actor TLS failed-service identity; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_64
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL Os_ErrorParametersType
    parameters; /**< Actor TLS ErrorHook snapshot; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL uint32
    hook_depth; /**< Actor TLS hook nesting count; no Init reset. */

/* MR12 RULE 8.9 VIOLATION: Explicit TLS mapping requires file-static scope under SWS_MemMap_00023.
 */
OS_MEMMAP static OS_THREAD_LOCAL sint32
    error_active; /**< Actor TLS ErrorHook recursion guard; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_64
#include "Os_MemMap.h"
OS_MEMMAP static const Os_ErrorParametersType empty_parameters = {
    .service_TerminateTask = {0u}}; /**< C-initialized immutable empty snapshot; no Init writes. */
#define OS_STOP_SEC_CONST_64
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_HookBlocksCategory2(void) { return (hook_depth != 0u) ? 1 : 0; }

/* MR12 RULE 18.6 VIOLATION: The bounded synchronous ISR snapshot borrows live caller outputs;
 * restore and retired-slot clearing bound access. See native-facilities/error-context.md. */
OS_MEMMAP void Os_ErrorContextSave(Os_ErrorContextType *context)
{
    context->phase = phase;
    context->service = service_id;
    context->parameters = parameters;
    context->arti = Os_ArtiCallerSave();
    phase = OS_HOOK_NONE;
    service_id = OSServiceId_Unknown;
    parameters = empty_parameters;
}

/* MR12 RULE 18.6 VIOLATION: Restore the same live parent's typed snapshot before its Hook resumes;
 * no pointee lifetime extension. See native-facilities/error-context.md. */
OS_MEMMAP void Os_ErrorContextRestore(const Os_ErrorContextType *context)
{
    phase = context->phase;
    service_id = context->service;
    parameters = context->parameters;
    Os_ArtiCallerRestore(context->arti);
}

OS_MEMMAP Os_HookPhaseType Os_HookContext(void) { return phase; }

OS_MEMMAP sint32 Os_HookQueryContext(void)
{
    return ((phase == OS_HOOK_ERROR) || (phase == OS_HOOK_PRE) || (phase == OS_HOOK_POST)) ? 1 : 0;
}

OS_MEMMAP static sint32 Os_HookServiceAllowed(OSServiceIdType service)
{
    if (phase == OS_HOOK_NONE) {
        return 1;
    }
    if (Os_HookQueryContext() != 0) {
        return ((service == OSServiceId_GetTaskID) || (service == OSServiceId_GetTaskState) ||
                (service == OSServiceId_GetEvent) || (service == OSServiceId_GetAlarmBase) ||
                (service == OSServiceId_GetAlarm) ||
                (service == OSServiceId_GetActiveApplicationMode) ||
                ((phase == OS_HOOK_ERROR) &&
                 ((service == OSServiceId_GetISRID) || (service == OSServiceId_ShutdownOS))))
                   ? 1
                   : 0;
    }
    if (phase == OS_HOOK_STARTUP) {
        return ((service == OSServiceId_GetActiveApplicationMode) ||
                (service == OSServiceId_ShutdownOS))
                   ? 1
                   : 0;
    }
    if (phase == OS_HOOK_SHUTDOWN) {
        return (service == OSServiceId_GetActiveApplicationMode) ? 1 : 0;
    }
    return 0;
}

OS_MEMMAP sint32 Os_ErrorHookConfigured(void)
{
    return ((Os_Config != NULL) && (Os_Config->hooks != NULL) && (Os_Config->hooks->error != NULL))
               ? 1
               : 0;
}

OS_MEMMAP StatusType Os_ServiceAccessStatus(OSServiceIdType service)
{
    StatusType result = E_OK;
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 interrupt = Os_BackendCurrentInterrupt();
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

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
OS_MEMMAP const Os_ErrorParametersType *Os_ErrorParametersCurrent(void)
{
    return (phase == OS_HOOK_ERROR) ? &parameters : &empty_parameters;
}

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
OS_MEMMAP OSServiceIdType Os_ErrorServiceId(void)
{
    return (phase == OS_HOOK_ERROR) ? service_id : OSServiceId_Unknown;
}

/* MR12 RULE 18.6 VIOLATION: Enabled ErrorHook borrows outputs only within the failing service call;
 * recursion guard and exit clearing preserve that bound. See native-facilities/error-context.md. */
OS_MEMMAP StatusType Os_ErrorResult(OSServiceIdType service,
                                    StatusType status,
                                    const Os_ErrorParametersType *arguments)
{
    boolean invoke = FALSE;
    if ((status != E_OK) && (error_active == 0) && (Os_ErrorHookConfigured() != 0) &&
        (Os_StackCurrent() != NULL)) {
        invoke = InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
    }
    if (invoke) {
        const Os_HookPhaseType previous = phase;
        const Os_NativeStackType *stack = Os_StackCurrent();
        const boolean isr = (stack->role == 'S') && (Os_BackendCurrentInterrupt() < 32u);
        /* The native ISR dispatcher already owns the interrupt mutex. Do not
         * introduce an extra kernel critical level around an ISR ErrorHook.
         * The Hook gate excludes Cat2; higher priority Cat1 may interrupt it. */
        if (!isr) {
            taskENTER_CRITICAL();
        }
        Os_BackendGuardService();
        const Os_ErrorParametersType *const sources[2] = {&empty_parameters, arguments};
        parameters = *sources[OS_USE_PARAMETER_ACCESS];
        service_id = service;
        phase = OS_HOOK_ERROR;
        ++hook_depth;
        error_active = 1;
        const Os_ArtiCallerType arti = Os_ArtiCallerSave();
        Os_ArtiHook(OS_HOOK_ERROR, status, 0);
        Os_Config->hooks->error(status);
        Os_ArtiHook(OS_HOOK_ERROR, status, 1);
        Os_ArtiCallerRestore(arti);
        error_active = 0;
        --hook_depth;
        phase = previous;
        service_id = OSServiceId_Unknown;
        parameters = empty_parameters;
        if (!isr) {
            taskEXIT_CRITICAL();
        } else {
            Os_PortDispatchNested();
        }
        Os_BackendGuardService();
    }
    return status;
}

OS_MEMMAP void Os_HookInvoke(void (*hook)(void), Os_HookPhaseType selected)
{
    if (hook != NULL) {
        const Os_HookPhaseType previous = phase;
        taskENTER_CRITICAL();
        Os_BackendGuardService();
        phase = selected;
        ++hook_depth;
        const Os_ArtiCallerType arti = Os_ArtiCallerSave();
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

OS_MEMMAP void Os_ShutdownHookInvoke(StatusType error)
{
    const Os_HookPhaseType previous = phase;
    /* Closing runs on the established healthy controller. Never acquire the
     * ordinary port mutex: a suspended or damaged automotive actor may own it. */
    phase = OS_HOOK_SHUTDOWN;
    const Os_ArtiCallerType arti = Os_ArtiCallerSave();
    Os_ArtiHook(OS_HOOK_SHUTDOWN, error, 0);
    ShutdownHook(error);
    Os_ArtiHook(OS_HOOK_SHUTDOWN, error, 1);
    Os_ArtiCallerRestore(arti);
    phase = previous;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
