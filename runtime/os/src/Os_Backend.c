/** @file FreeRTOS-backed Task activation, resource, event and shutdown core of the OS. */
#include "Os_Backend.h"
#include "Os_IntegrationHooks.h"
#include <errno.h>
#include <limits.h>
#include "Os_HostIo.h"
#include <stdlib.h>
#include <setjmp.h>
#include <string.h>

#ifdef __linux__
#define OS_NATIVE_HEAP "hidden=2 controllers=2 heap=posix static=freertos "
#else
#define OS_NATIVE_HEAP "hidden=2 controllers=2 heap=windows static=freertos "
#endif

#define OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static StaticTask_t
    task_memory[OS_MAX_TASKS + 2u]; /**< Native Task TCBs; kernel port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Backend.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Backend.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static StackType_t
    task_buffers[OS_MAX_TASKS + 2u][512]; /**< Kernel-owned 512-word buffers; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static TaskHandle_t
    handles[OS_MAX_TASKS]; /**< Bootstrap-published Task handles; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
typedef struct {
    uint64 requests[OS_MAX_ACTIVATIONS];
    uint32 head;
    uint32 count;
} Os_ActivationQueueType;
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_ActivationQueueType
    activations[OS_MAX_TASKS]; /**< Activation FIFO ledger; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static EventMaskType
    task_events[OS_MAX_TASKS]; /**< Stored Task event bits; OS port lock; no Init reset. */

OS_MEMMAP static EventMaskType
    wait_masks[OS_MAX_TASKS]; /**< Task wait predicates; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static uint8
    task_waiting[OS_MAX_TASKS]; /**< Task waiting flags; OS port lock; no Init reset. */

OS_MEMMAP static uint8
    internal_held[OS_MAX_TASKS]; /**< Internal-resource ownership; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_INIT_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 running_hook_task =
    OS_MAX_TASKS; /**< RUNNING hook identity; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_INIT_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static uint64
    ready_sequences[OS_MAX_TASKS]; /**< Live rebased ready stamps; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_RESOURCE_ACTORS (OS_MAX_TASKS + OS_MAX_INTERRUPTS)
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 resource_depth[OS_RESOURCE_ACTORS]; /**< Per-actor acquisition depth; OS
                                                     port lock; no Init reset. */

OS_MEMMAP static uint32
    owned_resources[OS_RESOURCE_ACTORS]
                   [OS_MAX_RESOURCES]; /**< Per-actor resource LIFO; OS port lock; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    interrupt_ceiling; /**< Atomic ISR ceiling publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_INIT_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 current_interrupt =
    32u; /**< Active vector, 32 means none; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_INIT_32
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const uint32 *const Os_ArtiCurrentIsr =
    &current_interrupt; /**< Fixed ISR identity tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
typedef struct {
    uint32 interrupt;
    Os_ErrorContextType error;
} Os_InterruptFrameType;
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_InterruptFrameType
    interrupt_frames[OS_MAX_INTERRUPTS]; /**< Interrupted actor snapshots;
                           port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 interrupt_depth; /**< Nested ISR frame count; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static uint64 request_sequence; /**< Live rebased FIFO stamp; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
typedef struct {
    uint64 *slot;
    uint64 previous;
} Os_OrderReferenceType;
#define OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static jmp_buf
    restart_frames[OS_MAX_TASKS]; /**< Per-Task restart frames; owning Task; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    isr_reschedule; /**< Atomic pending ISR reschedule; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_BackendTakeIsrReschedule(void)
{
    return InterlockedExchange(&isr_reschedule, 0) != 0;
}

OS_MEMMAP void Os_BackendRequestIsrReschedule(void) { InterlockedExchange(&isr_reschedule, 1); }

OS_MEMMAP uint32 Os_BackendInterruptPriority(uint32 interrupt)
{
    uint32 priority = 0u;
    if ((interrupt > OS_CONTROLLED_TICK_INTERRUPT) && (interrupt < OS_MAX_INTERRUPTS) &&
        (Os_Config != NULL)) {
        priority = (Os_Config->interrupts == NULL) ? OS_MAX_ISR_PRIORITY
                                                   : Os_Config->interrupts->priorities[interrupt];
    } else if (interrupt == OS_CONTROLLED_TICK_INTERRUPT) {
        priority = (Os_Config != NULL && Os_Config->interrupts != NULL) ? 1u : OS_MAX_ISR_PRIORITY;
    } else {
        /* Kernel yield and inactive identities cannot preempt an ISR. */
    }
    return priority;
}

OS_MEMMAP sint32 Os_BackendInterruptMaySchedule(uint32 interrupt)
{
    return (interrupt < OS_MAX_INTERRUPTS) &&
           ((Os_Config->category1_isrs & ((uint32)1u << interrupt)) == 0u);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE close_event; /**< Startup-published primary close event; no Init reset. */

OS_MEMMAP static HANDLE control_thread; /**< Startup-published primary controller; no Init reset. */

OS_MEMMAP static HANDLE backup_thread; /**< Startup-published backup controller; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_StackTestBackupThread(void) { return backup_thread; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE backup_event; /**< Startup-published backup close event; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#if defined(__linux__) && (defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON))
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    backup_fault_requested; /**< Atomic backup-fault test request; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackTestTriggerBackupFault(void)
{
    if (backup_thread == NULL || InterlockedExchange(&backup_fault_requested, 1) != 0 ||
        Os_HostSetEvent(backup_event) == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE main_thread; /**< Startup-published dispatcher handle; no Init reset. */

OS_MEMMAP static HANDLE
    controller_registered[2]; /**< Startup-published registration events; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    controller_healthy[2]; /**< Atomic controller readiness flags; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE
    task_threads[OS_MAX_TASKS + 2u]; /**< Startup-published actor handles; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32
    task_thread_count; /**< Bootstrap-owned actor handle count; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_StackTestTaskThread(uint32 index) { return task_threads[index]; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type ready; /**< Atomic admission readiness; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const volatile Os_Atomic32Type *const Os_ArtiOsReady =
    &ready; /**< Fixed readiness tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"

#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP volatile Os_Atomic32Type
    Os_Closing; /**< Atomic permanent-close publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
/* Bit 0: an accepted activation transaction; bit 1: admission permanently closed.
 * Both admission and close linearize on this atomic word. Close never waits for
 * a transaction or a damaged actor holding the ordinary interrupt mutex. */
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    activation_admission; /**< Atomic transaction/close bits; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static StatusType
    shutdown_reason; /**< First-close winner reason, atomically published; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static AppModeType startup_mode; /**< Startup-only selected AppMode; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const AppModeType *const Os_ArtiAppMode =
    &startup_mode; /**< Fixed AppMode tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32
    resource_owner[OS_MAX_RESOURCES]; /**< Resource owners; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const uint32 *const Os_ArtiResourceOwners =
    resource_owner; /**< Fixed resource owners tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP Os_ArtiTaskStateType
    Os_ArtiTasks[OS_MAX_TASKS]; /**< Observed Task state; OS port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_INIT_8
#include "Os_MemMap.h"
OS_MEMMAP TaskType Os_ArtiRunningTask =
    INVALID_TASK; /**< Observed RUNNING Task; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_INIT_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP const Os_NativeStackType
    *Os_ArtiTaskStacks[OS_MAX_TASKS]; /**< Observed stack pointers; port lock; no Init reset. */

OS_MEMMAP const void
    *Os_ArtiTaskContexts[OS_MAX_TASKS]; /**< Observed context pointers; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static uint8
    arti_completed[OS_MAX_TASKS]; /**< Completion observation flags; port lock; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void arti_observe_tasks(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static uint8 trace[128]; /**< Caller-serialized trace prefix bytes; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 trace_length; /**< Caller-serialized used trace bytes; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static uint64
    trace_dropped; /**< Caller-serialized omitted-marker count; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32 resource_calls, threads, events,
    mutexes; /**< Bootstrap-owned allocation/call totals; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32
    fail_resource; /**< Startup-only resource-failure selection; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    started; /**< Atomic one-shot backend-start latch; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void task_entry(void *argument);

OS_MEMMAP static void bootstrap(void *argument);

OS_MEMMAP static void acquire_internal(uint32 index);

OS_MEMMAP static void release_internal(uint32 index);

OS_MEMMAP static uint8 internal_ceiling(uint32 index);

OS_MEMMAP static void observe_transition(void);

OS_MEMMAP static uint32 current_task_index(void);

OS_MEMMAP static void leave_running(uint32 index);

OS_MEMMAP static uint32 configured_interrupt(void)
{
    const uint32 interrupt = current_interrupt;
    configASSERT(Os_Config != NULL && Os_Config->interrupts != NULL &&
                 Os_Config->interrupts->entries != NULL && interrupt < OS_MAX_INTERRUPTS);
    const Os_IsrEntryType entry = Os_Config->interrupts->entries[interrupt];
    configASSERT(entry != NULL);
    entry();
    /* Automotive services record their reschedule request in the established
     * backend ISR state. Its normal outer exit handles Task selection. */
    return 0u;
}

OS_MEMMAP sint32 Os_BackendHandlerAllowed(uint32 interrupt, uint32 (*handler)(void))
{
    if ((Os_Config != NULL) && (Os_Config->interrupts != NULL) &&
        (Os_Config->interrupts->entries != NULL) && (interrupt < OS_MAX_INTERRUPTS) &&
        (Os_Config->interrupts->entries[interrupt] != NULL)) {
        return handler == configured_interrupt;
    }
    return 1;
}

OS_MEMMAP uint32 Os_BackendCurrentInterrupt(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    /* Host controllers run concurrently with the dispatcher and own no ISR. */
    return ((stack != NULL) && (stack->role == 'S')) ? current_interrupt : 32u;
}

OS_MEMMAP sint32 Os_BackendStarted(void) { return InterlockedCompareExchange(&started, 0, 0) != 0; }

OS_MEMMAP void Os_BackendInterruptEnter(uint32 interrupt)
{
    configASSERT(interrupt < OS_MAX_INTERRUPTS);
    configASSERT(interrupt_depth < OS_MAX_INTERRUPTS);
    interrupt_frames[interrupt_depth].interrupt = current_interrupt;
    Os_ErrorContextSave(&interrupt_frames[interrupt_depth].error);
    ++interrupt_depth;
    current_interrupt = interrupt;
    if ((interrupt > OS_CONTROLLED_TICK_INTERRUPT) &&
        (Os_BackendInterruptMaySchedule(interrupt) != 0)) {
        Os_ArtiIsr((ISRType)interrupt, 0);
    }
}

OS_MEMMAP void Os_BackendInterruptLeave(void)
{
    const Os_ErrorParametersType arguments = {.service_TerminateTask = {0u}};
    const uint32 actor = OS_MAX_TASKS + current_interrupt;
    configASSERT(interrupt_depth != 0u && current_interrupt < OS_MAX_INTERRUPTS);
    if (Os_InterruptDisabled() != 0) {
        /* Native ISR dispatch already owns the interrupt mutex and has stopped
         * the automotive Task. Restore only this logical ISR's saved pairs. */
        Os_InterruptRestoreOwner();
        if ((current_interrupt < 32u) && (current_interrupt != OS_KERNEL_YIELD_INTERRUPT) &&
            (current_interrupt != OS_CONTROLLED_TICK_INTERRUPT) &&
            ((Os_Config->category1_isrs & ((uint32)1u << current_interrupt)) == 0u)) {
            (void)Os_ErrorResult(OSServiceId_InterruptMissingEnd, E_OS_DISABLEDINT, &arguments);
        }
    }
    if (resource_depth[actor] != 0u) {
        /* Dispatch still owns the native interrupt mutex. Release actual
         * configured resources in LIFO order before reporting the ISR fault. */
        while (resource_depth[actor] != 0u) {
            const uint32 resource = owned_resources[actor][resource_depth[actor] - 1u];
            const StatusType released = Os_BackendResource(Os_Config->resources[resource].id, 0);
            configASSERT(released == E_OK);
        }
        (void)Os_ErrorResult(OSServiceId_InterruptMissingEnd, E_OS_RESOURCE, &arguments);
    }
    if ((current_interrupt > OS_CONTROLLED_TICK_INTERRUPT) &&
        (Os_BackendInterruptMaySchedule(current_interrupt) != 0)) {
        Os_ArtiIsr((ISRType)current_interrupt, 1);
    }
    --interrupt_depth;
    Os_ErrorContextRestore(&interrupt_frames[interrupt_depth].error);
    current_interrupt = interrupt_frames[interrupt_depth].interrupt;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_masked(uint32 interrupt, sint32 allowed)
{
    if (interrupt == OS_CONTROLLED_TICK_INTERRUPT && allowed == 0) {
        Os_TimeTestMasked();
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_masked(uint32 interrupt, sint32 allowed)
{
    (void)interrupt;
    (void)allowed;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_BackendInterruptEnabled(uint32 interrupt)
{
    sint32 allowed;
    /* The private kernel yield is not an application ISR; it must remain
     * available to complete native port critical-section handshakes. */
    if (interrupt == OS_KERNEL_YIELD_INTERRUPT) {
        return 1;
    }
    allowed =
        (Os_BackendInterruptPriority(interrupt) != 0u) && (Os_InterruptAllows(interrupt) != 0) &&
        (Os_PortInterruptSourceEnabled(interrupt) != 0) &&
        (((interrupt < 32u) && ((Os_Config->category1_isrs & ((uint32)1u << interrupt)) != 0u)) ||
         ((LONG)Os_BackendInterruptPriority(interrupt) >
              InterlockedCompareExchange(&interrupt_ceiling, 0, 0) &&
          Os_HookBlocksCategory2() == 0));
    observe_tick_masked(interrupt, allowed);
    return allowed;
}

OS_MEMMAP static uint32 task_slot(const StaticTask_t *memory)
{
    uint32 i = 0u;
    while ((i < Os_Config->task_count) && (&task_memory[i] != memory)) {
        ++i;
    }
    return i;
}

OS_MEMMAP static uint32 task_index(TaskType id)
{
    uint32 i;
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (Os_Config->tasks[i].id == id) {
            return i;
        }
    }
    return OS_MAX_TASKS;
}

OS_MEMMAP sint32 Os_BackendServiceContext(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    Os_BackendGuardService();
    if (stack == NULL) {
        return 0;
    }
    if (Os_HookQueryContext() != 0) {
        return 1;
    }
    return (Os_BackendReady() != 0) &&
           ((stack->role == 'T') ||
            ((stack->role == 'S') &&
             ((current_interrupt >= 32u) ||
              ((Os_Config->category1_isrs & ((uint32)1u << current_interrupt)) == 0u))));
}

OS_MEMMAP sint32 Os_BackendTaskOwner(TaskType id)
{
    uint32 index = current_task_index();
    if (index == OS_MAX_TASKS) {
        return 0;
    }
    return (Os_Config->tasks[index].id == id) ? 1 : -1;
}

OS_MEMMAP sint32 Os_BackendInputOwner(void) { return Os_BackendTaskOwner(Os_Config->input_task); }

OS_MEMMAP StatusType Os_BackendTaskId(TaskRefType id)
{
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    *id = INVALID_TASK;
    for (uint32 i = 0u; i < Os_Config->task_count; ++i) {
        if ((handles[i] == xTaskGetCurrentTaskHandle()) &&
            (eTaskGetState(handles[i]) == eRunning)) {
            *id = Os_Config->tasks[i].id;
            break;
        }
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}

OS_MEMMAP AppModeType Os_BackendApplicationMode(void) { return startup_mode; }

OS_MEMMAP void Os_BackendGuardService(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    /* Every shutdown path sets the admission close bit before readiness ends. */
    if (((InterlockedCompareExchange(&activation_admission, 0, 0) & 2) != 0) && (stack != NULL) &&
        ((stack->role == 'T') || (stack->role == 'S'))) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_ready_query(void) { Os_TimeTestBeforeReady(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_ready_query(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_BackendReady(void)
{
    sint32 serving;
    observe_ready_query();
    serving = Os_TargetReady();
    if (serving == 0) {
        /* A service that passed its entry guard before shutdown stops here instead of
         * reporting the closing OS as a calling-context or state error. */
        Os_BackendGuardService();
    }
    return serving;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* Normalize only live metadata order stamps. This never chooses a runnable task;
 * the existing kernel ready-list policy remains the only selector. */
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static Os_OrderReferenceType
    rebase_entries[OS_MAX_TASKS *
                   (OS_MAX_ACTIVATIONS + 1u)]; /**< Persistent function-owned state; existing
                                                  caller/port synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void rebase_orders(void)
{
    uint32 i;
    uint32 j;
    uint32 count = 0u;
    uint64 previous = 0u;
    uint64 rank = 0u;
    for (i = 0u; i < Os_Config->task_count; ++i) {
        Os_ActivationQueueType *queue = &activations[i];
        for (j = 0u; j < queue->count; ++j) {
            uint64 *slot = &queue->requests[(queue->head + j) % OS_MAX_ACTIVATIONS];
            uint32 position = count;
            while ((position > 0u) && (rebase_entries[position - 1u].previous > *slot)) {
                rebase_entries[position] = rebase_entries[position - 1u];
                --position;
            }
            rebase_entries[position].slot = slot;
            rebase_entries[position].previous = *slot;
            ++count;
        }
        if (ready_sequences[i] != 0u) {
            uint32 position = count;
            while ((position > 0u) &&
                   (rebase_entries[position - 1u].previous > ready_sequences[i])) {
                rebase_entries[position] = rebase_entries[position - 1u];
                --position;
            }
            rebase_entries[position].slot = &ready_sequences[i];
            rebase_entries[position].previous = ready_sequences[i];
            ++count;
        }
    }
    for (j = 0u; j < count; ++j) {
        if (rebase_entries[j].previous != previous) {
            ++rank;
            previous = rebase_entries[j].previous;
        }
        *rebase_entries[j].slot = rank;
    }
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (activations[i].count != 0u) {
            vTaskOsSetReadySequence(handles[i], ready_sequences[i]);
        }
    }
    request_sequence = rank;
}

OS_MEMMAP static void append_activation(uint32 index)
{
    Os_ActivationQueueType *queue = &activations[index];
    uint32 tail = (queue->head + queue->count) % OS_MAX_ACTIVATIONS;
    if (queue->count == 0u) {
        task_events[index] = 0u;
        task_waiting[index] = 0u;
        wait_masks[index] = 0u;
        ready_sequences[index] = request_sequence + 1u;
    }
    ++request_sequence;
    queue->requests[tail] = request_sequence;
    ++queue->count;
}

OS_MEMMAP static void leave_running(uint32 index)
{
    if ((running_hook_task == index) && (index < OS_MAX_TASKS)) {
        if ((InterlockedCompareExchange(&Os_Closing, 0, 0) == 0) && (Os_Config->hooks != NULL)) {
            Os_HookInvoke(Os_Config->hooks->post_task, OS_HOOK_POST);
        }
        running_hook_task = OS_MAX_TASKS;
    }
}

OS_MEMMAP void Os_BackendBeforeSelect(void *current, void *next)
{
    if ((current != next) && (Os_TargetReady() != 0) &&
        (InterlockedCompareExchange(&Os_Closing, 0, 0) == 0)) {
        for (uint32 i = 0u; i < Os_Config->task_count; ++i) {
            if (handles[i] == current) {
                leave_running(i);
                break;
            }
        }
    }
}

OS_MEMMAP void Os_BackendOnSwitch(void)
{
    if (Os_TargetReady() != 0) {
        arti_observe_tasks();
        uint32 i;
        for (i = 0u; i < Os_Config->task_count; ++i) {
            if (handles[i] == xTaskGetCurrentTaskHandle()) {
                acquire_internal(i);
                arti_observe_tasks();
                if ((running_hook_task != i) &&
                    (InterlockedCompareExchange(&Os_Closing, 0, 0) == 0)) {
                    running_hook_task = i;
                    if (Os_Config->hooks != NULL) {
                        Os_HookInvoke(Os_Config->hooks->pre_task, OS_HOOK_PRE);
                    }
                }
                break;
            }
        }
    }
    observe_transition();
}

OS_MEMMAP static TaskStateType native_task_state(uint32 index)
{
    /* A self-suspend inside the critical section defers the native switch.
     * Its waiting predicate is already committed even while it is current. */
    if (task_waiting[index] != 0u) {
        return WAITING;
    }
    switch (eTaskGetState(handles[index])) {
        case eRunning:
            return RUNNING;
        case eReady:
            return READY;
        case eBlocked:
            return WAITING;
        case eSuspended:
            return (task_waiting[index] != 0u) ? WAITING : SUSPENDED;
        default:
            /* Automotive Task objects are never deleted. */
            configASSERT(0);
            return SUSPENDED;
    }
}

OS_MEMMAP static void arti_observe_tasks(void)
{
    TaskStateType previous[OS_MAX_TASKS];
    uint8 completed[OS_MAX_TASKS];
    Os_ArtiRunningTask = INVALID_TASK;
    for (uint32 i = 0u; i < Os_Config->task_count; ++i) {
        previous[i] = Os_ArtiTasks[i].state;
        completed[i] = arti_completed[i];
        arti_completed[i] = 0u;
        Os_ArtiTasks[i].state = native_task_state(i);
        Os_ArtiTasks[i].priority = (uint8)uxTaskPriorityGet(handles[i]);
        Os_ArtiTasks[i].activations = activations[i].count;
        Os_ArtiTasks[i].events = task_events[i];
        Os_ArtiTasks[i].wait_mask = wait_masks[i];
        if (Os_ArtiTasks[i].state == RUNNING) {
            Os_ArtiRunningTask = Os_Config->tasks[i].id;
        }
    }
    /* Outgoing edges precede incoming edges, independent of declaration order.
     * Queued Basic activations have an atomic logical terminate/activate edge
     * even when the kernel reselects the same native thread. */
    for (uint32 i = 0u; i < Os_Config->task_count; ++i) {
        const TaskStateType state = Os_ArtiTasks[i].state;
        if (completed[i] != 0u) {
            Os_ArtiTask(OS_ARTI_TASK_TERMINATE, Os_Config->tasks[i].id);
            previous[i] = SUSPENDED;
        } else if ((previous[i] == RUNNING) && (state != RUNNING)) {
            Os_ArtiTask(state == WAITING ? OS_ARTI_TASK_WAIT : OS_ARTI_TASK_PREEMPT,
                        Os_Config->tasks[i].id);
        }
    }
    for (uint32 i = 0u; i < Os_Config->task_count; ++i) {
        const TaskStateType state = Os_ArtiTasks[i].state;
        const TaskType id = Os_Config->tasks[i].id;
        if ((previous[i] == SUSPENDED) && (state != SUSPENDED)) {
            Os_ArtiTask(OS_ARTI_TASK_ACTIVATE, id);
        } else if ((previous[i] == WAITING) && ((state == READY) || (state == RUNNING))) {
            Os_ArtiTask(OS_ARTI_TASK_RELEASE, id);
        }
        if ((state == RUNNING) && (previous[i] != RUNNING)) {
            Os_ArtiTask(OS_ARTI_TASK_START, id);
        }
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if (defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)) ||                           \
    (defined(OS_FINISH_TESTS) && (OS_FINISH_TESTS == STD_ON)) ||                                   \
    (defined(OS_RESOURCE_TESTS) && (OS_RESOURCE_TESTS == STD_ON)) ||                               \
    (defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON))
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_transition(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    if ((Os_TargetReady() != 0) && (stack != NULL) &&
        ((stack->role == 'T') || (stack->role == 'S'))) {
        Os_TestObserve();
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_transition(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static uint8 internal_ceiling(uint32 index)
{
    uint32 i;
    uint8 ceiling = 0u;
    const Os_TaskConfigType *task = &Os_Config->tasks[index];
    if (task->schedule == OS_SCHEDULE_NON) {
        for (i = 0u; i < Os_Config->task_count; ++i) {
            if (Os_Config->tasks[i].priority > ceiling) {
                ceiling = Os_Config->tasks[i].priority;
            }
        }
    } else if (task->internal_resource != 0u) {
        for (i = 0u; i < Os_Config->internal_resource_count; ++i) {
            if (Os_Config->internal_resources[i].id == task->internal_resource) {
                ceiling = Os_Config->internal_resources[i].ceiling;
                break;
            }
        }
    } else {
        /* FULL tasks without an assigned internal resource retain base priority. */
    }
    return ceiling;
}

OS_MEMMAP static void acquire_internal(uint32 index)
{
    uint8 ceiling = internal_ceiling(index);
    if ((ceiling != 0u) && (internal_held[index] == 0u)) {
        configASSERT(resource_depth[index] == 0u);
        internal_held[index] = 1u;
        if (uxTaskPriorityGet(handles[index]) < ceiling) {
            vTaskPrioritySet(handles[index], ceiling);
        }
    }
}

OS_MEMMAP static void release_internal(uint32 index)
{
    if (internal_held[index] != 0u) {
        configASSERT(resource_depth[index] == 0u);
        internal_held[index] = 0u;
        vTaskPrioritySet(handles[index], Os_Config->tasks[index].priority);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TargetTestSequence(uint64 sequence)
{
    taskENTER_CRITICAL();
    configASSERT(sequence >= request_sequence);
    request_sequence = sequence;
    taskEXIT_CRITICAL();
}

OS_MEMMAP uint32 Os_TargetTestPending(TaskType id)
{
    uint32 index = task_index(id);
    configASSERT(InterlockedCompareExchange(&Os_Closing, 0, 0) != 0);
    configASSERT(index < Os_Config->task_count);
    return activations[index].count;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
typedef struct {
    TaskFunction_t entry;
    void *argument;
    HANDLE registered, gate;
    char role;
    const StackType_t *buffer_top;
} Os_NativeTaskStartType;
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_NativeTaskStartType
    native_starts[OS_MAX_TASKS + 2u]; /**< Startup-owned actor entry records; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static uint32
    failed_operation; /**< Test recovery selector, fixture/close handoff; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackTestFailOperation(uint32 operation) { failed_operation = operation; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static sint32 operation_fails(uint32 operation)
{
    if (failed_operation == operation) {
        failed_operation = 0u;
        Os_TargetTrace('F');
        return 1;
    }
    return 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static sint32 operation_fails(uint32 operation)
{
    (void)operation;
    return 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP DWORD Os_PortSuspendThread(HANDLE thread)
{
    return operation_fails(1u) ? (DWORD)-1 : SuspendThread(thread);
}

OS_MEMMAP BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context)
{
    return operation_fails(2u) ? FALSE : GetThreadContext(thread, context);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#ifdef _WIN32
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_PortPostInterrupt(uint32 interrupt)
{
    vPortGenerateSimulatedInterruptFromWindowsThread(interrupt);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_PortPostInterrupt(uint32 interrupt) { Os_PosixPostInterrupt(interrupt); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TargetTrace(uint8 marker)
{
    if (marker == 0u) {
        if (InterlockedCompareExchange(&Os_Closing, 0, 0) != 0) {
            return;
        }
        Os_BackendShutdown(E_OS_STATE);
    }
    /* This bounded diagnostic prefix is not an automotive output queue.
     * Continued operation must not depend on room for optional trace text;
     * report omitted markers explicitly without replacing the saved prefix. */
    if (trace_length + 1u >= sizeof(trace)) {
        if (trace_dropped != UINT64_MAX) {
            ++trace_dropped;
        }
        return;
    }
    trace[trace_length++] = marker;
}

OS_MEMMAP sint32 Os_TargetReady(void)
{
    return InterlockedCompareExchange(&ready, 0, 0) != 0 &&
           InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_ARTI_TESTS) && (OS_ARTI_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_shutdown(void) { Os_ArtiTestShutdownObserved(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_shutdown(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void report_and_exit(void)
{
    sint32 output_failed = 0;
    Os_StackCheck();
    if (Os_StackHasFault() != 0) {
        shutdown_reason = E_OS_STACKFAULT;
    }
    Os_ShutdownHookInvoke(shutdown_reason);
    observe_shutdown();
    if (Os_StackReport() != 0) {
        output_failed = 1;
    }
    char storage[512];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextAppend(&text, "lifecycle=Closed state=");
    Os_HostTextAppend(&text, (shutdown_reason == E_OK) ? "Ready" : "Failed");
    Os_HostTextField(&text, " reason=", shutdown_reason);
    Os_HostTextAppend(&text, " trace=");
    Os_HostTextAppend(&text, (const char *)trace);
    Os_HostTextField(&text, " threads=", threads);
    Os_HostTextField(&text, " events=", events);
    Os_HostTextField(&text, " mutexes=", mutexes);
    Os_HostTextField(&text, " resource_calls=", resource_calls);
    Os_HostTextAppend(&text, " " OS_NATIVE_HEAP "input_closed=1 tick_closed=1");
    Os_HostTextField(&text, " time_signal_failed=", (Os_TimeSignalFailed() != 0) ? 1u : 0u);
    Os_HostTextField(&text, " trace_dropped=", trace_dropped);
    Os_HostTextByte(&text, '\n');
    if (Os_HostTextWrite(&text) == 0) {
        output_failed = 1;
    }
    ExitProcess((output_failed != 0 && shutdown_reason == E_OK) ? E_OS_STATE : shutdown_reason);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void stop_thread(HANDLE thread)
{
    CONTEXT context;
    if (Os_StackHasFault() != 0 && Os_StackThreadHasFault(GetThreadId(thread)) != 0) {
        return; /* The guard handler parked the damaged actor on its altstack. */
    }
    if (Os_HostStopActor(thread, 2000u) == 0) {
        ExitProcess(E_OS_STATE);
    }
    context.ContextFlags = CONTEXT_CONTROL;
    if (!Os_PortGetThreadContext(thread, &context)) {
        ExitProcess(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void stop_thread(HANDLE thread)
{
    CONTEXT context;
    if (SuspendThread(thread) == (DWORD)-1) {
        ExitProcess(E_OS_STATE);
    }
    context.ContextFlags = CONTEXT_CONTROL;
    if (!Os_PortGetThreadContext(thread, &context)) {
        ExitProcess(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(__linux__) && (defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON))
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_backup_close(uint32 index)
{
    if (index == 1u && InterlockedExchange(&backup_fault_requested, 0) != 0) {
        void Os_StackTestBackupFault(void);
        Os_StackTestBackupFault();
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_backup_close(uint32 index) { (void)index; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_stack_close(void)
{
    {
        void Os_StackTestBeforeClose(void);
        Os_StackTestBeforeClose();
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_stack_close(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_close(void) { Os_TestBeforeClose(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_close(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void close_native_fault_resources(void)
{
    if (Os_StackHasFault() != 0) {
        Os_TimeClose();
        Os_MailboxClose();
    }
}
OS_MEMMAP static void stop_scheduler_before_actors(void) {}
OS_MEMMAP static void stop_scheduler_after_actors(void)
{
    /* No actor may still read the port state when the controller retires it. */
    vPortEndScheduler();
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void close_native_fault_resources(void) {}
OS_MEMMAP static void stop_scheduler_before_actors(void) { vPortEndScheduler(); }
OS_MEMMAP static void stop_scheduler_after_actors(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static DWORD WINAPI control(void *argument)
{
    HANDLE event = argument == NULL ? close_event : backup_event;
    uint32 i;
    DWORD current = GetCurrentThreadId();
    DWORD waited;
    uint32 index = argument == NULL ? 0u : 1u;
    if (Os_StackRegister(argument == NULL ? 'C' : 'D')) {
        InterlockedExchange(&controller_healthy[index], 1);
    }
    if (!Os_HostSetEvent(controller_registered[index])) {
        ExitProcess(E_OS_STATE);
    }
    if (controller_healthy[index] == 0) {
        Sleep(INFINITE);
        return 0u;
    }
    do {
        waited = WaitForSingleObjectEx(event, INFINITE, TRUE);
    } while (waited == WAIT_IO_COMPLETION);
    if (waited != WAIT_OBJECT_0) {
        ExitProcess(E_OS_STATE);
    }
    /* The requester may be stopped between its wake-up and its own publication. */
    (void)InterlockedExchange(&Os_Closing, 1);
    observe_backup_close(index);
    observe_stack_close();
    observe_activation_close();
    close_native_fault_resources();
    stop_scheduler_before_actors();
    /* Stop the ISR owner first, then all execution threads. Never take its mutex. */
    if (main_thread != NULL && GetThreadId(main_thread) != current) {
        stop_thread(main_thread);
    }
    for (i = 0u; i < task_thread_count; ++i) {
        stop_thread(task_threads[i]);
    }
    if (argument != NULL && control_thread != NULL && GetThreadId(control_thread) != current) {
        stop_thread(control_thread);
    }
    if (controller_healthy[1] == 0 && backup_thread != NULL && argument == NULL) {
        stop_thread(backup_thread);
    }
    stop_scheduler_after_actors();
    if (Os_StackHasFault()) {
        ShutdownOS(E_OS_STACKFAULT);
    }
    report_and_exit();
    return 0u;
}

OS_MEMMAP void Os_BackendStackFault(char failed_role)
{
    Os_TimeClose();
    Os_MailboxClose();
    InterlockedOr(&activation_admission, 2);
    InterlockedExchange(&ready, 0);
    shutdown_reason = E_OS_STACKFAULT;
    if (failed_role == 'C') {
        InterlockedExchange(&controller_healthy[0], 0);
    } else if (failed_role == 'D') {
        InterlockedExchange(&controller_healthy[1], 0);
    }
    if (controller_healthy[0] == 0 && controller_healthy[1] == 0) {
        Os_StackFatalExit();
    }
    if (!Os_HostSetEvent((controller_healthy[0] != 0) ? close_event : backup_event)) {
        ExitProcess(E_OS_STACKFAULT);
    }
    InterlockedExchange(&Os_Closing, 1);
    Sleep(INFINITE);
    ExitProcess(E_OS_STACKFAULT);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendSignalStackFault(char failed_role)
{
    (void)InterlockedOr(&activation_admission, 2);
    (void)InterlockedExchange(&ready, 0);
    shutdown_reason = E_OS_STACKFAULT;
    if (failed_role == 'C') {
        (void)InterlockedExchange(&controller_healthy[0], 0);
    } else if (failed_role == 'D') {
        (void)InterlockedExchange(&controller_healthy[1], 0);
    }
    if (controller_healthy[0] == 0 && controller_healthy[1] == 0) {
        Os_StackFatalExit();
    }
    if (!Os_HostSetEvent(controller_healthy[0] != 0 ? close_event : backup_event)) {
        ExitProcess(E_OS_STACKFAULT);
    }
    (void)InterlockedExchange(&Os_Closing, 1);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendShutdown(StatusType error)
{
    const Os_NativeStackType *current = Os_StackCurrent();
    if (error == E_OS_STACKFAULT && Os_StackHasFault() && current != NULL &&
        (current->role == 'C' || current->role == 'D') &&
        current->thread_id != Os_Fault.thread_id) {
        report_and_exit();
    }
    Os_BackendRequestShutdown(error);
    Sleep(INFINITE);
    ExitProcess(E_OS_STATE);
}

OS_MEMMAP void Os_BackendRequestShutdown(StatusType error)
{
    Os_TimeClose();
    Os_MailboxClose();
    if ((InterlockedOr(&activation_admission, 2) & 2) == 0) {
        shutdown_reason = error;
        InterlockedExchange(&ready, 0);
        if (controller_healthy[0] == 0 && controller_healthy[1] == 0) {
            report_and_exit();
        }
        /* Wake control before publishing Os_Closing: a dispatch that stops this Task
         * after the publication never resumes it, so the wake-up would be lost. */
        if (!Os_HostSetEvent((controller_healthy[0] != 0) ? close_event : backup_event)) {
            ExitProcess(E_OS_STATE);
        }
        InterlockedExchange(&Os_Closing, 1);
    }
}

OS_MEMMAP void Os_BackendAssert(const void *file, sint32 line)
{
    (void)file;
    (void)line;
    Os_BackendShutdown(E_OS_STATE);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static sint32 fail_next(void)
{
    ++resource_calls;
    return fail_resource != 0u && resource_calls == fail_resource;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static sint32 fail_next(void)
{
    ++resource_calls;
    return 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes,
                              BOOL manual,
                              BOOL initial,
                              LPCSTR name)
{
    HANDLE result = fail_next() ? NULL : CreateEventA(attributes, manual, initial, name);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++events;
    return result;
}

OS_MEMMAP HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name)
{
    HANDLE result = fail_next() ? NULL : CreateMutexA(attributes, owner, name);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++mutexes;
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes,
                               SIZE_T stack,
                               LPTHREAD_START_ROUTINE start,
                               LPVOID argument,
                               DWORD flags,
                               LPDWORD id)
{
    HANDLE result = (attributes != NULL || fail_next())
                        ? NULL
                        : Os_HostCreateActor(stack, start, argument, flags, id, 0);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++threads;
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes,
                               SIZE_T stack,
                               LPTHREAD_START_ROUTINE start,
                               LPVOID argument,
                               DWORD flags,
                               LPDWORD id)
{
    HANDLE result =
        fail_next() ? NULL : CreateThread(attributes, stack, start, argument, flags, id);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++threads;
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static DWORD WINAPI native_task_start(void *argument)
{
    Os_NativeTaskStartType *start = argument;
    if (!Os_StackRegister(start->role)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_StackRecordBuffer(start->buffer_top);
    if (start->entry == task_entry) {
        const uint32 index = task_slot(start->argument);
        configASSERT(index < Os_Config->task_count);
        Os_ArtiTaskStacks[index] = Os_StackCurrent();
        Os_ArtiTaskContexts[index] = Os_StackSavedContext(Os_ArtiTaskStacks[index]);
    }
    if (!Os_HostSetEvent(start->registered) ||
        WaitForSingleObject(start->gate, INFINITE) != WAIT_OBJECT_0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    start->entry(start->argument);
    Os_BackendShutdown(E_OS_STATE);
    return 0u;
}

OS_MEMMAP HANDLE Os_PortTaskThread(TaskFunction_t code,
                                   void *argument,
                                   const StackType_t *buffer_top)
{
    Os_NativeTaskStartType *start;
    HANDLE thread;
    CONTEXT context;
    if (task_thread_count >= OS_MAX_TASKS + 2u) {
        Os_BackendShutdown(E_OS_STATE);
    }
    start = &native_starts[task_thread_count];
    start->entry = code;
    start->argument = argument;
    start->buffer_top = buffer_top;
    start->role = code == task_entry ? 'T' : (code == bootstrap ? 'B' : 'I');
    start->registered = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    start->gate = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    thread = Os_PortThread(NULL,
                           Os_Config->host_stack_reserve,
                           native_task_start,
                           start,
                           STACK_SIZE_PARAM_IS_A_RESERVATION,
                           NULL);
    task_threads[task_thread_count++] = thread;
    if (WaitForSingleObject(start->registered, 5000u) != WAIT_OBJECT_0 ||
        SuspendThread(thread) == (DWORD)-1) {
        Os_BackendShutdown(E_OS_STATE);
    }
    context.ContextFlags = CONTEXT_CONTROL;
    if (!GetThreadContext(thread, &context)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_StackObserve(thread, &context);
    if (!Os_HostSetEvent(start->gate)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    return thread;
}

OS_MEMMAP void Os_PortResumeThread(HANDLE thread)
{
    CONTEXT context;
    context.ContextFlags = CONTEXT_CONTROL;
    if (!GetThreadContext(thread, &context)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_StackObserve(thread, &context);
    if (operation_fails(3u) || ResumeThread(thread) == (DWORD)-1) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

OS_MEMMAP void vApplicationGetIdleTaskMemory(StaticTask_t **task,
                                             StackType_t **stack,
                                             configSTACK_DEPTH_TYPE *depth)
{
    *task = &task_memory[OS_MAX_TASKS + 1u];
    *stack = task_buffers[OS_MAX_TASKS + 1u];
    *depth = 512;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_IDLE_TESTS) && (OS_IDLE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_idle(void) { Os_TestIdleObserved(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_idle(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_STACK_TESTS) && (OS_STACK_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_idle_stack(void)
{
    void Os_StackTestIdle(void);
    Os_StackTestIdle();
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_idle_stack(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void vApplicationIdleHook(void)
{
    Os_StackCheck();
    observe_idle();
    observe_idle_stack();
    /* Give the host CPU back: the interrupt thread suspends this thread to switch
     * Tasks, so nothing waits on the idle loop. Spinning would occupy a host CPU
     * per ECU process and delay the tick handoff threads under parallel load. */
    Sleep(1u);
}

OS_MEMMAP static void task_entry(void *argument)
{
    /* The argument is the task's own control block, so the configuration stays const.
     * The index stays live across setjmp, so it is volatile (C99 7.13.2.1). */
    const volatile uint32 index = task_slot(argument);
    configASSERT(index < Os_Config->task_count);
    const Os_TaskConfigType *task = &Os_Config->tasks[index];
    /* The private backend trampoline discards completed application frames.
     * No application or generated Runnable uses setjmp/longjmp. */
    (void)setjmp(restart_frames[index]);
    (void)Os_ArtiCallerSave();
    configASSERT(activations[index].count != 0u);
    task->entry();
    /* A returned entry is an erroneous activation, not a whole-OS shutdown. */
    Os_BackendMissingEnd();
}

OS_MEMMAP static void bootstrap(void *argument)
{
    uint32 i;
    (void)argument;
    if ((Os_Config->interrupts != NULL) && (Os_Config->interrupts->entries != NULL)) {
        for (i = 2u; i < OS_MAX_INTERRUPTS; ++i) {
            if (Os_Config->interrupts->entries[i] != NULL) {
                vPortSetInterruptHandler((uint32)i, configured_interrupt);
            }
        }
    }
    Os_HookInvoke(&StartupHook, OS_HOOK_STARTUP);
    if (Os_IntegrationTimingAuthorized() == 0) {
        Os_BackendShutdown(E_OS_STATE);
        return;
    }
    taskENTER_CRITICAL();
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if ((Os_Config->tasks[i].autostart_modes & startup_mode) != 0u) {
            vTaskResume(handles[i]);
        }
    }
    Os_TimeAutostart(startup_mode);
    Os_ScheduleAutostart(startup_mode);
    InterlockedExchange(&ready, 1);
    arti_observe_tasks();
    Os_TargetTrace('R');
    observe_transition();
    taskEXIT_CRITICAL();
    vTaskSuspend(NULL);
    Os_BackendShutdown(E_OS_STATE);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void prepare_native_stack(void) { Os_StackPrepare(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void prepare_native_stack(void) { Os_StackInit(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_resource_failure(void)
{
    const char *failure = getenv("AUTOSAR_OS_FAIL_RESOURCE");
    if (failure != NULL) {
        char *end;
        unsigned long value;
        errno = 0;
        value = strtoul(failure, &end, 10);
        if (errno != 0 || *failure < '0' || *failure > '9' || *end != '\0' || value > UINT_MAX ||
            value == 0u) {
            Os_BackendShutdown(E_OS_VALUE);
        }
        fail_resource = (uint32)value;
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_resource_failure(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#ifndef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void bind_main_thread(void)
{
    if (!DuplicateHandle(GetCurrentProcess(),
                         GetCurrentThread(),
                         GetCurrentProcess(),
                         &main_thread,
                         0u,
                         FALSE,
                         DUPLICATE_SAME_ACCESS)) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void bind_main_thread(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendStart(AppModeType mode)
{
    uint32 i;
    Os_TargetTrace('I');
    read_resource_failure();
    if (InterlockedCompareExchange(&started, 1, 0) != 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    if (Os_Config == NULL || (mode != 1u && mode != 2u)) {
        Os_BackendShutdown(E_OS_VALUE);
    }
    if ((Os_Config->input_event != 0u) &&
        ((Os_Config->tasks[task_index(Os_Config->input_task)].autostart_modes & mode) == 0u)) {
        Os_BackendShutdown(E_OS_VALUE);
    }
    startup_mode = mode;
    Os_TimeInit(mode);
    prepare_native_stack();
    Os_MailboxInstall();
    bind_main_thread();
    close_event = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    backup_event = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    controller_registered[0] = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    controller_registered[1] = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    control_thread =
        Os_PortThread(NULL, 262144u, control, NULL, STACK_SIZE_PARAM_IS_A_RESERVATION, NULL);
    backup_thread =
        Os_PortThread(NULL, 262144u, control, (void *)1, STACK_SIZE_PARAM_IS_A_RESERVATION, NULL);
    if (WaitForMultipleObjects(2u, controller_registered, TRUE, 5000u) != WAIT_OBJECT_0 ||
        controller_healthy[0] == 0 || controller_healthy[1] == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    for (i = 0u; i < Os_Config->task_count; ++i) {
        Os_ArtiTasks[i].state = SUSPENDED;
        if ((Os_Config->tasks[i].autostart_modes & mode) != 0u) {
            append_activation(i);
        }
        handles[i] = xTaskCreateStatic(task_entry,
                                       (const char *)Os_Config->tasks[i].name,
                                       512,
                                       &task_memory[i],
                                       Os_Config->tasks[i].priority,
                                       task_buffers[i],
                                       &task_memory[i]);
        configASSERT(handles[i] != NULL);
        vTaskOsSetReadySequence(
            handles[i],
            (activations[i].count == 0u) ? 0u : activations[i].requests[activations[i].head]);
        vTaskSuspend(handles[i]);
    }
    configASSERT(xTaskCreateStatic(bootstrap,
                                   "Os_Bootstrap",
                                   512,
                                   NULL,
                                   31,
                                   task_buffers[OS_MAX_TASKS],
                                   &task_memory[OS_MAX_TASKS]) != NULL);
    vTaskStartScheduler();
    Os_BackendShutdown(E_OS_STATE);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_BackendSetDispatcher(void)
{
    if (!DuplicateHandle(GetCurrentProcess(),
                         GetCurrentThread(),
                         GetCurrentProcess(),
                         &main_thread,
                         0u,
                         FALSE,
                         DUPLICATE_SAME_ACCESS)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_HostSetDispatcher(main_thread);
    Os_StackInit();
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_BackendState(TaskType id, TaskStateRefType state)
{
    uint32 i;
    Os_StackCheck();
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (Os_Config->tasks[i].id == id) {
            taskENTER_CRITICAL();
            Os_BackendGuardService();
            eTaskState actual = eTaskGetState(handles[i]);
            switch (actual) {
                case eRunning:
                    *state = RUNNING;
                    break;
                case eReady:
                    *state = READY;
                    break;
                case eBlocked:
                    *state = WAITING;
                    break;
                case eSuspended:
                    *state = (task_waiting[i] != 0u) ? WAITING : SUSPENDED;
                    break;
                default:
                    taskEXIT_CRITICAL();
                    return E_OS_STATE;
            }
            taskEXIT_CRITICAL();
            Os_BackendGuardService();
            return E_OK;
        }
    }
    return E_OS_ID;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_lock(void) { Os_TestBeforeActivationLock(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_lock(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#if defined(OS_ACTIVATION_TESTS) && (OS_ACTIVATION_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_admission(uint32 phase)
{
    Os_TestAtActivationAdmission(phase);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_activation_admission(uint32 phase) { (void)phase; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_BackendActivate(TaskType id)
{
    uint32 index;
    const Os_ActivationQueueType *queue;
    const Os_NativeStackType *stack;
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    index = task_index(id);
    if (index == OS_MAX_TASKS) {
        return E_OS_ID;
    }
    queue = &activations[index];
    stack = Os_StackCurrent();
    observe_activation_lock();
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (!Os_BackendReady()) {
        taskEXIT_CRITICAL();
        return E_OS_STATE;
    }
    if (queue->count >= Os_Config->tasks[index].activation_limit) {
        taskEXIT_CRITICAL();
        return E_OS_LIMIT;
    }
    observe_activation_admission(0u);
    if (InterlockedCompareExchange(&activation_admission, 1, 0) != 0) {
        /* Closing already owns admission; no record or ready key is changed. */
        Os_BackendShutdown(E_OS_STATE);
    }
    observe_activation_admission(1u);
    if (request_sequence == UINT64_MAX) {
        rebase_orders();
    }
    configASSERT((queue->count != 0u) || (eTaskGetState(handles[index]) == eSuspended));
    append_activation(index);
    if (queue->count == 1u) {
        vTaskOsSetReadySequence(handles[index], ready_sequences[index]);
        if (stack->role == 'S') {
            BaseType_t wake = xTaskResumeFromISR(handles[index]);
            if (wake != pdFALSE) {
                InterlockedExchange(&isr_reschedule, 1);
            }
        } else {
            vTaskResume(handles[index]);
        }
    }
    arti_observe_tasks();
    observe_transition();
    InterlockedAnd(&activation_admission, ~1L);
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}

OS_MEMMAP static uint32 current_task_index(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    uint32 index;
    if ((stack == NULL) || (stack->role != 'T') || (Os_BackendReady() == 0)) {
        return OS_MAX_TASKS;
    }
    for (index = 0u; index < Os_Config->task_count; ++index) {
        if (handles[index] == xTaskGetCurrentTaskHandle()) {
            return index;
        }
    }
    return OS_MAX_TASKS;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_FINISH_TESTS) && (OS_FINISH_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_finish_boundary(uint32 phase) { Os_TestFinishBoundary(phase); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_finish_boundary(uint32 phase) { (void)phase; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static StatusType complete_activation(TaskType id, sint32 chain, sint32 returned)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    uint32 index;
    TaskHandle_t current;
    Os_ActivationQueueType *queue;
    uint32 target = OS_MAX_TASKS;
    Os_BackendGuardService();
    if ((Os_BackendReady() == 0) || (stack == NULL) || (stack->role != 'T')) {
        return E_OS_CALLEVEL;
    }
    current = xTaskGetCurrentTaskHandle();
    for (index = 0u; index < Os_Config->task_count; ++index) {
        if (handles[index] == current) {
            break;
        }
    }
    if (index == Os_Config->task_count) {
        return E_OS_CALLEVEL;
    }
    queue = &activations[index];
    if (chain != 0) {
        target = task_index(id);
        if (target == OS_MAX_TASKS) {
            return E_OS_ID;
        }
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if ((resource_depth[index] != 0u) && (returned == 0)) {
        taskEXIT_CRITICAL();
        return E_OS_RESOURCE;
    }
    if (queue->count == 0u) {
        taskEXIT_CRITICAL();
        return E_OS_STATE;
    }
    if ((chain != 0) && (target != index) &&
        (activations[target].count >= Os_Config->tasks[target].activation_limit)) {
        taskEXIT_CRITICAL();
        return E_OS_LIMIT;
    }
    if (returned != 0) {
        Os_InterruptRestoreOwner();
        /* Restore real external ownership and ceilings in LIFO order within
         * this activation transaction. A pending preemption cannot expose a
         * partly cleaned activation before it has completed normally. */
        while (resource_depth[index] != 0u) {
            const uint32 resource = owned_resources[index][resource_depth[index] - 1u];
            configASSERT(resource < Os_Config->resource_count);
            const StatusType released = Os_BackendResource(Os_Config->resources[resource].id, 0);
            configASSERT(released == E_OK);
        }
    }
    observe_finish_boundary(0u);
    if (InterlockedCompareExchange(&activation_admission, 1, 0) != 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    if ((chain != 0) && (request_sequence == UINT64_MAX)) {
        rebase_orders();
    }
    if ((chain != 0) && (target != index)) {
        configASSERT((activations[target].count != 0u) ||
                     (eTaskGetState(handles[target]) == eSuspended));
    }
    /* PostTaskHook observes the outgoing activation while still RUNNING,
     * before changing its queue/state or releasing its internal resource. */
    leave_running(index);
    release_internal(index);
    queue->requests[queue->head] = 0u;
    queue->head = (queue->head + 1u) % OS_MAX_ACTIVATIONS;
    --queue->count;
    arti_completed[index] = 1u;
    if (chain != 0) {
        append_activation(target);
        if (target != index) {
            vTaskOsSetReadySequence(handles[target],
                                    activations[target].requests[activations[target].head]);
            if (activations[target].count == 1u) {
                vTaskResume(handles[target]);
            }
        }
    }
    ready_sequences[index] = (queue->count == 0u) ? 0u : queue->requests[queue->head];
    vTaskOsSetReadySequence(current, ready_sequences[index]);
    observe_finish_boundary(1u);
    InterlockedAnd(&activation_admission, ~1L);
    if (queue->count == 0u) {
        vTaskSuspend(NULL);
    } else {
        taskYIELD();
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    longjmp(restart_frames[index], 1);
}

OS_MEMMAP StatusType Os_BackendFinish(void) { return complete_activation(0u, 0, 0); }

OS_MEMMAP StatusType Os_BackendChain(TaskType id) { return complete_activation(id, 1, 0); }

OS_MEMMAP void Os_BackendMissingEnd(void)
{
    const Os_ErrorParametersType arguments = {.service_TerminateTask = {0u}};
    Os_StackCheck();
    /* Report before leaving RUNNING, while the application's leaked resource
     * state remains observable. Normal completion then cleans this activation. */
    (void)Os_ErrorResult(OSServiceId_TaskMissingEnd, E_OS_MISSINGEND, &arguments);
    (void)complete_activation(0u, 0, 1);
    Os_BackendShutdown(E_OS_STATE);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static UBaseType_t
    saved_priorities[OS_RESOURCE_ACTORS]
                    [OS_MAX_RESOURCES]; /**< Persistent function-owned state; existing caller/port
                                           synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static LONG
    saved_interrupt_ceilings[OS_RESOURCE_ACTORS]
                            [OS_MAX_RESOURCES]; /**< Persistent function-owned state; existing
                                                   caller/port synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_BackendResource(ResourceType id, sint32 acquire)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    uint32 task;
    uint32 resource;
    uint32 depth;
    StatusType status = E_OK;
    const Os_ResourceConfigType *config;
    Os_BackendGuardService();
    task = current_task_index();
    if (task == OS_MAX_TASKS) {
        if ((Os_BackendReady() == 0) || (stack == NULL) || (stack->role != 'S') ||
            (current_interrupt == OS_KERNEL_YIELD_INTERRUPT) || (current_interrupt >= 32u) ||
            ((Os_Config->category1_isrs & ((uint32)1u << current_interrupt)) != 0u)) {
            return E_OS_CALLEVEL;
        }
        task = OS_MAX_TASKS + current_interrupt;
    }
    resource = Os_ResourceIndex(Os_Config, id);
    if (resource == Os_Config->resource_count) {
        return E_OS_ID;
    }
    /* Access is granted per configured name; a LINKED name locks and owns its
     * group root, whose ceiling and ISR sharing it repeats (Os_TargetPrepare). */
    config = &Os_Config->resources[resource];
    resource = Os_ResourceRoot(Os_Config, resource);
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    depth = resource_depth[task];
    if (acquire != 0) {
        if ((depth >= OS_MAX_RESOURCES) || (resource_owner[resource] != 0u) ||
            ((task < OS_MAX_TASKS) &&
             (((config->task_access & (1u << Os_Config->tasks[task].id)) == 0u) ||
              ((config->isr_access == 0u) &&
               (Os_Config->tasks[task].priority > config->ceiling)))) ||
            ((task >= OS_MAX_TASKS) &&
             ((config->isr_access & ((uint32)1u << current_interrupt)) == 0u))) {
            status = E_OS_ACCESS;
        } else {
            saved_priorities[task][depth] =
                (task >= OS_MAX_TASKS) ? 0u : uxTaskPriorityGet(handles[task]);
            saved_interrupt_ceilings[task][depth] =
                InterlockedCompareExchange(&interrupt_ceiling, 0, 0);
            owned_resources[task][depth] = resource;
            resource_depth[task] = depth + 1u;
            resource_owner[resource] = (uint32)task + 1u;
            if (task < OS_MAX_TASKS) {
                const UBaseType_t ceiling =
                    config->isr_access != 0u ? OS_MAX_PRIORITY + 1u : config->ceiling;
                if (ceiling > saved_priorities[task][depth]) {
                    vTaskPrioritySet(handles[task], ceiling);
                }
            }
            if (config->isr_access != 0u) {
                const LONG previous = saved_interrupt_ceilings[task][depth];
                const LONG ceiling = (LONG)config->ceiling;
                InterlockedExchange(&interrupt_ceiling, ceiling > previous ? ceiling : previous);
            }
        }
    } else if ((depth == 0u) || (depth > OS_MAX_RESOURCES) ||
               (owned_resources[task][depth - 1u] != resource) ||
               (resource_owner[resource] != (uint32)task + 1u)) {
        status = E_OS_NOFUNC;
    } else {
        resource_depth[task] = depth - 1u;
        resource_owner[resource] = 0u;
        InterlockedExchange(&interrupt_ceiling, saved_interrupt_ceilings[task][depth - 1u]);
        if (task < OS_MAX_TASKS) {
            vTaskPrioritySet(handles[task], saved_priorities[task][depth - 1u]);
        }
    }
    arti_observe_tasks();
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}

OS_MEMMAP StatusType Os_BackendSchedule(void)
{
    uint32 index = current_task_index();
    if (index == OS_MAX_TASKS) {
        return E_OS_CALLEVEL;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (resource_depth[index] != 0u) {
        taskEXIT_CRITICAL();
        return E_OS_RESOURCE;
    }
    if (internal_held[index] != 0u) {
        release_internal(index);
        taskYIELD();
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    acquire_internal(index);
    return E_OK;
}

OS_MEMMAP StatusType Os_BackendWait(EventMaskType mask)
{
    uint32 index = current_task_index();
    if (index == OS_MAX_TASKS) {
        return E_OS_CALLEVEL;
    }
    if (Os_Config->tasks[index].kind != OS_EXTENDED_TASK) {
        return E_OS_ACCESS;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (resource_depth[index] != 0u) {
        taskEXIT_CRITICAL();
        return E_OS_RESOURCE;
    }
    if ((task_events[index] & mask) == 0u) {
        leave_running(index);
        wait_masks[index] = mask;
        task_waiting[index] = 1u;
        release_internal(index);
        ready_sequences[index] = 0u;
        vTaskOsSetReadySequence(handles[index], 0u);
        /* Native suspended-list membership plus the live activation/wait
         * predicate represents automotive WAITING. The ISR cannot interleave
         * before this complete transaction releases the port critical section. */
        vTaskSuspend(NULL);
        /* Publish this edge before the completion receipt admits the next
         * epoch; wake-up and selection may otherwise collapse the wait. */
        arti_observe_tasks();
        Os_TimeOnWaiting(Os_Config->tasks[index].id, task_events[index], mask);
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    acquire_internal(index);
    configASSERT(task_waiting[index] == 0u);
    return E_OK;
}

OS_MEMMAP StatusType Os_BackendClear(EventMaskType mask)
{
    uint32 index = current_task_index();
    if (index == OS_MAX_TASKS) {
        return E_OS_CALLEVEL;
    }
    if (Os_Config->tasks[index].kind != OS_EXTENDED_TASK) {
        return E_OS_ACCESS;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    task_events[index] &= ~mask;
    arti_observe_tasks();
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}

OS_MEMMAP StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output)
{
    uint32 index;
    StatusType status = E_OK;
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    index = task_index(id);
    if (index == OS_MAX_TASKS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (Os_Config->tasks[index].kind != OS_EXTENDED_TASK) {
        status = E_OS_ACCESS;
    } else if (activations[index].count == 0u) {
        status = E_OS_STATE;
    } else if (output != NULL) {
        *output = task_events[index];
    } else {
        task_events[index] |= mask;
        if ((task_waiting[index] != 0u) && ((task_events[index] & wait_masks[index]) != 0u)) {
            const Os_NativeStackType *stack = Os_StackCurrent();
            if (request_sequence == UINT64_MAX) {
                rebase_orders();
            }
            ++request_sequence;
            ready_sequences[index] = request_sequence;
            task_waiting[index] = 0u;
            wait_masks[index] = 0u;
            vTaskOsSetReadySequence(handles[index], ready_sequences[index]);
            if (stack->role == 'S') {
                if (xTaskResumeFromISR(handles[index]) != pdFALSE) {
                    InterlockedExchange(&isr_reschedule, 1);
                }
            } else {
                vTaskResume(handles[index]);
            }
        }
    }
    arti_observe_tasks();
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}

OS_MEMMAP StatusType Os_BackendInspect(TaskType id, Os_ActivationInfoType *info)
{
    uint32 index;
    uint32 j;
    Os_ActivationInfoType snapshot;
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    index = task_index(id);
    if (index == OS_MAX_TASKS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    (void)memset(&snapshot, 0, sizeof(snapshot));
    if (Os_BackendState(id, &snapshot.state) != E_OK) {
        taskEXIT_CRITICAL();
        return E_OS_STATE;
    }
    snapshot.count = activations[index].count;
    for (j = 0u; j < snapshot.count; ++j) {
        snapshot.requests[j] =
            activations[index].requests[(activations[index].head + j) % OS_MAX_ACTIVATIONS];
    }
    snapshot.kernel_sequence = ullTaskOsReadySequence(handles[index]);
    snapshot.events = task_events[index];
    snapshot.effective_priority = (uint8)uxTaskPriorityGet(handles[index]);
    snapshot.resource_count = resource_depth[index];
    snapshot.internal_held = internal_held[index];
    snapshot.internal_ceiling = internal_ceiling(index);
    snapshot.waiting = task_waiting[index];
    snapshot.wait_mask = wait_masks[index];
    for (j = 0u; j < snapshot.resource_count; ++j) {
        uint32 resource = owned_resources[index][j];
        snapshot.resources[j] = (resource < Os_Config->resource_count)
                                    ? Os_Config->resources[resource].id
                                    : RES_SCHEDULER;
    }
    *info = snapshot;
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
