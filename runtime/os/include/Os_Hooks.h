/** @file OS hook configuration, service identifiers and ErrorHook parameter snapshots. */
#ifndef AUTOSAR_OS_HOOKS_H
#define AUTOSAR_OS_HOOKS_H
#include "Os_Types.h"
#include "Os_Cfg.h"
/** Static optional ErrorHook/Task-hook bindings, retained with Os_TargetConfigType for the process.
 */
typedef struct {
    void (*error)(
        StatusType Error); /**< Optional failed-service callback, valid for the process lifetime. */
    void (*pre_task)(
        void); /**< Optional callback before a Task runs, valid for the process lifetime. */
    void (*post_task)(void); /**< Optional callback before a Task leaves RUNNING, valid for the
                                process lifetime. */
} Os_HookConfigType;
/** Current logical actor hook phase; NONE means ordinary execution, ALARM an AlarmCallback. */
typedef enum {
    OS_HOOK_NONE = 0, /**< Ordinary actor execution, outside a hook. */
    OS_HOOK_ERROR,    /**< Current ErrorHook invocation. */
    OS_HOOK_PRE,      /**< Current PreTaskHook invocation. */
    OS_HOOK_POST,     /**< Current PostTaskHook invocation. */
    OS_HOOK_STARTUP,  /**< Current StartupHook invocation. */
    OS_HOOK_SHUTDOWN, /**< Current ShutdownHook invocation. */
    OS_HOOK_ALARM     /**< Current AlarmCallback invocation. */
} Os_HookPhaseType;
#define OSServiceId_Unknown                                                                        \
    ((OSServiceIdType)0u) /**< No active failed-service identity. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetISRID                                                                       \
    ((OSServiceIdType)0x01u) /**< Failed-service identity for GetISRID. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetCoreID                                                                      \
    ((OSServiceIdType)0x16u) /**< Failed-service identity for GetCoreID. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_ControlIdle                                                                    \
    ((OSServiceIdType)0x1du) /**< Failed-service identity for ControlIdle. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_isOsStarted                                                                    \
    ((OSServiceIdType)0x36u) /**< Failed-service identity for isOsStarted. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_DisableInterruptSource                                                         \
    ((OSServiceIdType)0x30u) /**< Failed-service identity for DisableInterruptSource. OSEK         \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_EnableInterruptSource                                                          \
    ((OSServiceIdType)0x31u) /**< Failed-service identity for EnableInterruptSource. OSEK          \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_ClearPendingInterrupt                                                          \
    ((OSServiceIdType)0x32u) /**< Failed-service identity for ClearPendingInterrupt. OSEK          \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_InterruptMissingEnd                                                            \
    ((OSServiceIdType)0xfcu) /**< Failed-service identity for InterruptMissingEnd. OSEK            \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_TaskMissingEnd                                                                 \
    ((OSServiceIdType)0xfdu) /**< Failed-service identity for TaskMissingEnd. OSEK                 \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_GetTaskID                                                                      \
    ((OSServiceIdType)0x80u) /**< Failed-service identity for GetTaskID. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetTaskState                                                                   \
    ((OSServiceIdType)0x81u) /**< Failed-service identity for GetTaskState. OSEK OS 2.2.3 13.8.3.  \
                              */
#define OSServiceId_ActivateTask                                                                   \
    ((OSServiceIdType)0x82u) /**< Failed-service identity for ActivateTask. OSEK OS 2.2.3 13.8.3.  \
                              */
#define OSServiceId_TerminateTask                                                                  \
    ((OSServiceIdType)0x83u) /**< Failed-service identity for TerminateTask. OSEK OS 2.2.3 13.8.3. \
                              */
#define OSServiceId_ChainTask                                                                      \
    ((OSServiceIdType)0x84u) /**< Failed-service identity for ChainTask. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetResource                                                                    \
    ((OSServiceIdType)0x85u) /**< Failed-service identity for GetResource. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_ReleaseResource                                                                \
    ((OSServiceIdType)0x86u) /**< Failed-service identity for ReleaseResource. OSEK                \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_Schedule                                                                       \
    ((OSServiceIdType)0x87u) /**< Failed-service identity for Schedule. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_WaitEvent                                                                      \
    ((OSServiceIdType)0x88u) /**< Failed-service identity for WaitEvent. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_ClearEvent                                                                     \
    ((OSServiceIdType)0x89u) /**< Failed-service identity for ClearEvent. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_SetEvent                                                                       \
    ((OSServiceIdType)0x8au) /**< Failed-service identity for SetEvent. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetEvent                                                                       \
    ((OSServiceIdType)0x8bu) /**< Failed-service identity for GetEvent. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_IncrementCounter                                                               \
    ((OSServiceIdType)0xfu) /**< Failed-service identity for IncrementCounter. OSEK                \
                               OS 2.2.3 13.8.3. */
#define OSServiceId_GetCounterValue                                                                \
    ((OSServiceIdType)0x10u) /**< Failed-service identity for GetCounterValue. OSEK                \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_GetElapsedValue                                                                \
    ((OSServiceIdType)0x11u) /**< Failed-service identity for GetElapsedValue. OSEK                \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_GetAlarmBase                                                                   \
    ((OSServiceIdType)0x8cu) /**< Failed-service identity for GetAlarmBase. OSEK OS 2.2.3 13.8.3.  \
                              */
#define OSServiceId_GetAlarm                                                                       \
    ((OSServiceIdType)0x8du) /**< Failed-service identity for GetAlarm. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_SetRelAlarm                                                                    \
    ((OSServiceIdType)0x8eu) /**< Failed-service identity for SetRelAlarm. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_SetAbsAlarm                                                                    \
    ((OSServiceIdType)0x8fu) /**< Failed-service identity for SetAbsAlarm. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_CancelAlarm                                                                    \
    ((OSServiceIdType)0x90u) /**< Failed-service identity for CancelAlarm. OSEK OS 2.2.3 13.8.3.   \
                              */
#define OSServiceId_EnableAllInterrupts                                                            \
    ((OSServiceIdType)0x91u) /**< Failed-service identity for EnableAllInterrupts. OSEK            \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_DisableAllInterrupts                                                           \
    ((OSServiceIdType)0x92u) /**< Failed-service identity for DisableAllInterrupts. OSEK           \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_ResumeAllInterrupts                                                            \
    ((OSServiceIdType)0x93u) /**< Failed-service identity for ResumeAllInterrupts. OSEK            \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_SuspendAllInterrupts                                                           \
    ((OSServiceIdType)0x94u) /**< Failed-service identity for SuspendAllInterrupts. OSEK           \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_ResumeOSInterrupts                                                             \
    ((OSServiceIdType)0x95u) /**< Failed-service identity for ResumeOSInterrupts. OSEK             \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_SuspendOSInterrupts                                                            \
    ((OSServiceIdType)0x96u) /**< Failed-service identity for SuspendOSInterrupts. OSEK            \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_ShutdownOS                                                                     \
    ((OSServiceIdType)0x97u) /**< Failed-service identity for ShutdownOS. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_StartOS                                                                        \
    ((OSServiceIdType)0x98u) /**< Failed-service identity for StartOS. OSEK OS 2.2.3 13.8.3. */
#define OSServiceId_GetActiveApplicationMode                                                       \
    ((OSServiceIdType)0x99u) /**< Failed-service identity for GetActiveApplicationMode. OSEK       \
                                OS 2.2.3 13.8.3. */
#define OSServiceId_StartScheduleTableRel                                                          \
    ((OSServiceIdType)0x7u) /**< Failed-service identity for StartScheduleTableRel. OSEK           \
                               OS 2.2.3 13.8.3. */
#define OSServiceId_StartScheduleTableAbs                                                          \
    ((OSServiceIdType)0x8u) /**< Failed-service identity for StartScheduleTableAbs. OSEK           \
                               OS 2.2.3 13.8.3. */
#define OSServiceId_StopScheduleTable                                                              \
    ((OSServiceIdType)0x9u) /**< Failed-service identity for StopScheduleTable. OSEK               \
                               OS 2.2.3 13.8.3. */
#define OSServiceId_NextScheduleTable                                                              \
    ((OSServiceIdType)0xau) /**< Failed-service identity for NextScheduleTable. OSEK               \
                               OS 2.2.3 13.8.3. */
#define OSServiceId_GetScheduleTableStatus                                                         \
    ((OSServiceIdType)0xeu) /**< Failed-service identity for GetScheduleTableStatus. OSEK          \
                               OS 2.2.3 13.8.3. */
/** Typed failed-service arguments; active only during the matching ErrorHook invocation. */
typedef union {
    struct {
        CoreIdType CoreID;     /**< Original ControlIdle core argument; may be foreign. */
        IdleModeType IdleMode; /**< Original ControlIdle mode argument; may be invalid. */
    } service_ControlIdle;     /**< Failed ControlIdle parameter snapshot. */
    struct {
        ISRType ISRID;               /**< Original interrupt-source identifier; may be invalid. */
        boolean ClearPending;        /**< Original pending-edge clear selection. */
    } service_EnableInterruptSource; /**< Failed EnableInterruptSource parameter snapshot. */
    struct {
        ISRType ISRID;                /**< Original interrupt-source identifier; may be invalid. */
    } service_DisableInterruptSource; /**< Failed DisableInterruptSource parameter snapshot. */
    struct {
        ISRType ISRID;               /**< Original interrupt-source identifier; may be invalid. */
    } service_ClearPendingInterrupt; /**< Failed ClearPendingInterrupt parameter snapshot. */
    struct {
        AppModeType Mode; /**< Original startup application-mode argument. */
    } service_StartOS;    /**< Failed StartOS parameter snapshot. */
    struct {
        StatusType Error; /**< Original shutdown reason. */
    } service_ShutdownOS; /**< Failed ShutdownOS parameter snapshot. */
    struct {
        TaskRefType TaskID; /**< Original Task identifier or output pointer for the selected failed
                               service. */
    } service_GetTaskID;    /**< Failed GetTaskID parameter snapshot. */
    struct {
        TaskType TaskID; /**< Original Task identifier or output pointer for the selected failed
                            service. */
        TaskStateRefType State; /**< Original Task-state output pointer; may be NULL on refusal. */
    } service_GetTaskState;     /**< Failed GetTaskState parameter snapshot. */
    struct {
        TaskType TaskID;    /**< Original Task identifier or output pointer for the selected failed
                               service. */
    } service_ActivateTask; /**< Failed ActivateTask parameter snapshot. */
    struct {
        uint8 reserved;      /**< Reserved zero byte for an argument-free service. */
    } service_TerminateTask; /**< Failed TerminateTask parameter snapshot. */
    struct {
        TaskType TaskID; /**< Original Task identifier or output pointer for the selected failed
                            service. */
    } service_ChainTask; /**< Failed ChainTask parameter snapshot. */
    struct {
        ResourceType ResID; /**< Original resource identifier; may be invalid. */
    } service_GetResource;  /**< Failed GetResource parameter snapshot. */
    struct {
        ResourceType ResID;    /**< Original resource identifier; may be invalid. */
    } service_ReleaseResource; /**< Failed ReleaseResource parameter snapshot. */
    struct {
        uint8 reserved; /**< Reserved zero byte for an argument-free service. */
    } service_Schedule; /**< Failed Schedule parameter snapshot. */
    struct {
        EventMaskType Mask; /**< Original event-bit mask. */
    } service_WaitEvent;    /**< Failed WaitEvent parameter snapshot. */
    struct {
        EventMaskType Mask; /**< Original event-bit mask. */
    } service_ClearEvent;   /**< Failed ClearEvent parameter snapshot. */
    struct {
        TaskType TaskID;    /**< Original Task identifier or output pointer for the selected failed
                               service. */
        EventMaskType Mask; /**< Original event-bit mask. */
    } service_SetEvent;     /**< Failed SetEvent parameter snapshot. */
    struct {
        TaskType TaskID; /**< Original Task identifier or output pointer for the selected failed
                            service. */
        EventMaskRefType Event; /**< Original event-mask output pointer; may be NULL on refusal. */
    } service_GetEvent;         /**< Failed GetEvent parameter snapshot. */
    struct {
        CounterType CounterID;  /**< Original Counter identifier; may be invalid. */
    } service_IncrementCounter; /**< Failed IncrementCounter parameter snapshot. */
    struct {
        CounterType CounterID; /**< Original Counter identifier; may be invalid. */
        TickRefType Value; /**< Original Counter sample/result pointer; may be NULL on refusal. */
    } service_GetCounterValue; /**< Failed GetCounterValue parameter snapshot. */
    struct {
        CounterType CounterID; /**< Original Counter identifier; may be invalid. */
        TickRefType Value; /**< Original Counter sample/result pointer; may be NULL on refusal. */
        TickRefType
            ElapsedValue;      /**< Original elapsed-tick output pointer; may be NULL on refusal. */
    } service_GetElapsedValue; /**< Failed GetElapsedValue parameter snapshot. */
    struct {
        AlarmType AlarmID;     /**< Original Alarm identifier; may be invalid. */
        AlarmBaseRefType Info; /**< Original AlarmBase output pointer; may be NULL on refusal. */
    } service_GetAlarmBase;    /**< Failed GetAlarmBase parameter snapshot. */
    struct {
        AlarmType AlarmID; /**< Original Alarm identifier; may be invalid. */
        TickRefType
            Tick; /**< Original Alarm remaining-tick output pointer; may be NULL on refusal. */
    } service_GetAlarm; /**< Failed GetAlarm parameter snapshot. */
    struct {
        AlarmType AlarmID;  /**< Original Alarm identifier; may be invalid. */
        TickType Increment; /**< Original relative Alarm increment in Counter ticks. */
        TickType Cycle; /**< Original cyclic Alarm period in Counter ticks; zero means one-shot. */
    } service_SetRelAlarm; /**< Failed SetRelAlarm parameter snapshot. */
    struct {
        AlarmType AlarmID; /**< Original Alarm identifier; may be invalid. */
        TickType Start;    /**< Original absolute Counter tick value. */
        TickType Cycle; /**< Original cyclic Alarm period in Counter ticks; zero means one-shot. */
    } service_SetAbsAlarm; /**< Failed SetAbsAlarm parameter snapshot. */
    struct {
        AlarmType AlarmID; /**< Original Alarm identifier; may be invalid. */
    } service_CancelAlarm; /**< Failed CancelAlarm parameter snapshot. */
    struct {
        ScheduleTableType
            ScheduleTableID;         /**< Original ScheduleTable identifier; may be invalid. */
        TickType Offset;             /**< Original relative table offset in Counter ticks. */
    } service_StartScheduleTableRel; /**< Failed StartScheduleTableRel parameter snapshot. */
    struct {
        ScheduleTableType
            ScheduleTableID;         /**< Original ScheduleTable identifier; may be invalid. */
        TickType Start;              /**< Original absolute Counter tick value. */
    } service_StartScheduleTableAbs; /**< Failed StartScheduleTableAbs parameter snapshot. */
    struct {
        ScheduleTableType
            ScheduleTableID;     /**< Original ScheduleTable identifier; may be invalid. */
    } service_StopScheduleTable; /**< Failed StopScheduleTable parameter snapshot. */
    struct {
        ScheduleTableType ScheduleTableID_From; /**< Original source table identifier. */
        ScheduleTableType ScheduleTableID_To;   /**< Original successor table identifier. */
    } service_NextScheduleTable;                /**< Failed NextScheduleTable parameter snapshot. */
    struct {
        ScheduleTableType
            ScheduleTableID; /**< Original ScheduleTable identifier; may be invalid. */
        ScheduleTableStatusRefType
            ScheduleStatus; /**< Original table-state output pointer; may be NULL on refusal. */
    } service_GetScheduleTableStatus; /**< Failed GetScheduleTableStatus parameter snapshot. */
} Os_ErrorParametersType;
/** Current failed-service snapshot; meaningful only within ErrorHook.
 * @return Current failed-service arguments, or the immutable empty snapshot outside ErrorHook.
 * Preconditions: called while ErrorHook is active; otherwise returns an empty snapshot.
 * Context: current ErrorHook, including an interrupted logical ISR; no blocking or area entry.
 * Reentrancy: actor-local TLS; nested errors do not overwrite the active snapshot.
 * Ownership: returned pointer is borrowed only for the current ErrorHook; never modify or retain
 * it.
 */
const Os_ErrorParametersType *Os_ErrorParametersCurrent(void);
/** Current failed standard-service identity; meaningful only within ErrorHook.
 * @return Current failed-service identifier, or OSServiceId_Unknown outside ErrorHook.
 * Preconditions: called while ErrorHook is active; otherwise returns OSServiceId_Unknown.
 * Context: current ErrorHook; no blocking or area entry.
 * Reentrancy: actor-local TLS; nested errors preserve the active service identity.
 * Ownership: returns a scalar copy; no storage is transferred.
 */
OSServiceIdType Os_ErrorServiceId(void);
#if (OS_USE_GET_SERVICE_ID == STD_ON)
/** Read the captured service identifier of failed OS service (OSEK OS 2.2.3 13.8.4).
 * @return Captured service identifier value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSErrorGetServiceId() Os_ErrorServiceId()
#endif
#if (OS_USE_PARAMETER_ACCESS == STD_ON)
/** Read the captured CoreID of ControlIdle (OSEK OS 2.2.3 13.8.4).
 * @return Captured CoreID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ControlIdle_CoreID() (Os_ErrorParametersCurrent()->service_ControlIdle.CoreID)
/** Read the captured IdleMode of ControlIdle (OSEK OS 2.2.3 13.8.4).
 * @return Captured IdleMode value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ControlIdle_IdleMode() (Os_ErrorParametersCurrent()->service_ControlIdle.IdleMode)
/** Read the captured ISRID of EnableInterruptSource (OSEK OS 2.2.3 13.8.4).
 * @return Captured ISRID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_EnableInterruptSource_ISRID()                                                      \
    (Os_ErrorParametersCurrent()->service_EnableInterruptSource.ISRID)
/** Read the captured ClearPending of EnableInterruptSource (OSEK OS 2.2.3 13.8.4).
 * @return Captured ClearPending value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_EnableInterruptSource_ClearPending()                                               \
    (Os_ErrorParametersCurrent()->service_EnableInterruptSource.ClearPending)
/** Read the captured ISRID of DisableInterruptSource (OSEK OS 2.2.3 13.8.4).
 * @return Captured ISRID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_DisableInterruptSource_ISRID()                                                     \
    (Os_ErrorParametersCurrent()->service_DisableInterruptSource.ISRID)
/** Read the captured ISRID of ClearPendingInterrupt (OSEK OS 2.2.3 13.8.4).
 * @return Captured ISRID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ClearPendingInterrupt_ISRID()                                                      \
    (Os_ErrorParametersCurrent()->service_ClearPendingInterrupt.ISRID)
/** Read the captured Mode of StartOS (OSEK OS 2.2.3 13.8.4).
 * @return Captured Mode value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StartOS_Mode() (Os_ErrorParametersCurrent()->service_StartOS.Mode)
/** Read the captured Error of ShutdownOS (OSEK OS 2.2.3 13.8.4).
 * @return Captured Error value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ShutdownOS_Error() (Os_ErrorParametersCurrent()->service_ShutdownOS.Error)
/** Read the captured TaskID of GetTaskID (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetTaskID_TaskID() (Os_ErrorParametersCurrent()->service_GetTaskID.TaskID)
/** Read the captured TaskID of GetTaskState (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetTaskState_TaskID() (Os_ErrorParametersCurrent()->service_GetTaskState.TaskID)
/** Read the captured State of GetTaskState (OSEK OS 2.2.3 13.8.4).
 * @return Captured State value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetTaskState_State() (Os_ErrorParametersCurrent()->service_GetTaskState.State)
/** Read the captured TaskID of ActivateTask (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ActivateTask_TaskID() (Os_ErrorParametersCurrent()->service_ActivateTask.TaskID)
/** Read the captured TaskID of ChainTask (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ChainTask_TaskID() (Os_ErrorParametersCurrent()->service_ChainTask.TaskID)
/** Read the captured ResID of GetResource (OSEK OS 2.2.3 13.8.4).
 * @return Captured ResID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetResource_ResID() (Os_ErrorParametersCurrent()->service_GetResource.ResID)
/** Read the captured ResID of ReleaseResource (OSEK OS 2.2.3 13.8.4).
 * @return Captured ResID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ReleaseResource_ResID() (Os_ErrorParametersCurrent()->service_ReleaseResource.ResID)
/** Read the captured Mask of WaitEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured Mask value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_WaitEvent_Mask() (Os_ErrorParametersCurrent()->service_WaitEvent.Mask)
/** Read the captured Mask of ClearEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured Mask value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_ClearEvent_Mask() (Os_ErrorParametersCurrent()->service_ClearEvent.Mask)
/** Read the captured TaskID of SetEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetEvent_TaskID() (Os_ErrorParametersCurrent()->service_SetEvent.TaskID)
/** Read the captured Mask of SetEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured Mask value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetEvent_Mask() (Os_ErrorParametersCurrent()->service_SetEvent.Mask)
/** Read the captured TaskID of GetEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured TaskID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetEvent_TaskID() (Os_ErrorParametersCurrent()->service_GetEvent.TaskID)
/** Read the captured Event of GetEvent (OSEK OS 2.2.3 13.8.4).
 * @return Captured Event value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetEvent_Event() (Os_ErrorParametersCurrent()->service_GetEvent.Event)
/** Read the captured CounterID of IncrementCounter (OSEK OS 2.2.3 13.8.4).
 * @return Captured CounterID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_IncrementCounter_CounterID()                                                       \
    (Os_ErrorParametersCurrent()->service_IncrementCounter.CounterID)
/** Read the captured CounterID of GetCounterValue (OSEK OS 2.2.3 13.8.4).
 * @return Captured CounterID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetCounterValue_CounterID()                                                        \
    (Os_ErrorParametersCurrent()->service_GetCounterValue.CounterID)
/** Read the captured Value of GetCounterValue (OSEK OS 2.2.3 13.8.4).
 * @return Captured Value value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetCounterValue_Value() (Os_ErrorParametersCurrent()->service_GetCounterValue.Value)
/** Read the captured CounterID of GetElapsedValue (OSEK OS 2.2.3 13.8.4).
 * @return Captured CounterID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetElapsedValue_CounterID()                                                        \
    (Os_ErrorParametersCurrent()->service_GetElapsedValue.CounterID)
/** Read the captured Value of GetElapsedValue (OSEK OS 2.2.3 13.8.4).
 * @return Captured Value value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetElapsedValue_Value() (Os_ErrorParametersCurrent()->service_GetElapsedValue.Value)
/** Read the captured ElapsedValue of GetElapsedValue (OSEK OS 2.2.3 13.8.4).
 * @return Captured ElapsedValue value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetElapsedValue_ElapsedValue()                                                     \
    (Os_ErrorParametersCurrent()->service_GetElapsedValue.ElapsedValue)
/** Read the captured AlarmID of GetAlarmBase (OSEK OS 2.2.3 13.8.4).
 * @return Captured AlarmID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetAlarmBase_AlarmID() (Os_ErrorParametersCurrent()->service_GetAlarmBase.AlarmID)
/** Read the captured Info of GetAlarmBase (OSEK OS 2.2.3 13.8.4).
 * @return Captured Info value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetAlarmBase_Info() (Os_ErrorParametersCurrent()->service_GetAlarmBase.Info)
/** Read the captured AlarmID of GetAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured AlarmID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_GetAlarm.AlarmID)
/** Read the captured Tick of GetAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured Tick value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetAlarm_Tick() (Os_ErrorParametersCurrent()->service_GetAlarm.Tick)
/** Read the captured AlarmID of SetRelAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured AlarmID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetRelAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_SetRelAlarm.AlarmID)
/** Read the captured Increment of SetRelAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured Increment value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetRelAlarm_Increment() (Os_ErrorParametersCurrent()->service_SetRelAlarm.Increment)
/** Read the captured Cycle of SetRelAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured Cycle value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetRelAlarm_Cycle() (Os_ErrorParametersCurrent()->service_SetRelAlarm.Cycle)
/** Read the captured AlarmID of SetAbsAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured AlarmID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetAbsAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.AlarmID)
/** Read the captured Start of SetAbsAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured Start value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetAbsAlarm_Start() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.Start)
/** Read the captured Cycle of SetAbsAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured Cycle value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_SetAbsAlarm_Cycle() (Os_ErrorParametersCurrent()->service_SetAbsAlarm.Cycle)
/** Read the captured AlarmID of CancelAlarm (OSEK OS 2.2.3 13.8.4).
 * @return Captured AlarmID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_CancelAlarm_AlarmID() (Os_ErrorParametersCurrent()->service_CancelAlarm.AlarmID)
/** Read the captured ScheduleTableID of StartScheduleTableRel (OSEK OS 2.2.3 13.8.4).
 * @return Captured ScheduleTableID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StartScheduleTableRel_ScheduleTableID()                                            \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableRel.ScheduleTableID)
/** Read the captured Offset of StartScheduleTableRel (OSEK OS 2.2.3 13.8.4).
 * @return Captured Offset value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StartScheduleTableRel_Offset()                                                     \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableRel.Offset)
/** Read the captured ScheduleTableID of StartScheduleTableAbs (OSEK OS 2.2.3 13.8.4).
 * @return Captured ScheduleTableID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StartScheduleTableAbs_ScheduleTableID()                                            \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableAbs.ScheduleTableID)
/** Read the captured Start of StartScheduleTableAbs (OSEK OS 2.2.3 13.8.4).
 * @return Captured Start value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StartScheduleTableAbs_Start()                                                      \
    (Os_ErrorParametersCurrent()->service_StartScheduleTableAbs.Start)
/** Read the captured ScheduleTableID of StopScheduleTable (OSEK OS 2.2.3 13.8.4).
 * @return Captured ScheduleTableID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_StopScheduleTable_ScheduleTableID()                                                \
    (Os_ErrorParametersCurrent()->service_StopScheduleTable.ScheduleTableID)
/** Read the captured From of NextScheduleTable_ScheduleTableID (OSEK OS 2.2.3 13.8.4).
 * @return Captured From value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_NextScheduleTable_ScheduleTableID_From()                                           \
    (Os_ErrorParametersCurrent()->service_NextScheduleTable.ScheduleTableID_From)
/** Read the captured To of NextScheduleTable_ScheduleTableID (OSEK OS 2.2.3 13.8.4).
 * @return Captured To value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_NextScheduleTable_ScheduleTableID_To()                                             \
    (Os_ErrorParametersCurrent()->service_NextScheduleTable.ScheduleTableID_To)
/** Read the captured ScheduleTableID of GetScheduleTableStatus (OSEK OS 2.2.3 13.8.4).
 * @return Captured ScheduleTableID value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetScheduleTableStatus_ScheduleTableID()                                           \
    (Os_ErrorParametersCurrent()->service_GetScheduleTableStatus.ScheduleTableID)
/** Read the captured ScheduleStatus of GetScheduleTableStatus (OSEK OS 2.2.3 13.8.4).
 * @return Captured ScheduleStatus value, including its original pointer value when applicable.
 * Preconditions: active ErrorHook for the matching failed service; required accessor feature
 * enabled. Context: current ErrorHook only, under its existing OS critical boundary; no blocking.
 * Reentrancy: actor-local snapshot; nested service errors do not replace it.
 * Ownership: borrows the failed call snapshot until ErrorHook returns; does not copy or own
 * pointed-to storage.
 */
#define OSError_GetScheduleTableStatus_ScheduleStatus()                                            \
    (Os_ErrorParametersCurrent()->service_GetScheduleTableStatus.ScheduleStatus)
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Observe a failed OS service before its caller resumes (OSEK OS 2.2.3 13.8.2.1).
 * @param[in] Error Non-E_OK result of the failed service.
 * @par Return
 * None.
 * Preconditions: invoked by the OS, with configured callback storage valid for the process.
 * Context: OS hook under the existing Task/ISR critical boundary; only hook-permitted services, no
 * BSW exclusive areas. Reentrancy: non-reentrant in one logical actor; ErrorHook recursion is
 * suppressed. Ownership: ErrorHook accessors borrow the active snapshot until return; no pointer is
 * transferred.
 */
OS_CODE void ErrorHook(StatusType Error);
/** Observe a Task after entering RUNNING and before its body resumes (OSEK 13.8.2.2).
 * @par Return
 * None.
 * Preconditions: invoked by the OS, with configured callback storage valid for the process.
 * Context: OS hook under the existing Task/ISR critical boundary; only hook-permitted services, no
 * BSW exclusive areas. Reentrancy: non-reentrant in one logical actor; ErrorHook recursion is
 * suppressed. Ownership: ErrorHook accessors borrow the active snapshot until return; no pointer is
 * transferred.
 */
OS_CODE void PreTaskHook(void);
/** Observe a Task before leaving RUNNING (OSEK OS 2.2.3 13.8.2.3).
 * @par Return
 * None.
 * Preconditions: invoked by the OS, with configured callback storage valid for the process.
 * Context: OS hook under the existing Task/ISR critical boundary; only hook-permitted services, no
 * BSW exclusive areas. Reentrancy: non-reentrant in one logical actor; ErrorHook recursion is
 * suppressed. Ownership: ErrorHook accessors borrow the active snapshot until return; no pointer is
 * transferred.
 */
OS_CODE void PostTaskHook(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
