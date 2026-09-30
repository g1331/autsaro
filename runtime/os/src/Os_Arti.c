#include "Os_Arti.h"
#include <assert.h>

static __thread unsigned internal_depth;
static __thread unsigned service_depth;
static __thread StatusType access_status;
void Os_ArtiAccessResult(StatusType status) { access_status = status; }
int Os_ArtiAccessValid(void) { return access_status == E_OK; }
StatusType Os_ArtiAccessStatus(void) { return access_status; }
Os_ArtiCaller Os_ArtiCallerSave(void) {
    const Arti_AddressCapture address = Arti_SaveAddressCapture();
    const Os_ArtiCaller previous = {internal_depth, service_depth, access_status, address};
    internal_depth = 0u;
    service_depth = 0u;
    access_status = E_OK;
    return previous;
}
void Os_ArtiCallerRestore(Os_ArtiCaller caller) {
    internal_depth = caller.internal_depth;
    service_depth = caller.service_depth;
    access_status = caller.access_status;
    Arti_RestoreAddressCapture(caller.address);
}
void Os_ArtiInternalEnter(void) { ++internal_depth; }
void Os_ArtiInternalLeave(void) {
    assert(internal_depth != 0u);
    --internal_depth;
}
int Os_ArtiServiceEnter(void) {
    const int application = (internal_depth == 0u) && (service_depth == 0u);
    ++service_depth;
    access_status = E_OK;
    return application;
}
void Os_ArtiServiceLeave(void) {
    assert(service_depth != 0u);
    --service_depth;
}

void Os_ArtiTask(Os_ArtiTaskEvent event, TaskType task) {
    switch (event) {
    case OS_ARTI_TASK_ACTIVATE:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Activate, (uint32_t)task);
        break;
    case OS_ARTI_TASK_START:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Start, (uint32_t)task);
        break;
    case OS_ARTI_TASK_PREEMPT:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Preempt, (uint32_t)task);
        break;
    case OS_ARTI_TASK_WAIT:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Wait, (uint32_t)task);
        break;
    case OS_ARTI_TASK_RELEASE:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Release, (uint32_t)task);
        break;
    case OS_ARTI_TASK_TERMINATE:
        ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Terminate, (uint32_t)task);
        break;
    default:
        assert(0);
        break;
    }
}
void Os_ArtiIsr(ISRType isr, int returning) {
    if (returning != 0) {
        ARTI_TRACE(NOSUSP, AR_CP_OS_CAT2ISR, Os, 0u, OsCat2Isr_Stop, (uint32_t)isr);
    } else {
        ARTI_TRACE(NOSUSP, AR_CP_OS_CAT2ISR, Os, 0u, OsCat2Isr_Start, (uint32_t)isr);
    }
}
void Os_ArtiHook(Os_HookPhase hook, StatusType error, int returning) {
    switch (hook) {
    case OS_HOOK_ERROR:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ErrorHook_Return, 0u);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ErrorHook_Start, (uint32_t)error);
        }
        break;
    case OS_HOOK_PRE:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_PreTaskHook_Return, 0u);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_PreTaskHook_Start, 0u);
        }
        break;
    case OS_HOOK_POST:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_PostTaskHook_Return, 0u);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_PostTaskHook_Start, 0u);
        }
        break;
    case OS_HOOK_STARTUP:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_StartupHook_Return, 0u);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_StartupHook_Start, 0u);
        }
        break;
    case OS_HOOK_SHUTDOWN:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ShutdownHook_Return, 0u);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ShutdownHook_Start, (uint32_t)error);
        }
        break;
    default:
        /* AlarmCallback and NONE have no OS Hook events in this class. */
        break;
    }
}
void Os_ArtiService(OSServiceIdType service, uint32_t parameter, int returning) {
    switch (service) {
    case OSServiceId_GetISRID:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetISRID_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetISRID_Start,
                       parameter);
        }
        break;
    case OSServiceId_ControlIdle:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ControlIdle_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ControlIdle_Start,
                       parameter);
        }
        break;
    case OSServiceId_isOsStarted:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_isOsStarted_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_isOsStarted_Start,
                       parameter);
        }
        break;
    case OSServiceId_DisableInterruptSource:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_DisableInterruptSource_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_DisableInterruptSource_Start, parameter);
        }
        break;
    case OSServiceId_EnableInterruptSource:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_EnableInterruptSource_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_EnableInterruptSource_Start, parameter);
        }
        break;
    case OSServiceId_ClearPendingInterrupt:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ClearPendingInterrupt_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ClearPendingInterrupt_Start, parameter);
        }
        break;
    case OSServiceId_GetTaskID:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetTaskID_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetTaskID_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetTaskState:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetTaskState_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetTaskState_Start,
                       parameter);
        }
        break;
    case OSServiceId_ActivateTask:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ActivateTask_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ActivateTask_Start,
                       parameter);
        }
        break;
    case OSServiceId_TerminateTask:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_TerminateTask_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_TerminateTask_Start,
                       parameter);
        }
        break;
    case OSServiceId_ChainTask:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ChainTask_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ChainTask_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetResource:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetResource_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetResource_Start,
                       parameter);
        }
        break;
    case OSServiceId_ReleaseResource:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ReleaseResource_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ReleaseResource_Start,
                       parameter);
        }
        break;
    case OSServiceId_Schedule:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_Schedule_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_Schedule_Start,
                       parameter);
        }
        break;
    case OSServiceId_WaitEvent:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_WaitEvent_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_WaitEvent_Start,
                       parameter);
        }
        break;
    case OSServiceId_ClearEvent:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ClearEvent_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ClearEvent_Start,
                       parameter);
        }
        break;
    case OSServiceId_SetEvent:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetEvent_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetEvent_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetEvent:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetEvent_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetEvent_Start,
                       parameter);
        }
        break;
    case OSServiceId_IncrementCounter:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_IncrementCounter_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_IncrementCounter_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetCounterValue:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetCounterValue_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetCounterValue_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetElapsedValue:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetElapsedValue_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetElapsedValue_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetAlarmBase:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarmBase_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarmBase_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetAlarm:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarm_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarm_Start,
                       parameter);
        }
        break;
    case OSServiceId_SetRelAlarm:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetRelAlarm_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetRelAlarm_Start,
                       parameter);
        }
        break;
    case OSServiceId_SetAbsAlarm:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetAbsAlarm_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetAbsAlarm_Start,
                       parameter);
        }
        break;
    case OSServiceId_CancelAlarm:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_CancelAlarm_Return,
                       parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_CancelAlarm_Start,
                       parameter);
        }
        break;
    case OSServiceId_EnableAllInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_EnableAllInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_EnableAllInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_DisableAllInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_DisableAllInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_DisableAllInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_ResumeAllInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ResumeAllInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ResumeAllInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_SuspendAllInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_SuspendAllInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_SuspendAllInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_ResumeOSInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ResumeOSInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_ResumeOSInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_SuspendOSInterrupts:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_SuspendOSInterrupts_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_SuspendOSInterrupts_Start, parameter);
        }
        break;
    case OSServiceId_ShutdownOS:
        if (returning != 0) {
            /* This service has no standard Return event. */
            assert(0);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_ShutdownOS_Start,
                       parameter);
        }
        break;
    case OSServiceId_StartOS:
        if (returning != 0) {
            /* This service has no standard Return event. */
            assert(0);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_StartOS_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetActiveApplicationMode:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_GetActiveApplicationMode_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_GetActiveApplicationMode_Start, parameter);
        }
        break;
    case OSServiceId_StartScheduleTableRel:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_StartScheduleTableRel_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_StartScheduleTableRel_Start, parameter);
        }
        break;
    case OSServiceId_StartScheduleTableAbs:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_StartScheduleTableAbs_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_StartScheduleTableAbs_Start, parameter);
        }
        break;
    case OSServiceId_StopScheduleTable:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_StopScheduleTable_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_StopScheduleTable_Start,
                       parameter);
        }
        break;
    case OSServiceId_NextScheduleTable:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_NextScheduleTable_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_NextScheduleTable_Start,
                       parameter);
        }
        break;
    case OSServiceId_GetScheduleTableStatus:
        if (returning != 0) {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_GetScheduleTableStatus_Return, parameter);
        } else {
            ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u,
                       OsServiceCall_GetScheduleTableStatus_Start, parameter);
        }
        break;
    default:
        assert(0);
        break;
    }
}

void Os_ArtiAlarmBaseReturn(AlarmBaseRefType address) {
    const uintptr_t native_address = (uintptr_t)address;
    const uint32_t token = (uint32_t)native_address;
    Arti_CaptureAddress(native_address);
    ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarmBase_Return, token);
}
