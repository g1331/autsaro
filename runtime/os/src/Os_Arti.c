/** @file ARTI trace context and event emission for OS tasks, ISRs, hooks and services. */
#include "Os_Arti.h"
#include <assert.h>

#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL uint32
    internal_depth; /**< Actor TLS origin depth; CallerSave resets, not Init. */

OS_MEMMAP static OS_THREAD_LOCAL uint32
    service_depth; /**< Actor TLS service depth; CallerSave resets, not Init. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_8
#include "Os_MemMap.h"
OS_MEMMAP static OS_THREAD_LOCAL StatusType
    access_status; /**< Actor TLS access result; CallerSave resets, not Init. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_8
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_ArtiAccessResult(StatusType status) { access_status = status; }

OS_MEMMAP sint32 Os_ArtiAccessValid(void) { return access_status == E_OK; }

OS_MEMMAP StatusType Os_ArtiAccessStatus(void) { return access_status; }

OS_MEMMAP Os_ArtiCallerType Os_ArtiCallerSave(void)
{
    const Arti_AddressCapture address = Arti_SaveAddressCapture();
    const Os_ArtiCallerType previous = {internal_depth, service_depth, access_status, address};
    internal_depth = 0u;
    service_depth = 0u;
    access_status = E_OK;
    return previous;
}

OS_MEMMAP void Os_ArtiCallerRestore(Os_ArtiCallerType caller)
{
    internal_depth = caller.internal_depth;
    service_depth = caller.service_depth;
    access_status = caller.access_status;
    Arti_RestoreAddressCapture(caller.address);
}

OS_MEMMAP void Os_ArtiInternalEnter(void) { ++internal_depth; }

OS_MEMMAP void Os_ArtiInternalLeave(void)
{
    assert(internal_depth != 0u);
    --internal_depth;
}

OS_MEMMAP sint32 Os_ArtiServiceEnter(void)
{
    const sint32 application = (internal_depth == 0u) && (service_depth == 0u);
    ++service_depth;
    access_status = E_OK;
    return application;
}

OS_MEMMAP void Os_ArtiServiceLeave(void)
{
    assert(service_depth != 0u);
    --service_depth;
}

OS_MEMMAP void Os_ArtiTask(Os_ArtiTaskEventType event, TaskType task)
{
    switch (event) {
        case OS_ARTI_TASK_ACTIVATE:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Activate, (uint32)task);
            break;
        case OS_ARTI_TASK_START:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Start, (uint32)task);
            break;
        case OS_ARTI_TASK_PREEMPT:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Preempt, (uint32)task);
            break;
        case OS_ARTI_TASK_WAIT:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Wait, (uint32)task);
            break;
        case OS_ARTI_TASK_RELEASE:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Release, (uint32)task);
            break;
        case OS_ARTI_TASK_TERMINATE:
            ARTI_TRACE(NOSUSP, AR_CP_OS_TASK, Os, 0u, OsTask_Terminate, (uint32)task);
            break;
        default:
            assert(0);
            break;
    }
}

OS_MEMMAP void Os_ArtiIsr(ISRType isr, sint32 returning)
{
    if (returning != 0) {
        ARTI_TRACE(NOSUSP, AR_CP_OS_CAT2ISR, Os, 0u, OsCat2Isr_Stop, (uint32)isr);
    } else {
        ARTI_TRACE(NOSUSP, AR_CP_OS_CAT2ISR, Os, 0u, OsCat2Isr_Start, (uint32)isr);
    }
}

OS_MEMMAP void Os_ArtiHook(Os_HookPhaseType hook, StatusType error, sint32 returning)
{
    switch (hook) {
        case OS_HOOK_ERROR:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ErrorHook_Return, 0u);
            } else {
                ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ErrorHook_Start, (uint32)error);
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
                ARTI_TRACE(SPRVSR, AR_CP_OS_HOOK, Os, 0u, OsHook_ShutdownHook_Start, (uint32)error);
            }
            break;
        default:
            /* AlarmCallback and NONE have no OS Hook events in this class. */
            break;
    }
}

OS_MEMMAP void Os_ArtiService(OSServiceIdType service, uint32 parameter, sint32 returning)
{
    switch (service) {
        case OSServiceId_GetISRID:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetISRID_Return,
                           parameter);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetISRID_Start, parameter);
            }
            break;
        case OSServiceId_ControlIdle:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ControlIdle_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ControlIdle_Start,
                           parameter);
            }
            break;
        case OSServiceId_isOsStarted:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_isOsStarted_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_isOsStarted_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetCoreID:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetCoreID_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetCoreID_Start,
                           parameter);
            }
            break;
        case OSServiceId_DisableInterruptSource:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_DisableInterruptSource_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_DisableInterruptSource_Start,
                           parameter);
            }
            break;
        case OSServiceId_EnableInterruptSource:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_EnableInterruptSource_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_EnableInterruptSource_Start,
                           parameter);
            }
            break;
        case OSServiceId_ClearPendingInterrupt:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ClearPendingInterrupt_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ClearPendingInterrupt_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetTaskID:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetTaskID_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetTaskID_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetTaskState:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetTaskState_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetTaskState_Start,
                           parameter);
            }
            break;
        case OSServiceId_ActivateTask:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ActivateTask_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ActivateTask_Start,
                           parameter);
            }
            break;
        case OSServiceId_TerminateTask:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_TerminateTask_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_TerminateTask_Start,
                           parameter);
            }
            break;
        case OSServiceId_ChainTask:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ChainTask_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ChainTask_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetResource:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetResource_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetResource_Start,
                           parameter);
            }
            break;
        case OSServiceId_ReleaseResource:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ReleaseResource_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ReleaseResource_Start,
                           parameter);
            }
            break;
        case OSServiceId_Schedule:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_Schedule_Return,
                           parameter);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_Schedule_Start, parameter);
            }
            break;
        case OSServiceId_WaitEvent:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_WaitEvent_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_WaitEvent_Start,
                           parameter);
            }
            break;
        case OSServiceId_ClearEvent:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ClearEvent_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ClearEvent_Start,
                           parameter);
            }
            break;
        case OSServiceId_SetEvent:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SetEvent_Return,
                           parameter);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_SetEvent_Start, parameter);
            }
            break;
        case OSServiceId_GetEvent:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetEvent_Return,
                           parameter);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetEvent_Start, parameter);
            }
            break;
        case OSServiceId_IncrementCounter:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_IncrementCounter_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_IncrementCounter_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetCounterValue:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetCounterValue_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetCounterValue_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetElapsedValue:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetElapsedValue_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetElapsedValue_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetAlarmBase:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetAlarmBase_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetAlarmBase_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetAlarm:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetAlarm_Return,
                           parameter);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarm_Start, parameter);
            }
            break;
        case OSServiceId_SetRelAlarm:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SetRelAlarm_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SetRelAlarm_Start,
                           parameter);
            }
            break;
        case OSServiceId_SetAbsAlarm:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SetAbsAlarm_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SetAbsAlarm_Start,
                           parameter);
            }
            break;
        case OSServiceId_CancelAlarm:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_CancelAlarm_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_CancelAlarm_Start,
                           parameter);
            }
            break;
        case OSServiceId_EnableAllInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_EnableAllInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_EnableAllInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_DisableAllInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_DisableAllInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_DisableAllInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_ResumeAllInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ResumeAllInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ResumeAllInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_SuspendAllInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SuspendAllInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SuspendAllInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_ResumeOSInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ResumeOSInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ResumeOSInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_SuspendOSInterrupts:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SuspendOSInterrupts_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_SuspendOSInterrupts_Start,
                           parameter);
            }
            break;
        case OSServiceId_ShutdownOS:
            if (returning != 0) {
                /* This service has no standard Return event. */
                assert(0);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_ShutdownOS_Start,
                           parameter);
            }
            break;
        case OSServiceId_StartOS:
            if (returning != 0) {
                /* This service has no standard Return event. */
                assert(0);
            } else {
                ARTI_TRACE(
                    SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_StartOS_Start, parameter);
            }
            break;
        case OSServiceId_GetActiveApplicationMode:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetActiveApplicationMode_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetActiveApplicationMode_Start,
                           parameter);
            }
            break;
        case OSServiceId_StartScheduleTableRel:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StartScheduleTableRel_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StartScheduleTableRel_Start,
                           parameter);
            }
            break;
        case OSServiceId_StartScheduleTableAbs:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StartScheduleTableAbs_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StartScheduleTableAbs_Start,
                           parameter);
            }
            break;
        case OSServiceId_StopScheduleTable:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StopScheduleTable_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_StopScheduleTable_Start,
                           parameter);
            }
            break;
        case OSServiceId_NextScheduleTable:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_NextScheduleTable_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_NextScheduleTable_Start,
                           parameter);
            }
            break;
        case OSServiceId_GetScheduleTableStatus:
            if (returning != 0) {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetScheduleTableStatus_Return,
                           parameter);
            } else {
                ARTI_TRACE(SPRVSR,
                           AR_CP_OS_SERVICECALLS,
                           Os,
                           0u,
                           OsServiceCall_GetScheduleTableStatus_Start,
                           parameter);
            }
            break;
        default:
            assert(0);
            break;
    }
}

OS_MEMMAP void Os_ArtiAlarmBaseReturn(AlarmBaseRefType address)
{
    const uint64 native_address = (uint64)address;
    const uint32 token = (uint32)native_address;
    Arti_CaptureAddress(native_address);
    ARTI_TRACE(SPRVSR, AR_CP_OS_SERVICECALLS, Os, 0u, OsServiceCall_GetAlarmBase_Return, token);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
