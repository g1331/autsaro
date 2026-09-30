#include "Os_Backend.h"

ISRType GetISRID(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetISRID, (uint32_t)0u, 0);
    }
    const ISRType result = Os_Implementation_GetISRID();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetISRID, Os_ArtiAccessValid() ? (uint32_t)result : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ControlIdle(CoreIdType CoreID, IdleModeType IdleMode) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ControlIdle, (uint32_t)IdleMode, 0);
    }
    const StatusType result = Os_Implementation_ControlIdle(CoreID, IdleMode);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ControlIdle, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

boolean isOsStarted(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_isOsStarted, (uint32_t)0u, 0);
    }
    const boolean result = Os_Implementation_isOsStarted();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_isOsStarted,
                       Os_ArtiAccessValid() ? (uint32_t)result : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetTaskID(TaskRefType TaskID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskID, (uint32_t)0u, 0);
    }
    const StatusType result = Os_Implementation_GetTaskID(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskID, (result == E_OK) ? (uint32_t)*TaskID : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetTaskState(TaskType TaskID, TaskStateRefType State) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskState, (uint32_t)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetTaskState(TaskID, State);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskState, (result == E_OK) ? (uint32_t)*State : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ActivateTask(TaskType TaskID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ActivateTask, (uint32_t)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ActivateTask(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ActivateTask, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType TerminateTask(void) {
    const int trace = Os_ArtiServiceEnter();
    const Os_NativeStack *stack = Os_StackCurrent();
    const uint32_t current =
        ((stack != NULL) && (stack->role == 'T')) ? (uint32_t)Os_ArtiRunningTask : UINT32_MAX;
    if (trace != 0) {
        Os_ArtiService(OSServiceId_TerminateTask, (uint32_t)current, 0);
    }
    const StatusType result = Os_Implementation_TerminateTask();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_TerminateTask, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ChainTask(TaskType TaskID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ChainTask, (uint32_t)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ChainTask(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ChainTask, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetResource(ResourceType ResID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetResource, (uint32_t)ResID, 0);
    }
    const StatusType result = Os_Implementation_GetResource(ResID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetResource, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ReleaseResource(ResourceType ResID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32_t)ResID, 0);
    }
    const StatusType result = Os_Implementation_ReleaseResource(ResID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType Schedule(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_Schedule, (uint32_t)0u, 0);
    }
    const StatusType result = Os_Implementation_Schedule();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_Schedule, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType WaitEvent(EventMaskType Mask) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_WaitEvent, (uint32_t)Mask, 0);
    }
    const StatusType result = Os_Implementation_WaitEvent(Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_WaitEvent, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ClearEvent(EventMaskType Mask) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearEvent, (uint32_t)Mask, 0);
    }
    const StatusType result = Os_Implementation_ClearEvent(Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearEvent, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetEvent(TaskType TaskID, EventMaskType Mask) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetEvent, (uint32_t)TaskID, 0);
    }
    const StatusType result = Os_Implementation_SetEvent(TaskID, Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetEvent, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetEvent(TaskType TaskID, EventMaskRefType Event) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetEvent, (uint32_t)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetEvent(TaskID, Event);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetEvent, (result == E_OK) ? (uint32_t)*Event : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

void ShutdownOS(StatusType Error) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ShutdownOS, (uint32_t)Error, 0);
    }
    Os_Implementation_ShutdownOS(Error);
    Os_ArtiServiceLeave();
}

void StartOS(AppModeType Mode) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartOS, (uint32_t)Mode, 0);
    }
    Os_Implementation_StartOS(Mode);
    Os_ArtiServiceLeave();
}

AppModeType GetActiveApplicationMode(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetActiveApplicationMode, (uint32_t)0u, 0);
    }
    const AppModeType result = Os_Implementation_GetActiveApplicationMode();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetActiveApplicationMode,
                       Os_ArtiAccessValid() ? (uint32_t)result : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType IncrementCounter(CounterType CounterID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32_t)CounterID, 0);
    }
    const StatusType result = Os_Implementation_IncrementCounter(CounterID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetCounterValue(CounterType CounterID, TickRefType Value) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetCounterValue, (uint32_t)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetCounterValue(CounterID, Value);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetCounterValue,
                       (result == E_OK) ? (uint32_t)*Value : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetElapsedValue(CounterType CounterID, TickRefType Value, TickRefType ElapsedValue) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetElapsedValue, (uint32_t)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetElapsedValue(CounterID, Value, ElapsedValue);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetElapsedValue,
                       (result == E_OK) ? (uint32_t)*ElapsedValue : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarmBase, (uint32_t)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarmBase(AlarmID, Info);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        if (result == E_OK) {
            Os_ArtiAlarmBaseReturn(Info);
        } else {
            Os_ArtiService(OSServiceId_GetAlarmBase, UINT32_MAX, 1);
        }
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetAlarm(AlarmType AlarmID, TickRefType Tick) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarm, (uint32_t)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarm(AlarmID, Tick);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarm, (result == E_OK) ? (uint32_t)*Tick : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetRelAlarm(AlarmType AlarmID, TickType Increment, TickType Cycle) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32_t)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetRelAlarm(AlarmID, Increment, Cycle);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32_t)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetAbsAlarm(AlarmID, Start, Cycle);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType CancelAlarm(AlarmType AlarmID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32_t)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_CancelAlarm(AlarmID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StartScheduleTableRel(ScheduleTableType ScheduleTableID, TickType Offset) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32_t)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableRel(ScheduleTableID, Offset);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StartScheduleTableAbs(ScheduleTableType ScheduleTableID, TickType Start) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32_t)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableAbs(ScheduleTableID, Start);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StopScheduleTable(ScheduleTableType ScheduleTableID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32_t)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StopScheduleTable(ScheduleTableID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                             ScheduleTableType ScheduleTableID_To) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32_t)ScheduleTableID_To, 0);
    }
    const StatusType result =
        Os_Implementation_NextScheduleTable(ScheduleTableID_From, ScheduleTableID_To);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetScheduleTableStatus(ScheduleTableType ScheduleTableID,
                                  ScheduleTableStatusRefType ScheduleStatus) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetScheduleTableStatus, (uint32_t)ScheduleTableID, 0);
    }
    const StatusType result =
        Os_Implementation_GetScheduleTableStatus(ScheduleTableID, ScheduleStatus);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetScheduleTableStatus,
                       (result == E_OK) ? (uint32_t)*ScheduleStatus : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType DisableInterruptSource(ISRType ISRID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32_t)ISRID, 0);
    }
    const StatusType result = Os_Implementation_DisableInterruptSource(ISRID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType EnableInterruptSource(ISRType ISRID, boolean ClearPending) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32_t)ISRID, 0);
    }
    const StatusType result = Os_Implementation_EnableInterruptSource(ISRID, ClearPending);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ClearPendingInterrupt(ISRType ISRID) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32_t)ISRID, 0);
    }
    const StatusType result = Os_Implementation_ClearPendingInterrupt(ISRID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32_t)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32_t)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

void EnableAllInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_EnableAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void DisableAllInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_DisableAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void ResumeAllInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_ResumeAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void SuspendAllInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_SuspendAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void ResumeOSInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_ResumeOSInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void SuspendOSInterrupts(void) {
    const int trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, (uint32_t)0u, 0);
    }
    Os_Implementation_SuspendOSInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}
