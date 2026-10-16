/** @file Traced public OS service wrappers around the implementation entry points. */
#include "Os_Backend.h"

ISRType GetISRID(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetISRID, (uint32)0u, 0);
    }
    const ISRType result = Os_Implementation_GetISRID();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetISRID, Os_ArtiAccessValid() ? (uint32)result : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ControlIdle(CoreIdType CoreID, IdleModeType IdleMode)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ControlIdle, (uint32)IdleMode, 0);
    }
    const StatusType result = Os_Implementation_ControlIdle(CoreID, IdleMode);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ControlIdle, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

boolean isOsStarted(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_isOsStarted, (uint32)0u, 0);
    }
    const boolean result = Os_Implementation_isOsStarted();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        const uint32 started = result ? 1u : 0u;
        Os_ArtiService(OSServiceId_isOsStarted, Os_ArtiAccessValid() ? started : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

CoreIdType GetCoreID(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetCoreID, (uint32)0u, 0);
    }
    const CoreIdType result = Os_Implementation_GetCoreID();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)E_OK);
        Os_ArtiService(OSServiceId_GetCoreID, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetTaskID(TaskRefType TaskID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskID, (uint32)0u, 0);
    }
    const StatusType result = Os_Implementation_GetTaskID(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskID, (result == E_OK) ? (uint32)*TaskID : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetTaskState(TaskType TaskID, TaskStateRefType State)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskState, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetTaskState(TaskID, State);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetTaskState, (result == E_OK) ? (uint32)*State : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ActivateTask(TaskType TaskID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ActivateTask, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ActivateTask(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ActivateTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType TerminateTask(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    const Os_NativeStack *stack = Os_StackCurrent();
    const uint32 current =
        ((stack != NULL) && (stack->role == 'T')) ? (uint32)Os_ArtiRunningTask : UINT32_MAX;
    if (trace != 0) {
        Os_ArtiService(OSServiceId_TerminateTask, (uint32)current, 0);
    }
    const StatusType result = Os_Implementation_TerminateTask();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_TerminateTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ChainTask(TaskType TaskID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ChainTask, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_ChainTask(TaskID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ChainTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetResource(ResourceType ResID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetResource, (uint32)ResID, 0);
    }
    const StatusType result = Os_Implementation_GetResource(ResID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetResource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ReleaseResource(ResourceType ResID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32)ResID, 0);
    }
    const StatusType result = Os_Implementation_ReleaseResource(ResID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ReleaseResource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType Schedule(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_Schedule, (uint32)0u, 0);
    }
    const StatusType result = Os_Implementation_Schedule();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_Schedule, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType WaitEvent(EventMaskType Mask)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_WaitEvent, (uint32)Mask, 0);
    }
    const StatusType result = Os_Implementation_WaitEvent(Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_WaitEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ClearEvent(EventMaskType Mask)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearEvent, (uint32)Mask, 0);
    }
    const StatusType result = Os_Implementation_ClearEvent(Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetEvent(TaskType TaskID, EventMaskType Mask)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetEvent, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_SetEvent(TaskID, Mask);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetEvent, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetEvent(TaskType TaskID, EventMaskRefType Event)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetEvent, (uint32)TaskID, 0);
    }
    const StatusType result = Os_Implementation_GetEvent(TaskID, Event);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetEvent, (result == E_OK) ? (uint32)*Event : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

void ShutdownOS(StatusType Error)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ShutdownOS, (uint32)Error, 0);
    }
    Os_Implementation_ShutdownOS(Error);
    Os_ArtiServiceLeave();
}

void StartOS(AppModeType Mode)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartOS, (uint32)Mode, 0);
    }
    Os_Implementation_StartOS(Mode);
    Os_ArtiServiceLeave();
}

AppModeType GetActiveApplicationMode(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetActiveApplicationMode, (uint32)0u, 0);
    }
    const AppModeType result = Os_Implementation_GetActiveApplicationMode();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetActiveApplicationMode,
                       Os_ArtiAccessValid() ? (uint32)result : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType IncrementCounter(CounterType CounterID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_IncrementCounter(CounterID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_IncrementCounter, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetCounterValue(CounterType CounterID, TickRefType Value)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetCounterValue, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetCounterValue(CounterID, Value);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(
            OSServiceId_GetCounterValue, (result == E_OK) ? (uint32)*Value : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetElapsedValue(CounterType CounterID, TickRefType Value, TickRefType ElapsedValue)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetElapsedValue, (uint32)CounterID, 0);
    }
    const StatusType result = Os_Implementation_GetElapsedValue(CounterID, Value, ElapsedValue);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(
            OSServiceId_GetElapsedValue, (result == E_OK) ? (uint32)*ElapsedValue : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarmBase, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarmBase(AlarmID, Info);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
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

StatusType GetAlarm(AlarmType AlarmID, TickRefType Tick)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_GetAlarm(AlarmID, Tick);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetAlarm, (result == E_OK) ? (uint32)*Tick : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetRelAlarm(AlarmType AlarmID, TickType Increment, TickType Cycle)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetRelAlarm(AlarmID, Increment, Cycle);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetRelAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_SetAbsAlarm(AlarmID, Start, Cycle);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SetAbsAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType CancelAlarm(AlarmType AlarmID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32)AlarmID, 0);
    }
    const StatusType result = Os_Implementation_CancelAlarm(AlarmID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_CancelAlarm, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StartScheduleTableRel(ScheduleTableType ScheduleTableID, TickType Offset)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableRel(ScheduleTableID, Offset);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableRel, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StartScheduleTableAbs(ScheduleTableType ScheduleTableID, TickType Start)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StartScheduleTableAbs(ScheduleTableID, Start);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StartScheduleTableAbs, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType StopScheduleTable(ScheduleTableType ScheduleTableID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32)ScheduleTableID, 0);
    }
    const StatusType result = Os_Implementation_StopScheduleTable(ScheduleTableID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_StopScheduleTable, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                             ScheduleTableType ScheduleTableID_To)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32)ScheduleTableID_To, 0);
    }
    const StatusType result =
        Os_Implementation_NextScheduleTable(ScheduleTableID_From, ScheduleTableID_To);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType GetScheduleTableStatus(ScheduleTableType ScheduleTableID,
                                  ScheduleTableStatusRefType ScheduleStatus)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetScheduleTableStatus, (uint32)ScheduleTableID, 0);
    }
    const StatusType result =
        Os_Implementation_GetScheduleTableStatus(ScheduleTableID, ScheduleStatus);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_GetScheduleTableStatus,
                       (result == E_OK) ? (uint32)*ScheduleStatus : UINT32_MAX,
                       1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType DisableInterruptSource(ISRType ISRID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_DisableInterruptSource(ISRID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableInterruptSource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType EnableInterruptSource(ISRType ISRID, boolean ClearPending)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_EnableInterruptSource(ISRID, ClearPending);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableInterruptSource, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

StatusType ClearPendingInterrupt(ISRType ISRID)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32)ISRID, 0);
    }
    const StatusType result = Os_Implementation_ClearPendingInterrupt(ISRID);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
    }
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ClearPendingInterrupt, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

void EnableAllInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_EnableAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_EnableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void DisableAllInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_DisableAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_DisableAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void ResumeAllInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_ResumeAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void SuspendAllInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_SuspendAllInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendAllInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void ResumeOSInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_ResumeOSInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_ResumeOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}

void SuspendOSInterrupts(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, (uint32)0u, 0);
    }
    Os_Implementation_SuspendOSInterrupts();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_SuspendOSInterrupts, 0u, 1);
    }
    Os_ArtiServiceLeave();
}
