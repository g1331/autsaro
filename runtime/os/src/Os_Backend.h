#ifndef AUTOSAR_EPIC4_OS_BACKEND_H
#define AUTOSAR_EPIC4_OS_BACKEND_H
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include "Os_Host.h"
#include "Os_Stack.h"
#include "Os_Vector.h"
#include "Os_Mailbox.h"
#include "Os_Arti.h"
#define OS_KERNEL_YIELD_INTERRUPT 0u
#define OS_CONTROLLED_TICK_INTERRUPT 1u
typedef struct {
    Os_HookPhase phase;
    OSServiceIdType service;
    Os_ErrorParameters parameters;
    Os_ArtiCaller arti;
} Os_ErrorContext;
void Os_ErrorContextSave(Os_ErrorContext *context);
void Os_ErrorContextRestore(const Os_ErrorContext *context);
int Os_HookBlocksCategory2(void);
unsigned Os_BackendInterruptPriority(unsigned interrupt);
int Os_BackendInterruptMaySchedule(unsigned interrupt);
void Os_BackendRequestIsrReschedule(void);
void Os_PortDispatchNested(void);
void Os_PortPostInterrupt(uint32_t interrupt);
int Os_BackendInputOwner(void);
int Os_BackendTaskOwner(TaskType id);
int Os_BackendServiceContext(void);
StatusType Os_BridgeContext(void);
int Os_BridgeActorAllowed(void);
int Os_MailboxQuiescent(void);
StatusType Os_TimeValidate(const Os_TargetConfig *target);
void Os_TimeInit(AppModeType mode);
void Os_TimeAutostart(AppModeType mode);
StatusType Os_ScheduleValidate(const Os_TargetConfig *target);
void Os_ScheduleAutostart(AppModeType mode);
void Os_ScheduleIncrement(CounterType counter);
TickType Os_TimeCounterValue(CounterType counter);
void Os_TimeReportAction(StatusType status);
void Os_TimeClose(void);
int Os_TimeSignalFailed(void);
void Os_TimeTick(void);
void vApplicationTickHook(void);
int Os_TimeBeginTick(void);
#ifdef OS_IDLE_TESTS
void Os_TestIdleObserved(void);
#endif
#ifdef OS_ARTI_TESTS
void Os_ArtiTestShutdownObserved(void);
#endif
#ifdef OS_TIME_TESTS
void Os_TimeTestSeed(uint64_t epoch, uint32_t kernel_tick, TickType value);
void Os_TimeTestCloseEvent(void);
void Os_TimeTestBeforePending(void);
void Os_TimeTestMasked(void);
#endif
void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);
void Os_MailboxInstall(void);
void Os_MailboxClose(void);
int Os_MailboxHandlerAllowed(uint32_t interrupt, uint32_t (*handler)(void));
int Os_BackendHandlerAllowed(uint32_t interrupt, uint32_t (*handler)(void));
#ifdef OS_EVENT_TESTS
void Os_TestObserve(void);
void Os_TestInputNotified(void);
void Os_TestInputLastTicket(uint64_t ticket);
#endif
void Os_BackendStart(AppModeType mode);
void Os_BackendShutdown(StatusType error);
void Os_BackendRequestShutdown(StatusType error);
#ifdef __linux__
void Os_BackendSignalStackFault(char role);
void Os_BackendSetDispatcher(void);
#endif
void Os_BackendGuardService(void);
Os_HookPhase Os_HookContext(void);
int Os_HookServiceAllowed(OSServiceIdType service);
StatusType Os_ServiceAccessStatus(OSServiceIdType service);
unsigned Os_BackendCurrentInterrupt(void);
int Os_BackendStarted(void);
int Os_InterruptAllows(unsigned interrupt);
int Os_PortInterruptSourceEnabled(uint32_t interrupt);
/* Native mutex held: 0 invalid, 1 applied, 2 duplicate pair with no side effect. */
int Os_PortInterruptSourceControl(uint32_t interrupt, unsigned operation, boolean clear);
int Os_InterruptDisabled(void);
void Os_InterruptRestoreOwner(void);
int Os_HookQueryContext(void);
int Os_ErrorHookConfigured(void);
StatusType Os_ErrorResult(OSServiceIdType service, StatusType status,
                          const Os_ErrorParameters *arguments);
void Os_HookInvoke(void (*hook)(void), Os_HookPhase selected);
void Os_ShutdownHookInvoke(StatusType error);
#ifdef OS_TIME_TESTS
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
StatusType Os_BackendResource(ResourceType id, int acquire);
StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output);
StatusType Os_BackendSchedule(void);
StatusType Os_BackendWait(EventMaskType mask);
StatusType Os_BackendClear(EventMaskType mask);
int Os_BackendInterruptEnabled(unsigned interrupt);
void Os_BackendInterruptEnter(unsigned interrupt);
void Os_BackendInterruptLeave(void);
StatusType Os_BackendInspect(TaskType id, Os_ActivationInfo *info);
void Os_BackendOnSwitch(void);
void Os_BackendBeforeSelect(void *current, void *next);
int Os_BackendTakeIsrReschedule(void);
void Os_BackendAssert(const char *file, int line);
HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name);
HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);
HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                     LPVOID argument, DWORD flags, LPDWORD id);
HANDLE Os_PortTaskThread(TaskFunction_t code, void *argument, const StackType_t *buffer_top);
void Os_PortResumeThread(HANDLE thread);
DWORD Os_PortSuspendThread(HANDLE thread);
BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context);
#if defined(__linux__) && defined(OS_STACK_TESTS)
void Os_StackTestTriggerBackupFault(void);
#endif

/* Private implementations behind application-only ARTI service wrappers. */
ISRType Os_Implementation_GetISRID(void);
StatusType Os_Implementation_ControlIdle(CoreIdType CoreID, IdleModeType IdleMode);
boolean Os_Implementation_isOsStarted(void);
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
StatusType Os_Implementation_GetElapsedValue(CounterType CounterID, TickRefType Value,
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
