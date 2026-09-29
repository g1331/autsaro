#ifndef AUTOSAR_EPIC4_OS_BACKEND_H
#define AUTOSAR_EPIC4_OS_BACKEND_H
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include "Os_Windows.h"
#include "Os_Stack.h"
#include "Os_Mailbox.h"
int Os_BackendInputOwner(void);
int Os_BackendTaskOwner(TaskType id);
int Os_BackendServiceContext(void);
StatusType Os_BridgeContext(void);
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
#ifdef OS_TIME_TESTS
void Os_TimeTestSeed(uint64_t epoch, uint32_t kernel_tick, TickType value);
void Os_TimeTestCloseEvent(void);
void Os_TimeTestBeforePending(void);
#endif
void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);
void Os_MailboxInstall(void);
void Os_MailboxClose(void);
int Os_MailboxHandlerAllowed(uint32_t interrupt, uint32_t (*handler)(void));
#ifdef OS_EVENT_TESTS
void Os_TestObserve(void);
void Os_TestInputNotified(void);
void Os_TestInputLastTicket(uint64_t ticket);
#endif
void Os_BackendStart(AppModeType mode);
void Os_BackendShutdown(StatusType error);
void Os_BackendRequestShutdown(StatusType error);
void Os_BackendGuardService(void);
Os_HookPhase Os_HookContext(void);
int Os_HookServiceAllowed(OSServiceIdType service);
int Os_HookQueryContext(void);
int Os_ErrorHookConfigured(void);
StatusType Os_ErrorResult(OSServiceIdType service, StatusType status,
                          const Os_ErrorParameters *arguments);
void Os_HookInvoke(void (*hook)(void), Os_HookPhase selected);
StatusType Os_BackendState(TaskType id, TaskStateRefType state);
StatusType Os_BackendTaskId(TaskRefType id);
AppModeType Os_BackendApplicationMode(void);
StatusType Os_BackendActivate(TaskType id);
StatusType Os_BackendFinish(void);
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
extern const Os_TargetConfig *Os_Config;
extern volatile LONG Os_Closing;
#endif
