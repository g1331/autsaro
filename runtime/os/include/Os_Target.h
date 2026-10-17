/** @file Static target configuration and host-process preparation for the OS. */
#ifndef AUTOSAR_OS_TARGET_H
#define AUTOSAR_OS_TARGET_H
#include "Os.h"
#include "Os_Time.h"
#include <stddef.h>
#define OS_MAX_TASKS    16u /**< Maximum configured automotive Tasks; also bounds Task ID masks. */
#define OS_MAX_PRIORITY 30u /**< Highest application Task base priority. */
#define OS_MAX_ACTIVATIONS 32u /**< Maximum queued activations of one Basic Task. */
#define OS_BASIC_TASK      0u  /**< Basic Task kind; no event wait. */
#define OS_EXTENDED_TASK   1u  /**< Extended Task kind; exactly one activation, with event wait. */
#define OS_MAX_RESOURCES   8u /**< Maximum external resource descriptors and held-resource depth. */
#define OS_MAX_INTERNAL_RESOURCES 2u  /**< Maximum internal resource descriptors. */
#define OS_MAX_INTERRUPTS         32u /**< Native vector slots including reserved kernel vectors. */
#define OS_MAX_ISR_PRIORITY                                                                        \
    31u                     /**< Highest application ISR priority; larger values preempt smaller. */
#define OS_SCHEDULE_FULL 0u /**< Fully preemptable Task scheduling selection. */
#define OS_SCHEDULE_NON  1u /**< Nonpreemptable Task scheduling selection. */
/** Static Category 2 body taking no arguments and returning no value, called by the native
 * dispatcher. */
typedef void (*Os_IsrEntry)(void);
/** Process-lifetime ISR priority and optional entry bindings validated before StartOS. */
typedef struct {
    /* Higher values preempt lower values. Zero leaves an application source
     * unconfigured. Kernel yield/tick entries are reserved and must be zero. */
    uint8 priorities[OS_MAX_INTERRUPTS]; /**< ISR preemption priorities; zero unconfigured, larger
                                            values preempt smaller. */
    /* Optional array of exactly OS_MAX_INTERRUPTS static Category 2 bodies.
     * Use OS_ISR_ENTRY(name). NULL retains manual native handler binding.
     * Slots 0/1, Category 1 and the configured input mailbox must be NULL. */
    const Os_IsrEntry *entries; /**< Optional OS_MAX_INTERRUPTS static Category 2 entry array;
                                   reserved slots NULL. */
} Os_IsrConfig;
/** Static resource access/ceiling and optional LINKED-resource alias configuration. */
typedef struct {
    ResourceType id; /**< Configured resource identifier below OS_MAX_RESOURCES. */
    /* Task priority for Task-only resources; ISR priority when isr_access is
     * nonzero. An ISR-shared resource also excludes all Task preemption. */
    uint8 ceiling;      /**< Task priority, or ISR priority when isr_access is nonzero. */
    uint16 task_access; /**< Mask of Task identifiers allowed to acquire this resource. */
    uint32
        isr_access; /**< Mask of Category 2 ISR vectors allowed to acquire; zero for Task-only. */
    /* OsResourceProperty LINKED: nonzero makes this resource another name for
     * `link` (itself possibly LINKED). The group shares one owner and the root's
     * ceiling and ISR sharing; the ceiling covers the accessors of every member. */
    uint8 linked; /**< Nonzero aliases the resource named by link; zero selects a root resource. */
    ResourceType link; /**< Linked resource identifier when linked is nonzero; otherwise ignored. */
} Os_ResourceConfig;
/** Static internal-resource identity and Task-priority ceiling. */
typedef struct {
    uint8 id;      /**< Internal resource ID, 1 through OS_MAX_INTERNAL_RESOURCES. */
    uint8 ceiling; /**< Task-priority ceiling, 1 through OS_MAX_PRIORITY. */
} Os_InternalResourceConfig;
/** Process-lifetime Task descriptor; entry/name and all static settings survive OS execution. */
typedef struct {
    TaskType id;             /**< Unique Task identifier below OS_MAX_TASKS. */
    const char *name;        /**< Non-NULL process-lifetime native Task name. */
    void (*entry)(void);     /**< Non-NULL static Task body, normally OS_TASK_ENTRY(name). */
    uint8 priority;          /**< Base Task priority, 1 through OS_MAX_PRIORITY. */
    uint8 autostart_modes;   /**< Mode bit mask: zero none, bit 0 mode 1, bit 1 mode 2. */
    uint8 kind;              /**< OS_BASIC_TASK or OS_EXTENDED_TASK. */
    uint8 activation_limit;  /**< 1 through OS_MAX_ACTIVATIONS; Extended Tasks require exactly 1. */
    uint8 schedule;          /**< OS_SCHEDULE_FULL or OS_SCHEDULE_NON. */
    uint8 internal_resource; /**< Internal resource ID; zero means no explicit internal resource. */
} Os_TaskConfig;
/** Caller-owned stable Task/activation/resource snapshot copied under the native OS critical
 * section. */
typedef struct {
    TaskStateType state; /**< Task state observed at the snapshot boundary. */
    uint32 count;        /**< Number of live FIFO activation requests. */
    /* Live FIFO order stamps; atomic rebasing preserves order, not lifetime IDs. */
    uint64 requests[OS_MAX_ACTIVATIONS]; /**< Live request ordering stamps; only the first count
                                            entries are meaningful. */
    uint64 kernel_sequence;              /**< Current native ready-list ordering stamp. */
    EventMaskType events;                /**< Stored Extended Task event bits. */
    uint8 effective_priority; /**< Current priority including active resource ceilings. */
    uint32 resource_count;    /**< Number of external resources currently held. */
    ResourceType resources[OS_MAX_RESOURCES]; /**< Held external resource identifiers in acquisition
                                                 order. */
    uint8 internal_held;     /**< Nonzero while the Task internal resource is held. */
    uint8 internal_ceiling;  /**< Active internal-resource priority ceiling. */
    uint8 waiting;           /**< Nonzero while the current activation is waiting for events. */
    EventMaskType wait_mask; /**< Current WaitEvent predicate. */
} Os_ActivationInfo;
/** One process-lifetime native target graph, validated and retained by Os_TargetPrepare. */
typedef struct {
    const Os_TaskConfig *tasks; /**< Process-lifetime array of task_count immutable descriptors. */
    size_t task_count;          /**< Configured Tasks, 1 through OS_MAX_TASKS. */
    uint32 host_stack_reserve;  /**< Native stack reserve bytes: 64 KiB through 16 MiB, in 64 KiB
                                   steps. */
    const Os_ResourceConfig
        *resources; /**< Static resource_count descriptors; may be NULL when count is zero. */
    size_t resource_count; /**< Configured resources, zero through OS_MAX_RESOURCES. */
    const Os_InternalResourceConfig
        *internal_resources; /**< Static internal_resource_count descriptors; NULL allowed for zero
                                count. */
    size_t internal_resource_count; /**< Configured internal resources, zero through
                                       OS_MAX_INTERNAL_RESOURCES. */
    uint32 category1_isrs; /**< Category 1 vector mask; kernel vectors 0 and 1 must be clear. */
    TaskType input_task;   /**< Mailbox owner Task; meaningful when input_event is nonzero. */
    EventMaskType
        input_event; /**< Mailbox notification bits; zero disables the native input bridge. */
    const Os_TimeConfig
        *time; /**< Optional static Counter/Alarm/ScheduleTable configuration; NULL disables it. */
    const Os_HookConfig
        *hooks; /**< Optional Error/PreTask/PostTask callbacks; NULL disables those callbacks. */
    /* NULL selects a flat, nonpreempting application ISR configuration. */
    const Os_IsrConfig *interrupts; /**< Optional static priority/entry graph; NULL selects flat
                                       application ISR handling. */
    /* OsUseResScheduler: nonzero reserves RES_SCHEDULER for the scheduler lock
     * (STANDARD, all Tasks, highest Task ceiling); zero lets a configured
     * RES_SCHEDULER behave as any other resource (SWS_Os_00854). */
    uint8 use_res_scheduler; /**< Nonzero reserves RES_SCHEDULER as the global Task scheduler
                                resource. */
} Os_TargetConfig;
/** Prepare one process-local target before StartOS.
 * @param[in] config Static configuration retained for the lifetime of the process.
 * @return E_OK or a standard validation error; no threads created on rejection.
 * Preconditions: first preparation, no actors running; invalid descriptors are rejected without
 * publication. Context: initialized native startup thread, before StartOS and outside exclusive
 * areas. Reentrancy: non-reentrant; the first successful preparation is retained. Ownership:
 * borrows config and all referenced arrays/strings/callbacks for the entire process lifetime.
 */
StatusType Os_TargetPrepare(const Os_TargetConfig *config);
/** Bounded diagnostic prefix; does not perform host I/O.
 * Full capacity preserves the prefix and counts omitted markers in the closing
 * lifecycle report. Optional diagnostic text cannot terminate normal execution.
 * @param[in] marker Nonzero character to append.
 * @par Return
 * None.
 * Preconditions: registered OS actor; NUL requests shutdown unless already closing.
 * Context: serialized OS actor/hook context; no host I/O and no BSW exclusive area.
 * Reentrancy: non-reentrant; callers serialize the bounded trace prefix.
 * Ownership: copies marker into internal trace; no caller storage retained.
 */
void Os_TargetTrace(char marker);
/** Whether target services may accept input/tick (Ready and not closing).
 * @return Nonzero only after successful StartupHook.
 * Preconditions: initialized C environment; returns zero before readiness or after close.
 * Context: any native or automotive actor; observation only.
 * Reentrancy: reentrant; readiness and close are atomic publications.
 * Ownership: returns a scalar snapshot; no storage transferred.
 */
sint32 Os_TargetReady(void);
/** Inspect a stable request/state snapshot in Task or Category 2 ISR context.
 * @param[in] id Configured Task identifier.
 * @param[out] info Caller-owned output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS, E_OS_CALLEVEL or E_OS_STATE.
 * Preconditions: configured running OS and non-NULL info; refusal leaves output unchanged.
 * Context: Task or Category 2 ISR, outside BSW exclusive areas.
 * Reentrancy: reentrant; snapshots are taken under the existing OS critical section.
 * Ownership: caller owns info, borrowed until return only.
 */
StatusType Os_TargetInspectActivation(TaskType id, Os_ActivationInfo *info);
#if defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)
/** Observe a committed native scheduling transition in test builds.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestObserve(void);
/** Set the activation ordering stamp for saturation tests.
 * @param[in] sequence New stamp, at least the current request sequence.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TargetTestSequence(uint64 sequence);
/** Observe admission before the native activation lock in tests.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestBeforeActivationLock(void);
/** Observe a selected activation admission boundary in tests.
 * @param[in] point Implementation-defined boundary selector used by the fixture.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestAtActivationAdmission(uint32 point);
/** Observe the shutdown boundary before permanent close in tests.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestBeforeClose(void);
/** Read a configured Task pending activation count in tests.
 * @param[in] id Configured Task identifier; must exist.
 * @return Current queued activation count.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
uint32 Os_TargetTestPending(TaskType id);
#endif
#if defined(OS_FINISH_TESTS) && (OS_FINISH_TESTS == STD_ON)
/** Observe a committed native scheduling transition in test builds.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestObserve(void);
/** Observe a selected Task finish boundary in tests.
 * @param[in] point Implementation-defined boundary selector used by the fixture.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestFinishBoundary(uint32 point);
#endif
#if defined(OS_RESOURCE_TESTS) && (OS_RESOURCE_TESTS == STD_ON)
/** Observe a committed native scheduling transition in test builds.
 * @par Return
 * None.
 * Preconditions: matching native test feature enabled; only the linked fixture may use this seam.
 * Context: native test boundary specified above; preserve the existing OS lock state.
 * Reentrancy: fixture controls serialization; no production caller.
 * Ownership: no pointer is retained; all arguments/results are scalar.
 */
void Os_TestObserve(void);
#endif
#endif
