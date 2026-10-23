/** @file Private backend operations shared across the OS sources. */
#ifndef AUTOSAR_OS_BACKEND_H
#define AUTOSAR_OS_BACKEND_H
#include "Os_Version.h"
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
    Os_HookPhaseType phase;
    OSServiceIdType service;
    Os_ErrorParametersType parameters;
    Os_ArtiCallerType arti;
} Os_ErrorContextType;
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_ErrorContextSave(Os_ErrorContextType *context);

OS_MEMMAP void Os_ErrorContextRestore(const Os_ErrorContextType *context);

OS_MEMMAP sint32 Os_HookBlocksCategory2(void);

OS_MEMMAP uint32 Os_BackendInterruptPriority(uint32 interrupt);

OS_MEMMAP sint32 Os_BackendInterruptMaySchedule(uint32 interrupt);

OS_MEMMAP void Os_BackendRequestIsrReschedule(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Backend.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Backend.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
void Os_PortDispatchNested(void);
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_PortPostInterrupt(uint32 interrupt);

OS_MEMMAP sint32 Os_BackendInputOwner(void);

OS_MEMMAP sint32 Os_BackendTaskOwner(TaskType id);

OS_MEMMAP sint32 Os_BackendServiceContext(void);

OS_MEMMAP StatusType Os_BridgeContext(void);

OS_MEMMAP sint32 Os_BridgeActorAllowed(void);

OS_MEMMAP sint32 Os_MailboxQuiescent(void);

OS_MEMMAP StatusType Os_TimeValidate(const Os_TargetConfigType *target);

OS_MEMMAP void Os_TimeInit(AppModeType mode);

OS_MEMMAP void Os_TimeAutostart(AppModeType mode);

OS_MEMMAP StatusType Os_ScheduleValidate(const Os_TargetConfigType *target);

OS_MEMMAP void Os_ScheduleAutostart(AppModeType mode);

OS_MEMMAP void Os_ScheduleIncrement(CounterType counter);

OS_MEMMAP TickType Os_TimeCounterValue(CounterType counter);

OS_MEMMAP void Os_TimeReportAction(StatusType status);

OS_MEMMAP void Os_TimeClose(void);

OS_MEMMAP sint32 Os_TimeSignalFailed(void);

OS_MEMMAP void Os_TimeTick(void);

OS_MEMMAP void vApplicationTickHook(void);

OS_MEMMAP sint32 Os_TimeBeginTick(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_IDLE_TESTS) && (OS_IDLE_TESTS == STD_ON)
void Os_TestIdleObserved(void);
#endif
#if defined(OS_ARTI_TESTS) && (OS_ARTI_TESTS == STD_ON)
void Os_ArtiTestShutdownObserved(void);
#endif
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TimeTestSeed(uint64 epoch, uint32 kernel_tick, TickType value);

OS_MEMMAP void Os_TimeTestCloseEvent(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
void Os_TimeTestBeforePending(void);
void Os_TimeTestMasked(void);
void Os_TimeTestBeforeReady(void);
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);

OS_MEMMAP void Os_MailboxInstall(void);

OS_MEMMAP void Os_MailboxClose(void);

OS_MEMMAP sint32 Os_MailboxHandlerAllowed(uint32 interrupt, uint32 (*handler)(void));

OS_MEMMAP sint32 Os_BackendHandlerAllowed(uint32 interrupt, uint32 (*handler)(void));
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
void Os_TestObserve(void);
void Os_TestInputNotified(void);
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TestInputLastTicket(uint64 ticket);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendStart(AppModeType mode);

OS_MEMMAP void Os_BackendShutdown(StatusType error) __attribute__((noreturn));

OS_MEMMAP void Os_BackendRequestShutdown(StatusType error);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendSignalStackFault(char role);

OS_MEMMAP void Os_BackendSetDispatcher(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendGuardService(void);

OS_MEMMAP sint32 Os_BackendReady(void);

OS_MEMMAP Os_HookPhaseType Os_HookContext(void);

OS_MEMMAP sint32 Os_HookServiceAllowed(OSServiceIdType service);

OS_MEMMAP StatusType Os_ServiceAccessStatus(OSServiceIdType service);

OS_MEMMAP uint32 Os_BackendCurrentInterrupt(void);

OS_MEMMAP sint32 Os_BackendStarted(void);

OS_MEMMAP sint32 Os_InterruptAllows(uint32 interrupt);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
sint32 Os_PortInterruptSourceEnabled(uint32 interrupt);
/* Native mutex held: 0 invalid, 1 applied, 2 duplicate pair with no side effect. */
sint32 Os_PortInterruptSourceControl(uint32 interrupt, uint32 operation, boolean clear);
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_InterruptDisabled(void);

OS_MEMMAP void Os_InterruptRestoreOwner(void);

OS_MEMMAP sint32 Os_HookQueryContext(void);

OS_MEMMAP sint32 Os_ErrorHookConfigured(void);

OS_MEMMAP StatusType Os_ErrorResult(OSServiceIdType service,
                                    StatusType status,
                                    const Os_ErrorParametersType *arguments);

OS_MEMMAP void Os_HookInvoke(void (*hook)(void), Os_HookPhaseType selected);

OS_MEMMAP void Os_ShutdownHookInvoke(StatusType error);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
void Os_TimeTestDispatchFinishing(void);
void Os_TimeTestDispatchUnlocked(void);
void Os_TimeTestOnWaiting(void);
extern volatile BaseType_t xInsideInterrupt;
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_BackendState(TaskType id, TaskStateRefType state);

OS_MEMMAP StatusType Os_BackendTaskId(TaskRefType id);

OS_MEMMAP AppModeType Os_BackendApplicationMode(void);

OS_MEMMAP StatusType Os_BackendActivate(TaskType id);

OS_MEMMAP StatusType Os_BackendFinish(void);

OS_MEMMAP void Os_BackendMissingEnd(void);

OS_MEMMAP StatusType Os_BackendChain(TaskType id);

OS_MEMMAP StatusType Os_BackendResource(ResourceType id, sint32 acquire);

OS_MEMMAP StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output);

OS_MEMMAP StatusType Os_BackendSchedule(void);

OS_MEMMAP StatusType Os_BackendWait(EventMaskType mask);

OS_MEMMAP StatusType Os_BackendClear(EventMaskType mask);

OS_MEMMAP sint32 Os_BackendInterruptEnabled(uint32 interrupt);

OS_MEMMAP void Os_BackendInterruptEnter(uint32 interrupt);

OS_MEMMAP void Os_BackendInterruptLeave(void);

OS_MEMMAP StatusType Os_BackendInspect(TaskType id, Os_ActivationInfoType *info);

OS_MEMMAP void Os_BackendOnSwitch(void);

OS_MEMMAP void Os_BackendBeforeSelect(void *current, void *next);

OS_MEMMAP sint32 Os_BackendTakeIsrReschedule(void);

OS_MEMMAP void Os_BackendAssert(const void *file, sint32 line);

OS_MEMMAP HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes,
                              BOOL manual,
                              BOOL initial,
                              LPCSTR name);

OS_MEMMAP HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);

OS_MEMMAP HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes,
                               SIZE_T stack,
                               LPTHREAD_START_ROUTINE start,
                               LPVOID argument,
                               DWORD flags,
                               LPDWORD id);

OS_MEMMAP HANDLE Os_PortTaskThread(TaskFunction_t code,
                                   void *argument,
                                   const StackType_t *buffer_top);

OS_MEMMAP void Os_PortResumeThread(HANDLE thread);

OS_MEMMAP DWORD Os_PortSuspendThread(HANDLE thread);

OS_MEMMAP BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(__linux__) && (defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON))
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackTestTriggerBackupFault(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif

/* Private implementations behind application-only ARTI service wrappers. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP ISRType Os_Implementation_GetISRID(void);

OS_MEMMAP StatusType Os_Implementation_ControlIdle(CoreIdType CoreID, IdleModeType IdleMode);

OS_MEMMAP boolean Os_Implementation_isOsStarted(void);

OS_MEMMAP CoreIdType Os_Implementation_GetCoreID(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* Index of the configured resource `id`, or resource_count when absent. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP uint32 Os_ResourceIndex(const Os_TargetConfigType *config, ResourceType id);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* Index of the STANDARD resource that the LINKED chain from `index` names, or
 * resource_count for a missing or cyclic link. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP uint32 Os_ResourceRoot(const Os_TargetConfigType *config, uint32 index);

OS_MEMMAP StatusType Os_Implementation_GetTaskID(TaskRefType TaskID);

OS_MEMMAP StatusType Os_Implementation_GetTaskState(TaskType TaskID, TaskStateRefType State);

OS_MEMMAP StatusType Os_Implementation_ActivateTask(TaskType TaskID);

OS_MEMMAP StatusType Os_Implementation_TerminateTask(void);

OS_MEMMAP StatusType Os_Implementation_ChainTask(TaskType TaskID);

OS_MEMMAP StatusType Os_Implementation_GetResource(ResourceType ResID);

OS_MEMMAP StatusType Os_Implementation_ReleaseResource(ResourceType ResID);

OS_MEMMAP StatusType Os_Implementation_Schedule(void);

OS_MEMMAP StatusType Os_Implementation_WaitEvent(EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_ClearEvent(EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_SetEvent(TaskType TaskID, EventMaskType Mask);

OS_MEMMAP StatusType Os_Implementation_GetEvent(TaskType TaskID, EventMaskRefType Event);

OS_MEMMAP void Os_Implementation_ShutdownOS(StatusType Error);

OS_MEMMAP void Os_Implementation_StartOS(AppModeType Mode);

OS_MEMMAP AppModeType Os_Implementation_GetActiveApplicationMode(void);

OS_MEMMAP StatusType Os_Implementation_IncrementCounter(CounterType CounterID);

OS_MEMMAP StatusType Os_Implementation_GetCounterValue(CounterType CounterID, TickRefType Value);

OS_MEMMAP StatusType Os_Implementation_GetElapsedValue(CounterType CounterID,
                                                       TickRefType Value,
                                                       TickRefType ElapsedValue);

OS_MEMMAP StatusType Os_Implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info);

OS_MEMMAP StatusType Os_Implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick);

OS_MEMMAP StatusType Os_Implementation_SetRelAlarm(AlarmType AlarmID,
                                                   TickType Increment,
                                                   TickType Cycle);

OS_MEMMAP StatusType Os_Implementation_SetAbsAlarm(AlarmType AlarmID,
                                                   TickType Start,
                                                   TickType Cycle);

OS_MEMMAP StatusType Os_Implementation_CancelAlarm(AlarmType AlarmID);

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                             TickType Offset);

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                             TickType Start);

OS_MEMMAP StatusType Os_Implementation_StopScheduleTable(ScheduleTableType ScheduleTableID);

OS_MEMMAP StatusType Os_Implementation_NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                                         ScheduleTableType ScheduleTableID_To);

OS_MEMMAP StatusType Os_Implementation_GetScheduleTableStatus(
    ScheduleTableType ScheduleTableID, ScheduleTableStatusRefType ScheduleStatus);

OS_MEMMAP StatusType Os_Implementation_DisableInterruptSource(ISRType ISRID);

OS_MEMMAP StatusType Os_Implementation_EnableInterruptSource(ISRType ISRID, boolean ClearPending);

OS_MEMMAP StatusType Os_Implementation_ClearPendingInterrupt(ISRType ISRID);

OS_MEMMAP void Os_Implementation_EnableAllInterrupts(void);

OS_MEMMAP void Os_Implementation_DisableAllInterrupts(void);

OS_MEMMAP void Os_Implementation_ResumeAllInterrupts(void);

OS_MEMMAP void Os_Implementation_SuspendAllInterrupts(void);

OS_MEMMAP void Os_Implementation_ResumeOSInterrupts(void);

OS_MEMMAP void Os_Implementation_SuspendOSInterrupts(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern const Os_TargetConfigType *Os_Config;
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP extern volatile LONG Os_Closing;
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#endif
