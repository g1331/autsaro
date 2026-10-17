/** @file Private backend operations shared across the OS sources. */
#ifndef AUTOSAR_OS_BACKEND_H
#define AUTOSAR_OS_BACKEND_H
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include "Os_Host.h"
#include "Os_Stack.h"
#include "Os_Vector.h"
#include "Os_Mailbox.h"
#include "Os_Arti.h"
#define OS_KERNEL_YIELD_INTERRUPT    0u
#define OS_CONTROLLED_TICK_INTERRUPT 1u
typedef struct {
    Os_HookPhase phase;
    OSServiceIdType service;
    Os_ErrorParameters parameters;
    Os_ArtiCaller arti;
} Os_ErrorContext;
void Os_ErrorContextSave(Os_ErrorContext *context);
void Os_ErrorContextRestore(const Os_ErrorContext *context);
sint32 Os_HookBlocksCategory2(void);
uint32 Os_BackendInterruptPriority(uint32 interrupt);
sint32 Os_BackendInterruptMaySchedule(uint32 interrupt);
void Os_BackendRequestIsrReschedule(void);
void Os_PortDispatchNested(void);
void Os_PortPostInterrupt(uint32 interrupt);
sint32 Os_BackendInputOwner(void);
sint32 Os_BackendTaskOwner(TaskType id);
sint32 Os_BackendServiceContext(void);
StatusType Os_BridgeContext(void);
sint32 Os_BridgeActorAllowed(void);
sint32 Os_MailboxQuiescent(void);
StatusType Os_TimeValidate(const Os_TargetConfig *target);
void Os_TimeInit(AppModeType mode);
void Os_TimeAutostart(AppModeType mode);
StatusType Os_ScheduleValidate(const Os_TargetConfig *target);
void Os_ScheduleAutostart(AppModeType mode);
void Os_ScheduleIncrement(CounterType counter);
TickType Os_TimeCounterValue(CounterType counter);
void Os_TimeReportAction(StatusType status);
void Os_TimeClose(void);
sint32 Os_TimeSignalFailed(void);
void Os_TimeTick(void);
void vApplicationTickHook(void);
sint32 Os_TimeBeginTick(void);
#if defined(OS_IDLE_TESTS) && (OS_IDLE_TESTS == STD_ON)
void Os_TestIdleObserved(void);
#endif
#if defined(OS_ARTI_TESTS) && (OS_ARTI_TESTS == STD_ON)
void Os_ArtiTestShutdownObserved(void);
#endif
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
void Os_TimeTestSeed(uint64 epoch, uint32 kernel_tick, TickType value);
void Os_TimeTestCloseEvent(void);
void Os_TimeTestBeforePending(void);
void Os_TimeTestMasked(void);
void Os_TimeTestBeforeReady(void);
#endif
void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);
void Os_MailboxInstall(void);
void Os_MailboxClose(void);
sint32 Os_MailboxHandlerAllowed(uint32 interrupt, uint32 (*handler)(void));
sint32 Os_BackendHandlerAllowed(uint32 interrupt, uint32 (*handler)(void));
#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
void Os_TestObserve(void);
void Os_TestInputNotified(void);
void Os_TestInputLastTicket(uint64 ticket);
#endif
void Os_BackendStart(AppModeType mode);
void Os_BackendShutdown(StatusType error);
void Os_BackendRequestShutdown(StatusType error);
#ifdef __linux__
void Os_BackendSignalStackFault(char role);
void Os_BackendSetDispatcher(void);
#endif
void Os_BackendGuardService(void);
sint32 Os_BackendReady(void);
Os_HookPhase Os_HookContext(void);
sint32 Os_HookServiceAllowed(OSServiceIdType service);
StatusType Os_ServiceAccessStatus(OSServiceIdType service);
uint32 Os_BackendCurrentInterrupt(void);
sint32 Os_BackendStarted(void);
sint32 Os_InterruptAllows(uint32 interrupt);
sint32 Os_PortInterruptSourceEnabled(uint32 interrupt);
/* Native mutex held: 0 invalid, 1 applied, 2 duplicate pair with no side effect. */
sint32 Os_PortInterruptSourceControl(uint32 interrupt, uint32 operation, boolean clear);
sint32 Os_InterruptDisabled(void);
void Os_InterruptRestoreOwner(void);
sint32 Os_HookQueryContext(void);
sint32 Os_ErrorHookConfigured(void);
StatusType Os_ErrorResult(OSServiceIdType service,
                          StatusType status,
                          const Os_ErrorParameters *arguments);
void Os_HookInvoke(void (*hook)(void), Os_HookPhase selected);
void Os_ShutdownHookInvoke(StatusType error);
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
void Os_TimeTestDispatchFinishing(void);
void Os_TimeTestDispatchUnlocked(void);
void Os_TimeTestOnWaiting(void);
extern volatile BaseType_t xInsideInterrupt;
#endif
StatusType Os_BackendState(TaskType id, TaskStateRefType state);
StatusType Os_BackendTaskId(TaskRefType id);
AppModeType Os_BackendApplicationMode(void);
StatusType Os_BackendActivate(TaskType id);
StatusType Os_BackendFinish(void);
void Os_BackendMissingEnd(void);
StatusType Os_BackendChain(TaskType id);
StatusType Os_BackendResource(ResourceType id, sint32 acquire);
StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output);
StatusType Os_BackendSchedule(void);
StatusType Os_BackendWait(EventMaskType mask);
StatusType Os_BackendClear(EventMaskType mask);
sint32 Os_BackendInterruptEnabled(uint32 interrupt);
void Os_BackendInterruptEnter(uint32 interrupt);
void Os_BackendInterruptLeave(void);
StatusType Os_BackendInspect(TaskType id, Os_ActivationInfo *info);
void Os_BackendOnSwitch(void);
void Os_BackendBeforeSelect(void *current, void *next);
sint32 Os_BackendTakeIsrReschedule(void);
void Os_BackendAssert(const char *file, sint32 line);
HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name);
HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);
HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes,
                     SIZE_T stack,
                     LPTHREAD_START_ROUTINE start,
                     LPVOID argument,
                     DWORD flags,
                     LPDWORD id);
HANDLE Os_PortTaskThread(TaskFunction_t code, void *argument, const StackType_t *buffer_top);
void Os_PortResumeThread(HANDLE thread);
DWORD Os_PortSuspendThread(HANDLE thread);
BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context);
#if defined(__linux__) && (defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON))
void Os_StackTestTriggerBackupFault(void);
#endif

/* Private implementations behind application-only ARTI service wrappers. */
ISRType Os_Implementation_GetISRID(void);
StatusType Os_Implementation_ControlIdle(CoreIdType CoreID, IdleModeType IdleMode);
boolean Os_Implementation_isOsStarted(void);
CoreIdType Os_Implementation_GetCoreID(void);
/* Index of the configured resource `id`, or resource_count when absent. */
size_t Os_ResourceIndex(const Os_TargetConfig *config, ResourceType id);
/* Index of the STANDARD resource that the LINKED chain from `index` names, or
 * resource_count for a missing or cyclic link. */
size_t Os_ResourceRoot(const Os_TargetConfig *config, size_t index);
StatusType Os_Implementation_GetTaskID(TaskRefType TaskID);
StatusType Os_Implementation_GetTaskState(TaskType TaskID, TaskStateRefType State);
StatusType Os_Implementation_ActivateTask(TaskType TaskID);
StatusType Os_Implementation_TerminateTask(void);
StatusType Os_Implementation_ChainTask(TaskType TaskID);
StatusType Os_Implementation_GetResource(ResourceType ResID);
StatusType Os_Implementation_ReleaseResource(ResourceType ResID);
StatusType Os_Implementation_Schedule(void);
StatusType Os_Implementation_WaitEvent(EventMaskType Mask);
StatusType Os_Implementation_ClearEvent(EventMaskType Mask);
StatusType Os_Implementation_SetEvent(TaskType TaskID, EventMaskType Mask);
StatusType Os_Implementation_GetEvent(TaskType TaskID, EventMaskRefType Event);
void Os_Implementation_ShutdownOS(StatusType Error);
void Os_Implementation_StartOS(AppModeType Mode);
AppModeType Os_Implementation_GetActiveApplicationMode(void);
StatusType Os_Implementation_IncrementCounter(CounterType CounterID);
StatusType Os_Implementation_GetCounterValue(CounterType CounterID, TickRefType Value);
StatusType Os_Implementation_GetElapsedValue(CounterType CounterID,
                                             TickRefType Value,
                                             TickRefType ElapsedValue);
StatusType Os_Implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info);
StatusType Os_Implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick);
StatusType Os_Implementation_SetRelAlarm(AlarmType AlarmID, TickType Increment, TickType Cycle);
StatusType Os_Implementation_SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle);
StatusType Os_Implementation_CancelAlarm(AlarmType AlarmID);
StatusType Os_Implementation_StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                   TickType Offset);
StatusType Os_Implementation_StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                   TickType Start);
StatusType Os_Implementation_StopScheduleTable(ScheduleTableType ScheduleTableID);
StatusType Os_Implementation_NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                               ScheduleTableType ScheduleTableID_To);
StatusType Os_Implementation_GetScheduleTableStatus(ScheduleTableType ScheduleTableID,
                                                    ScheduleTableStatusRefType ScheduleStatus);
StatusType Os_Implementation_DisableInterruptSource(ISRType ISRID);
StatusType Os_Implementation_EnableInterruptSource(ISRType ISRID, boolean ClearPending);
StatusType Os_Implementation_ClearPendingInterrupt(ISRType ISRID);
void Os_Implementation_EnableAllInterrupts(void);
void Os_Implementation_DisableAllInterrupts(void);
void Os_Implementation_ResumeAllInterrupts(void);
void Os_Implementation_SuspendAllInterrupts(void);
void Os_Implementation_ResumeOSInterrupts(void);
void Os_Implementation_SuspendOSInterrupts(void);
extern const Os_TargetConfig *Os_Config;
extern volatile LONG Os_Closing;
#endif
