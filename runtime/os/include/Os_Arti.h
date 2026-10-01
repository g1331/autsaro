/** @file OS ARTI binding; include the tool header before module declarations. */
#ifndef AUTOSAR_HOST_OS_ARTI_H
#define AUTOSAR_HOST_OS_ARTI_H
#include "Arti.h"
#include "Os_Target.h"
#include "Os_Hooks.h"
typedef enum {
    OS_ARTI_TASK_ACTIVATE,
    OS_ARTI_TASK_START,
    OS_ARTI_TASK_PREEMPT,
    OS_ARTI_TASK_WAIT,
    OS_ARTI_TASK_RELEASE,
    OS_ARTI_TASK_TERMINATE
} Os_ArtiTaskEvent;
typedef struct {
    TaskStateType state;
    uint8_t priority;
    unsigned activations;
    EventMaskType events;
    EventMaskType wait_mask;
} Os_ArtiTaskState;
typedef struct {
    TickType remaining;
    TickType cycle;
    uint8_t active;
} Os_ArtiAlarmState;
typedef struct {
    ScheduleTableStatusType status;
    size_t point;
    TickType remaining;
    size_t next;
    size_t previous;
} Os_ArtiScheduleState;
/* Task observations are refreshed from native kernel state while its mutex is
 * owned. None of these cells is consulted by scheduling or public OS services.
 * The other pointers expose the original authoritative storage read-only. */
extern Os_ArtiTaskState Os_ArtiTasks[OS_MAX_TASKS];
extern TaskType Os_ArtiRunningTask;
struct Os_NativeStack;
extern const struct Os_NativeStack *Os_ArtiTaskStacks[OS_MAX_TASKS];
extern const void *Os_ArtiTaskContexts[OS_MAX_TASKS];
extern const size_t Os_ArtiNativeContextSize;
extern const Os_ArtiAlarmState *const Os_ArtiAlarms;
extern const Os_ArtiScheduleState *const Os_ArtiScheduleTables;
extern const TickType *const Os_ArtiCounters;
extern const unsigned *const Os_ArtiResourceOwners;
extern const AppModeType *const Os_ArtiAppMode;
#ifdef _WIN32
extern const volatile long *const Os_ArtiOsReady;
#else
extern const volatile int32_t *const Os_ArtiOsReady;
#endif
extern const unsigned *const Os_ArtiCurrentIsr;
/** @brief Emit a captured task transition while the native OS mutex is owned. */
void Os_ArtiTask(Os_ArtiTaskEvent event, TaskType task);
/** @brief Emit paired configured Category 2 ISR events under the dispatcher mutex. */
void Os_ArtiIsr(ISRType isr, int returning);
/** @brief Emit a standard Hook event; AlarmCallback is not a standard OS Hook. */
void Os_ArtiHook(Os_HookPhase hook, StatusType error, int returning);
/** @brief Emit an application service event with an already captured payload. */
void Os_ArtiService(OSServiceIdType service, uint32_t parameter, int returning);
/** @brief Preserve the complete native pointer alongside its standard uint32 token. */
void Os_ArtiAlarmBaseReturn(AlarmBaseRefType address);
/** @brief Caller classification scopes; save/restore when entering logical actors. */
typedef struct {
    unsigned internal_depth;
    unsigned service_depth;
    StatusType access_status;
    Arti_AddressCapture address;
} Os_ArtiCaller;
Os_ArtiCaller Os_ArtiCallerSave(void);
void Os_ArtiCallerRestore(Os_ArtiCaller caller);
void Os_ArtiInternalEnter(void);
void Os_ArtiInternalLeave(void);
int Os_ArtiServiceEnter(void);
void Os_ArtiServiceLeave(void);
void Os_ArtiAccessResult(StatusType status);
int Os_ArtiAccessValid(void);
StatusType Os_ArtiAccessStatus(void);
#endif
