#ifndef AUTOSAR_EPIC4_OS_HOOKS_H
#define AUTOSAR_EPIC4_OS_HOOKS_H
#include "Os_Types.h"
#include "Os_Cfg.h"
typedef struct {
    void (*error)(StatusType Error);
    void (*pre_task)(void);
    void (*post_task)(void);
} Os_HookConfig;
typedef enum {
    OS_HOOK_NONE = 0,
    OS_HOOK_ERROR,
    OS_HOOK_PRE,
    OS_HOOK_POST,
    OS_HOOK_STARTUP,
    OS_HOOK_SHUTDOWN,
    OS_HOOK_ALARM
} Os_HookPhase;
#define OSServiceId_Unknown ((OSServiceIdType)0u)
#define OSServiceId_GetISRID ((OSServiceIdType)0x01u)
#define OSServiceId_DisableInterruptSource ((OSServiceIdType)0x30u)
#define OSServiceId_EnableInterruptSource ((OSServiceIdType)0x31u)
#define OSServiceId_ClearPendingInterrupt ((OSServiceIdType)0x32u)
#define OSServiceId_InterruptMissingEnd ((OSServiceIdType)0xfcu)
#define OSServiceId_TaskMissingEnd ((OSServiceIdType)0xfdu)
#define OSServiceId_GetTaskID ((OSServiceIdType)0x80u)
#define OSServiceId_GetTaskState ((OSServiceIdType)0x81u)
#define OSServiceId_ActivateTask ((OSServiceIdType)0x82u)
#define OSServiceId_TerminateTask ((OSServiceIdType)0x83u)
#define OSServiceId_ChainTask ((OSServiceIdType)0x84u)
#define OSServiceId_GetResource ((OSServiceIdType)0x85u)
#define OSServiceId_ReleaseResource ((OSServiceIdType)0x86u)
#define OSServiceId_Schedule ((OSServiceIdType)0x87u)
#define OSServiceId_WaitEvent ((OSServiceIdType)0x88u)
#define OSServiceId_ClearEvent ((OSServiceIdType)0x89u)
#define OSServiceId_SetEvent ((OSServiceIdType)0x8au)
#define OSServiceId_GetEvent ((OSServiceIdType)0x8bu)
#define OSServiceId_IncrementCounter ((OSServiceIdType)0xfu)
#define OSServiceId_GetCounterValue ((OSServiceIdType)0x10u)
#define OSServiceId_GetElapsedValue ((OSServiceIdType)0x11u)
#define OSServiceId_GetAlarmBase ((OSServiceIdType)0x8cu)
#define OSServiceId_GetAlarm ((OSServiceIdType)0x8du)
#define OSServiceId_SetRelAlarm ((OSServiceIdType)0x8eu)
#define OSServiceId_SetAbsAlarm ((OSServiceIdType)0x8fu)
#define OSServiceId_CancelAlarm ((OSServiceIdType)0x90u)
#define OSServiceId_EnableAllInterrupts ((OSServiceIdType)0x91u)
#define OSServiceId_DisableAllInterrupts ((OSServiceIdType)0x92u)
#define OSServiceId_ResumeAllInterrupts ((OSServiceIdType)0x93u)
#define OSServiceId_SuspendAllInterrupts ((OSServiceIdType)0x94u)
#define OSServiceId_ResumeOSInterrupts ((OSServiceIdType)0x95u)
#define OSServiceId_SuspendOSInterrupts ((OSServiceIdType)0x96u)
#define OSServiceId_ShutdownOS ((OSServiceIdType)0x97u)
#define OSServiceId_StartOS ((OSServiceIdType)0x98u)
#define OSServiceId_GetActiveApplicationMode ((OSServiceIdType)0x99u)
#define OSServiceId_StartScheduleTableRel ((OSServiceIdType)0x7u)
#define OSServiceId_StartScheduleTableAbs ((OSServiceIdType)0x8u)
#define OSServiceId_StopScheduleTable ((OSServiceIdType)0x9u)
#define OSServiceId_NextScheduleTable ((OSServiceIdType)0xau)
#define OSServiceId_GetScheduleTableStatus ((OSServiceIdType)0xeu)
typedef union {
    struct {
        ISRType ISRID;
        boolean ClearPending;
    } service_EnableInterruptSource;
    struct {
        ISRType ISRID;
    } service_DisableInterruptSource;
    struct {
        ISRType ISRID;
    } service_ClearPendingInterrupt;
    struct {
        AppModeType Mode;
    } service_StartOS;
    struct {
        StatusType Error;
    } service_ShutdownOS;
    struct {
        TaskRefType TaskID;
    } service_GetTaskID;
    struct {
        TaskType TaskID;
        TaskStateRefType State;
    } service_GetTaskState;
    struct {
        TaskType TaskID;
    } service_ActivateTask;
    struct {
        uint8_t reserved;
    } service_TerminateTask;
    struct {
        TaskType TaskID;
    } service_ChainTask;
    struct {
        ResourceType ResID;
    } service_GetResource;
    struct {
        ResourceType ResID;
    } service_ReleaseResource;
    struct {
        uint8_t reserved;
    } service_Schedule;
    struct {
        EventMaskType Mask;
    } service_WaitEvent;
    struct {
        EventMaskType Mask;
    } service_ClearEvent;
    struct {
        TaskType TaskID;
        EventMaskType Mask;
    } service_SetEvent;
    struct {
        TaskType TaskID;
        EventMaskRefType Event;
    } service_GetEvent;
    struct {
        CounterType CounterID;
    } service_IncrementCounter;
    struct {
        CounterType CounterID;
        TickRefType Value;
    } service_GetCounterValue;
    struct {
        CounterType CounterID;
        TickRefType Value;
        TickRefType ElapsedValue;
    } service_GetElapsedValue;
    struct {
        AlarmType AlarmID;
        AlarmBaseRefType Info;
    } service_GetAlarmBase;
    struct {
        AlarmType AlarmID;
        TickRefType Tick;
    } service_GetAlarm;
    struct {
        AlarmType AlarmID;
        TickType Increment;
        TickType Cycle;
    } service_SetRelAlarm;
    struct {
        AlarmType AlarmID;
        TickType Start;
        TickType Cycle;
    } service_SetAbsAlarm;
    struct {
        AlarmType AlarmID;
    } service_CancelAlarm;
    struct {
        ScheduleTableType ScheduleTableID;
        TickType Offset;
    } service_StartScheduleTableRel;
    struct {
        ScheduleTableType ScheduleTableID;
        TickType Start;
    } service_StartScheduleTableAbs;
    struct {
        ScheduleTableType ScheduleTableID;
    } service_StopScheduleTable;
    struct {
        ScheduleTableType ScheduleTableID_From;
        ScheduleTableType ScheduleTableID_To;
    } service_NextScheduleTable;
    struct {
        ScheduleTableType ScheduleTableID;
        ScheduleTableStatusRefType ScheduleStatus;
    } service_GetScheduleTableStatus;
} Os_ErrorParameters;
/** Current failed-service snapshot; meaningful only within ErrorHook. */
const Os_ErrorParameters *Os_ErrorParametersCurrent(void);
/** Current failed standard-service identity; meaningful only within ErrorHook. */
OSServiceIdType Os_ErrorServiceId(void);
#if OS_USE_GET_SERVICE_ID
#define OSErrorGetServiceId() Os_ErrorServiceId()
#endif
#if OS_USE_PARAMETER_ACCESS
#define OSError_EnableInterruptSource_ISRID()                                                      \
    (Os_ErrorParametersCurrent()->service_EnableInterruptSource.ISRID)
#define OSError_EnableInterruptSource_ClearPending()                                               \
    (Os_ErrorParametersCurrent()->service_EnableInterruptSource.ClearPending)
#define OSError_DisableInterruptSource_ISRID()                                                     \
    (Os_ErrorParametersCurrent()->service_DisableInterruptSource.ISRID)
#define OSError_ClearPendingInterrupt_ISRID()                                                      \
    (Os_ErrorParametersCurrent()->service_ClearPendingInterrupt.ISRID)
#define OSError_StartOS_Mode() (Os_ErrorParametersCurrent()->service_StartOS.Mode)
#define OSError_ShutdownOS_Error() (Os_ErrorParametersCurrent()->service_ShutdownOS.Error)
#define OSError_GetTaskID_TaskID() (Os_ErrorParametersCurrent()->service_GetTaskID.TaskID)
#define OSError_GetTaskState_TaskID() (Os_ErrorParametersCurrent()->service_GetTaskState.TaskID)
#define OSError_GetTaskState_State() (Os_ErrorParametersCurrent()->service_GetTaskState.State)
#define OSError_ActivateTask_TaskID() (Os_ErrorParametersCurrent()->service_ActivateTask.TaskID)
#define OSError_ChainTask_TaskID() (Os_ErrorParametersCurrent()->service_ChainTask.TaskID)
#define OSError_GetResource_ResID() (Os_ErrorParametersCurrent()->service_GetResource.ResID)
#define OSError_ReleaseResource_ResID() (Os_ErrorParametersCurrent()->service_ReleaseResource.ResID)
#define OSError_WaitEvent_Mask() (Os_ErrorParametersCurrent()->service_WaitEvent.Mask)
#define OSError_ClearEvent_Mask() (Os_ErrorParametersCurrent()->service_ClearEvent.Mask)
#define OSError_SetEvent_TaskID() (Os_ErrorParametersCurrent()->service_SetEvent.TaskID)
#define OSError_SetEvent_Mask() (Os_ErrorParametersCurrent()->service_SetEvent.Mask)
#define OSError_GetEvent_TaskID() (Os_ErrorParametersCurrent()->service_GetEvent.TaskID)
#define OSError_GetEvent_Event() (Os_ErrorParametersCurrent()->service_GetEvent.Event)
#define OSError_IncrementCounter_CounterID()                                                       \
    (Os_ErrorParametersCurrent()->service_IncrementCounter.CounterID)
#define OSError_GetCounterValue_CounterID()                                                        \
    (Os_ErrorParametersCurrent()->service_GetCounterValue.CounterID)
#define OSError_GetCounterValue_Value() (Os_ErrorParametersCurrent()->service_GetCounterValue.Value)
#define OSError_GetElapsedValue_CounterID()                                                        \
    (Os_ErrorParametersCurrent()->service_GetElapsedValue.CounterID)
#define OSError_GetElapsedValue_Value() (Os_ErrorParametersCurrent()->service_GetElapsedValue.Value)
#define OSError_GetElapsedValue_ElapsedValue()                                                     \
    (Os_ErrorParametersCurrent()->service_GetElapsedValue.ElapsedValue)
#define OSError_GetAlarmBase_AlarmID() (Os_ErrorParametersCurrent()->service_GetAlarmBase.AlarmID)
#define OSError_GetAlarmBase_Info() (Os_ErrorParametersCurrent()->service_GetAlarmBase.Info)
#define OSError_GetAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_GetAlarm.AlarmID)
#define OSError_GetAlarm_Tick() (Os_ErrorParametersCurrent()->service_GetAlarm.Tick)
#define OSError_SetRelAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_SetRelAlarm.AlarmID)
#define OSError_SetRelAlarm_Increment() (Os_ErrorParametersCurrent()->service_SetRelAlarm.Increment)
#define OSError_SetRelAlarm_Cycle() (Os_ErrorParametersCurrent()->service_SetRelAlarm.Cycle)
#define OSError_SetAbsAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.AlarmID)
#define OSError_SetAbsAlarm_Start() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.Start)
#define OSError_SetAbsAlarm_Cycle() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.Cycle)
#define OSError_CancelAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_CancelAlarm.AlarmID)
#define OSError_StartScheduleTableRel_ScheduleTableID()                                            \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableRel.ScheduleTableID)
#define OSError_StartScheduleTableRel_Offset()                                                     \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableRel.Offset)
#define OSError_StartScheduleTableAbs_ScheduleTableID()                                            \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableAbs.ScheduleTableID)
#define OSError_StartScheduleTableAbs_Start()                                                      \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableAbs.Start)
#define OSError_StopScheduleTable_ScheduleTableID()                                                \
    (Os_ErrorParametersCurrent()->service_StopScheduleTable.ScheduleTableID)
#define OSError_NextScheduleTable_ScheduleTableID_From()                                           \
    (Os_ErrorParametersCurrent()->service_NextScheduleTable.ScheduleTableID_From)
#define OSError_NextScheduleTable_ScheduleTableID_To()                                             \
    (Os_ErrorParametersCurrent()->service_NextScheduleTable.ScheduleTableID_To)
#define OSError_GetScheduleTableStatus_ScheduleTableID()                                           \
    (Os_ErrorParametersCurrent()->service_GetScheduleTableStatus.ScheduleTableID)
#define OSError_GetScheduleTableStatus_ScheduleStatus()                                            \
    (Os_ErrorParametersCurrent()->service_GetScheduleTableStatus.ScheduleStatus)
#endif
void ErrorHook(StatusType Error);
void PreTaskHook(void);
void PostTaskHook(void);
#endif
