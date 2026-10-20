/** @file OS ARTI binding; include the tool header before module declarations. */
#ifndef AUTOSAR_HOST_OS_ARTI_H
#define AUTOSAR_HOST_OS_ARTI_H
#include "Arti.h"
#include "Os_Host.h"
#include "Os_Target.h"
#include "Os_Hooks.h"
/** Task transition selectors mapped to standard ARTI task events by Os_ArtiTask. */
typedef enum {
    OS_ARTI_TASK_ACTIVATE, /**< Activation admitted. */
    OS_ARTI_TASK_START,    /**< Task begins/resumes running. */
    OS_ARTI_TASK_PREEMPT,  /**< Task preempted by another Task. */
    OS_ARTI_TASK_WAIT,     /**< Task enters event wait. */
    OS_ARTI_TASK_RELEASE,  /**< Waiting Task becomes ready. */
    OS_ARTI_TASK_TERMINATE /**< Current Task activation completes. */
} Os_ArtiTaskEventType;
/** Debugger/tool Task observation; refreshed under the OS mutex and never used to schedule. */
typedef struct {
    TaskStateType state;     /**< Current observed Task state. */
    uint8 priority;          /**< Current effective Task priority after ceiling adjustments. */
    uint32 activations;      /**< Number of admitted current/pending activations. */
    EventMaskType events;    /**< Stored Extended Task event bits. */
    EventMaskType wait_mask; /**< Current WaitEvent predicate; zero when none. */
} Os_ArtiTaskStateType;
/** Authoritative Alarm remaining ticks, cycle and activation state exposed read-only to tools. */
typedef struct {
    TickType remaining; /**< Counter ticks remaining before the active Alarm expires. */
    TickType cycle;     /**< Repeat interval in Counter ticks; zero means one-shot. */
    uint8 active;       /**< Nonzero while the Alarm is active. */
} Os_ArtiAlarmStateType;
/** Authoritative ScheduleTable state and expiry/successor positions exposed read-only to tools. */
typedef struct {
    ScheduleTableStatusType status; /**< Current SCHEDULETABLE_* state. */
    uint32 point;                   /**< Index of the next expiry point. */
    TickType remaining;             /**< Counter ticks until the next point/final delay. */
    uint32 next;     /**< Successor table index; OS_MAX_SCHEDULE_TABLES means none. */
    uint32 previous; /**< Predecessor table index; OS_MAX_SCHEDULE_TABLES means none. */
} Os_ArtiScheduleStateType;
/* Task observations are refreshed from native kernel state while its mutex is
 * owned. None of these cells is consulted by scheduling or public OS services.
 * The other pointers expose the original authoritative storage read-only. */
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP extern Os_ArtiTaskStateType Os_ArtiTasks[OS_MAX_TASKS];
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_INIT_8
#include "Os_MemMap.h"
OS_MEMMAP extern TaskType Os_ArtiRunningTask;
#define OS_STOP_SEC_VAR_INIT_8
#include "Os_MemMap.h"
struct Os_NativeStackType;
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern const struct Os_NativeStackType *Os_ArtiTaskStacks[OS_MAX_TASKS];

OS_MEMMAP extern const void *Os_ArtiTaskContexts[OS_MAX_TASKS];
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_64
#include "Os_MemMap.h"
OS_MEMMAP extern const uint64 Os_ArtiNativeContextSize;
#define OS_STOP_SEC_CONST_64
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern const Os_ArtiAlarmStateType *const Os_ArtiAlarms;

OS_MEMMAP extern const Os_ArtiScheduleStateType *const Os_ArtiScheduleTables;

OS_MEMMAP extern const TickType *const Os_ArtiCounters;

OS_MEMMAP extern const uint32 *const Os_ArtiResourceOwners;

OS_MEMMAP extern const AppModeType *const Os_ArtiAppMode;

OS_MEMMAP extern const volatile Os_Atomic32Type *const Os_ArtiOsReady;

OS_MEMMAP extern const uint32 *const Os_ArtiCurrentIsr;
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** @brief Emit a captured task transition while the native OS mutex is owned.
 * @param[in] event Task transition selector; must be a defined Os_ArtiTaskEventType.
 * @param[in] task Configured Task identifier.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; captured arguments describe the current OS
 * transition. Context: serialized OS transition/service path under its existing native critical
 * boundary; outside BSW areas. Reentrancy: single-core transition sites serialize metadata; tool
 * slot reservation supports concurrent actors. Ownership: event payloads are copied; native
 * addresses are recorded without retaining their pointed-to storage.
 */
OS_MEMMAP void Os_ArtiTask(Os_ArtiTaskEventType event, TaskType task);

/** @brief Emit paired configured Category 2 ISR events under the dispatcher mutex.
 * @param[in] isr Configured Category 2 ISR identifier.
 * @param[in] returning Zero for entry, nonzero for return.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; captured arguments describe the current OS
 * transition. Context: serialized OS transition/service path under its existing native critical
 * boundary; outside BSW areas. Reentrancy: single-core transition sites serialize metadata; tool
 * slot reservation supports concurrent actors. Ownership: event payloads are copied; native
 * addresses are recorded without retaining their pointed-to storage.
 */
OS_MEMMAP void Os_ArtiIsr(ISRType isr, sint32 returning);

/** @brief Emit a standard Hook event; AlarmCallback is not a standard OS Hook.
 * @param[in] hook Current global hook phase; NONE/AlarmCallback emit no standard hook event.
 * @param[in] error Captured ErrorHook/ShutdownHook status; ignored by other hook phases.
 * @param[in] returning Zero for entry, nonzero for return.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; arguments are already captured for the current
 * logical caller. Context: current hook actor under its existing OS synchronization. ShutdownHook
 * runs on the close controller without acquiring the ordinary port mutex; outside BSW areas.
 * Reentrancy: reentrant across native actors; logical caller scopes preserve TLS side data, and
 * atomic slot reservation/publication protects the tool buffer. Ownership: event payloads are
 * copied; native addresses are recorded as bits without retaining or dereferencing pointed-to
 * storage.
 */
OS_MEMMAP void Os_ArtiHook(Os_HookPhaseType hook, StatusType error, sint32 returning);

/** @brief Emit an application service event with an already captured payload.
 * @param[in] service Captured OS service identifier.
 * @param[in] parameter Already captured scalar service event payload.
 * @param[in] returning Zero for entry, nonzero for return.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; arguments are already captured for the current
 * logical caller. Context: service wrapper entry/return on its Task, ISR, Hook or initialized-C
 * caller, including refused calls; no native critical section is required, outside BSW areas.
 * Reentrancy: reentrant across native actors; logical caller scopes preserve TLS side data, and
 * atomic slot reservation/publication protects the tool buffer. Ownership: event payloads are
 * copied; native addresses are recorded as bits without retaining or dereferencing pointed-to
 * storage.
 */
OS_MEMMAP void Os_ArtiService(OSServiceIdType service, uint32 parameter, sint32 returning);

/** @brief Preserve the complete native pointer alongside its standard uint32 token.
 * @param[in] address Native AlarmBase result address, captured as bits without dereferencing.
 * @par Return
 * None.
 * Preconditions: ARTI initialized before actors; arguments are already captured for the current
 * logical caller. Context: successful GetAlarmBase wrapper return after the implementation has
 * returned; no native critical section is guaranteed, outside BSW areas. Reentrancy: reentrant
 * across native actors; logical caller scopes preserve TLS side data, and atomic slot
 * reservation/publication protects the tool buffer. Ownership: event payloads are copied; native
 * addresses are recorded as bits without retaining or dereferencing pointed-to storage.
 */
OS_MEMMAP void Os_ArtiAlarmBaseReturn(AlarmBaseRefType address);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/** @brief Caller classification scopes; save/restore when entering logical actors. */
/** Complete suspended logical caller classification and capture, for balanced save/restore. */
typedef struct {
    uint32 internal_depth;       /**< Nested internal-origin scopes to restore. */
    uint32 service_depth;        /**< Nested service scopes to restore. */
    StatusType access_status;    /**< Suspended caller access-guard result. */
    Arti_AddressCapture address; /**< Suspended caller lossless address/status capture. */
} Os_ArtiCallerType;
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Save and reset the current logical caller classification and capture.
 * @return Complete previous classification/capture for matching restore.
 * Preconditions: initialized actor TLS; logical caller scopes are nested.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP Os_ArtiCallerType Os_ArtiCallerSave(void);

/** Restore classification and capture of a suspended logical caller.
 * @param[in] caller Snapshot returned by the matching Os_ArtiCallerSave.
 * @par Return
 * None.
 * Preconditions: matching saved caller snapshot.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiCallerRestore(Os_ArtiCallerType caller);

/** Enter one internal service-origin scope.
 * @par Return
 * None.
 * Preconditions: internal nesting depth remains below UINT32_MAX.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiInternalEnter(void);

/** Leave the matching internal service-origin scope.
 * @par Return
 * None.
 * Preconditions: an unmatched leave asserts; internal depth must be nonzero.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiInternalLeave(void);

/** Enter a service scope and reset its captured access result.
 * @return Nonzero only for an outermost application-origin service call.
 * Preconditions: service nesting depth remains below UINT32_MAX.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP sint32 Os_ArtiServiceEnter(void);

/** Leave the matching service scope.
 * @par Return
 * None.
 * Preconditions: an unmatched leave asserts; service depth must be nonzero.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiServiceLeave(void);

/** Capture the current service access result.
 * @param[in] status Result of the service access/context guard.
 * @par Return
 * None.
 * Preconditions: current logical caller classification is established.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP void Os_ArtiAccessResult(StatusType status);

/** Test whether the last captured service access was accepted.
 * @return Nonzero for E_OK; zero for a refused access.
 * Preconditions: current logical service access result has been captured.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP sint32 Os_ArtiAccessValid(void);

/** Read the last captured service access result.
 * @return Current logical caller access status.
 * Preconditions: current logical service access result has been captured.
 * Context: current native OS actor, including its existing critical section; no blocking or BSW
 * area entry. Reentrancy: actor-local TLS; interrupt/hook nesting uses matching save/restore.
 * Ownership: copies scalar/capture state; no application object pointer is retained.
 */
OS_MEMMAP StatusType Os_ArtiAccessStatus(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
