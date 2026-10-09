#include "Os_Backend.h"
#include "Os_IntegrationHooks.h"
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <setjmp.h>
#include <string.h>

static StaticTask_t task_memory[OS_MAX_TASKS + 2u];
static StackType_t task_buffers[OS_MAX_TASKS + 2u][512];
static TaskHandle_t handles[OS_MAX_TASKS];
typedef struct {
    uint64_t requests[OS_MAX_ACTIVATIONS];
    unsigned head;
    unsigned count;
} ActivationQueue;
static ActivationQueue activations[OS_MAX_TASKS];
static EventMaskType task_events[OS_MAX_TASKS];
static EventMaskType wait_masks[OS_MAX_TASKS];
static uint8_t task_waiting[OS_MAX_TASKS];
static uint8_t internal_held[OS_MAX_TASKS];
static size_t running_hook_task = OS_MAX_TASKS;
static uint64_t ready_sequences[OS_MAX_TASKS];
#define OS_RESOURCE_ACTORS (OS_MAX_TASKS + OS_MAX_INTERRUPTS)
static unsigned resource_depth[OS_RESOURCE_ACTORS];
static size_t owned_resources[OS_RESOURCE_ACTORS][OS_MAX_RESOURCES];
static volatile Os_Atomic32 interrupt_ceiling;
static unsigned current_interrupt = 32u;
const unsigned *const Os_ArtiCurrentIsr = &current_interrupt;
typedef struct {
    unsigned interrupt;
    Os_ErrorContext error;
} InterruptFrame;
static InterruptFrame interrupt_frames[OS_MAX_INTERRUPTS];
static unsigned interrupt_depth;
static uint64_t request_sequence;
typedef struct {
    uint64_t *slot;
    uint64_t previous;
} OrderReference;
static jmp_buf restart_frames[OS_MAX_TASKS];
static volatile Os_Atomic32 isr_reschedule;
int Os_BackendTakeIsrReschedule(void) { return InterlockedExchange(&isr_reschedule, 0) != 0; }
void Os_BackendRequestIsrReschedule(void) { InterlockedExchange(&isr_reschedule, 1); }
unsigned Os_BackendInterruptPriority(unsigned interrupt) {
    unsigned priority = 0u;
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
int Os_BackendInterruptMaySchedule(unsigned interrupt) {
    return (interrupt < OS_MAX_INTERRUPTS) &&
           ((Os_Config->category1_isrs & ((uint32_t)1u << interrupt)) == 0u);
}
static HANDLE close_event;
static HANDLE control_thread;
static HANDLE backup_thread;
#ifdef OS_STACK_TESTS
HANDLE Os_StackTestBackupThread(void) { return backup_thread; }
#endif
static HANDLE backup_event;
#if defined(__linux__) && defined(OS_STACK_TESTS)
static volatile Os_Atomic32 backup_fault_requested;
void Os_StackTestTriggerBackupFault(void) {
    if (backup_thread == NULL || InterlockedExchange(&backup_fault_requested, 1) != 0 ||
        Os_HostSetEvent(backup_event) == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#endif
static HANDLE main_thread;
static HANDLE controller_registered[2];
static volatile Os_Atomic32 controller_healthy[2];
static HANDLE task_threads[OS_MAX_TASKS + 2u];
static unsigned task_thread_count;
#ifdef OS_STACK_TESTS
HANDLE Os_StackTestTaskThread(unsigned index) { return task_threads[index]; }
#endif
static volatile Os_Atomic32 ready;
#ifdef _WIN32
const volatile long *const Os_ArtiOsReady = &ready;
#else
const volatile int32_t *const Os_ArtiOsReady = &ready;
#endif
volatile Os_Atomic32 Os_Closing;
/* Bit 0: an accepted activation transaction; bit 1: admission permanently closed.
 * Both admission and close linearize on this atomic word. Close never waits for
 * a transaction or a damaged actor holding the ordinary interrupt mutex. */
static volatile Os_Atomic32 activation_admission;
static StatusType shutdown_reason;
static AppModeType startup_mode;
const AppModeType *const Os_ArtiAppMode = &startup_mode;
static unsigned resource_owner[OS_MAX_RESOURCES];
const unsigned *const Os_ArtiResourceOwners = resource_owner;
Os_ArtiTaskState Os_ArtiTasks[OS_MAX_TASKS];
TaskType Os_ArtiRunningTask = INVALID_TASK;
const Os_NativeStack *Os_ArtiTaskStacks[OS_MAX_TASKS];
const void *Os_ArtiTaskContexts[OS_MAX_TASKS];
static uint8_t arti_completed[OS_MAX_TASKS];
static void arti_observe_tasks(void);
static char trace[128];
static size_t trace_length;
static uint64_t trace_dropped;
static unsigned resource_calls, threads, events, mutexes;
#ifdef OS_HOST_FAILURE_TESTS
static unsigned fail_resource;
#endif
static volatile Os_Atomic32 started;
static void task_entry(void *argument);
static void bootstrap(void *argument);
static void acquire_internal(size_t index);
static void release_internal(size_t index);
static uint8_t internal_ceiling(size_t index);
static void observe_transition(void);
static size_t current_task_index(void);
static void leave_running(size_t index);
static uint32_t configured_interrupt(void) {
    const unsigned interrupt = current_interrupt;
    configASSERT(Os_Config != NULL && Os_Config->interrupts != NULL &&
                 Os_Config->interrupts->entries != NULL && interrupt < OS_MAX_INTERRUPTS);
    const Os_IsrEntry entry = Os_Config->interrupts->entries[interrupt];
    configASSERT(entry != NULL);
    entry();
    /* Automotive services record their reschedule request in the established
     * backend ISR state. Its normal outer exit handles Task selection. */
    return 0u;
}
int Os_BackendHandlerAllowed(uint32_t interrupt, uint32_t (*handler)(void)) {
    if ((Os_Config != NULL) && (Os_Config->interrupts != NULL) &&
        (Os_Config->interrupts->entries != NULL) && (interrupt < OS_MAX_INTERRUPTS) &&
        (Os_Config->interrupts->entries[interrupt] != NULL)) {
        return handler == configured_interrupt;
    }
    return 1;
}
unsigned Os_BackendCurrentInterrupt(void) { return current_interrupt; }
int Os_BackendStarted(void) { return InterlockedCompareExchange(&started, 0, 0) != 0; }
void Os_BackendInterruptEnter(unsigned interrupt) {
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
void Os_BackendInterruptLeave(void) {
    const Os_ErrorParameters arguments = {.service_TerminateTask = {0u}};
    const size_t actor = OS_MAX_TASKS + current_interrupt;
    configASSERT(interrupt_depth != 0u && current_interrupt < OS_MAX_INTERRUPTS);
    if (Os_InterruptDisabled() != 0) {
        /* Native ISR dispatch already owns the interrupt mutex and has stopped
         * the automotive Task. Restore only this logical ISR's saved pairs. */
        Os_InterruptRestoreOwner();
        if ((current_interrupt < 32u) && (current_interrupt != OS_KERNEL_YIELD_INTERRUPT) &&
            (current_interrupt != OS_CONTROLLED_TICK_INTERRUPT) &&
            ((Os_Config->category1_isrs & ((uint32_t)1u << current_interrupt)) == 0u)) {
            (void)Os_ErrorResult(OSServiceId_InterruptMissingEnd, E_OS_DISABLEDINT, &arguments);
        }
    }
    if (resource_depth[actor] != 0u) {
        /* Dispatch still owns the native interrupt mutex. Release actual
         * configured resources in LIFO order before reporting the ISR fault. */
        while (resource_depth[actor] != 0u) {
            const size_t resource = owned_resources[actor][resource_depth[actor] - 1u];
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
int Os_BackendInterruptEnabled(unsigned interrupt) {
    int allowed;
    /* The private kernel yield is not an application ISR; it must remain
     * available to complete native port critical-section handshakes. */
    if (interrupt == OS_KERNEL_YIELD_INTERRUPT) {
        return 1;
    }
    allowed =
        (Os_BackendInterruptPriority(interrupt) != 0u) && (Os_InterruptAllows(interrupt) != 0) &&
        (Os_PortInterruptSourceEnabled(interrupt) != 0) &&
        (((interrupt < 32u) && ((Os_Config->category1_isrs & ((uint32_t)1u << interrupt)) != 0u)) ||
         ((LONG)Os_BackendInterruptPriority(interrupt) >
              InterlockedCompareExchange(&interrupt_ceiling, 0, 0) &&
          Os_HookBlocksCategory2() == 0));
#ifdef OS_TIME_TESTS
    if (interrupt == OS_CONTROLLED_TICK_INTERRUPT && allowed == 0) {
        Os_TimeTestMasked();
    }
#endif
    return allowed;
}
static size_t task_index(TaskType id) {
    size_t i;
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (Os_Config->tasks[i].id == id) {
            return i;
        }
    }
    return OS_MAX_TASKS;
}
int Os_BackendServiceContext(void) {
    const Os_NativeStack *stack = Os_StackCurrent();
    Os_BackendGuardService();
    if (stack == NULL) {
        return 0;
    }
    if (Os_HookQueryContext() != 0) {
        return 1;
    }
    return (Os_TargetReady() != 0) &&
           ((stack->role == 'T') ||
            ((stack->role == 'S') &&
             ((current_interrupt >= 32u) ||
              ((Os_Config->category1_isrs & ((uint32_t)1u << current_interrupt)) == 0u))));
}
int Os_BackendTaskOwner(TaskType id) {
    size_t index = current_task_index();
    if (index == OS_MAX_TASKS) {
        return 0;
    }
    return (Os_Config->tasks[index].id == id) ? 1 : -1;
}
int Os_BackendInputOwner(void) { return Os_BackendTaskOwner(Os_Config->input_task); }
StatusType Os_BackendTaskId(TaskRefType id) {
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    *id = INVALID_TASK;
    for (size_t i = 0u; i < Os_Config->task_count; ++i) {
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
AppModeType Os_BackendApplicationMode(void) { return startup_mode; }
void Os_BackendGuardService(void) {
    const Os_NativeStack *stack = Os_StackCurrent();
    if ((InterlockedCompareExchange(&Os_Closing, 0, 0) != 0) && (stack != NULL) &&
        ((stack->role == 'T') || (stack->role == 'S'))) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
/* Normalize only live metadata order stamps. This never chooses a runnable task;
 * the existing kernel ready-list policy remains the only selector. */
static void rebase_orders(void) {
    static OrderReference rebase_entries[OS_MAX_TASKS * (OS_MAX_ACTIVATIONS + 1u)];
    size_t i;
    unsigned j;
    unsigned count = 0u;
    uint64_t previous = 0u;
    uint64_t rank = 0u;
    for (i = 0u; i < Os_Config->task_count; ++i) {
        ActivationQueue *queue = &activations[i];
        for (j = 0u; j < queue->count; ++j) {
            uint64_t *slot = &queue->requests[(queue->head + j) % OS_MAX_ACTIVATIONS];
            unsigned position = count;
            while ((position > 0u) && (rebase_entries[position - 1u].previous > *slot)) {
                rebase_entries[position] = rebase_entries[position - 1u];
                --position;
            }
            rebase_entries[position].slot = slot;
            rebase_entries[position].previous = *slot;
            ++count;
        }
        if (ready_sequences[i] != 0u) {
            unsigned position = count;
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
static void append_activation(size_t index) {
    ActivationQueue *queue = &activations[index];
    unsigned tail = (queue->head + queue->count) % OS_MAX_ACTIVATIONS;
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
static void leave_running(size_t index) {
    if ((running_hook_task == index) && (index < OS_MAX_TASKS)) {
        if ((InterlockedCompareExchange(&Os_Closing, 0, 0) == 0) && (Os_Config->hooks != NULL)) {
            Os_HookInvoke(Os_Config->hooks->post_task, OS_HOOK_POST);
        }
        running_hook_task = OS_MAX_TASKS;
    }
}
void Os_BackendBeforeSelect(void *current, void *next) {
    if ((current != next) && (Os_TargetReady() != 0) &&
        (InterlockedCompareExchange(&Os_Closing, 0, 0) == 0)) {
        for (size_t i = 0u; i < Os_Config->task_count; ++i) {
            if (handles[i] == current) {
                leave_running(i);
                break;
            }
        }
    }
}
void Os_BackendOnSwitch(void) {
    if (Os_TargetReady() != 0) {
        arti_observe_tasks();
        size_t i;
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
static TaskStateType native_task_state(size_t index) {
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
static void arti_observe_tasks(void) {
    TaskStateType previous[OS_MAX_TASKS];
    uint8_t completed[OS_MAX_TASKS];
    Os_ArtiRunningTask = INVALID_TASK;
    for (size_t i = 0u; i < Os_Config->task_count; ++i) {
        previous[i] = Os_ArtiTasks[i].state;
        completed[i] = arti_completed[i];
        arti_completed[i] = 0u;
        Os_ArtiTasks[i].state = native_task_state(i);
        Os_ArtiTasks[i].priority = (uint8_t)uxTaskPriorityGet(handles[i]);
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
    for (size_t i = 0u; i < Os_Config->task_count; ++i) {
        const TaskStateType state = Os_ArtiTasks[i].state;
        if (completed[i] != 0u) {
            Os_ArtiTask(OS_ARTI_TASK_TERMINATE, Os_Config->tasks[i].id);
            previous[i] = SUSPENDED;
        } else if ((previous[i] == RUNNING) && (state != RUNNING)) {
            Os_ArtiTask(state == WAITING ? OS_ARTI_TASK_WAIT : OS_ARTI_TASK_PREEMPT,
                        Os_Config->tasks[i].id);
        }
    }
    for (size_t i = 0u; i < Os_Config->task_count; ++i) {
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
static void observe_transition(void) {
#if defined(OS_ACTIVATION_TESTS) || defined(OS_FINISH_TESTS) || defined(OS_RESOURCE_TESTS) ||      \
    defined(OS_EVENT_TESTS)
    const Os_NativeStack *stack = Os_StackCurrent();
    if ((Os_TargetReady() != 0) && (stack != NULL) &&
        ((stack->role == 'T') || (stack->role == 'S'))) {
        Os_TestObserve();
    }
#endif
}
static uint8_t internal_ceiling(size_t index) {
    size_t i;
    uint8_t ceiling = 0u;
    const Os_TaskConfig *task = &Os_Config->tasks[index];
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
static void acquire_internal(size_t index) {
    uint8_t ceiling = internal_ceiling(index);
    if ((ceiling != 0u) && (internal_held[index] == 0u)) {
        configASSERT(resource_depth[index] == 0u);
        internal_held[index] = 1u;
        if (uxTaskPriorityGet(handles[index]) < ceiling) {
            vTaskPrioritySet(handles[index], ceiling);
        }
    }
}
static void release_internal(size_t index) {
    if (internal_held[index] != 0u) {
        configASSERT(resource_depth[index] == 0u);
        internal_held[index] = 0u;
        vTaskPrioritySet(handles[index], Os_Config->tasks[index].priority);
    }
}
#ifdef OS_ACTIVATION_TESTS
void Os_TargetTestSequence(uint64_t sequence) {
    taskENTER_CRITICAL();
    configASSERT(sequence >= request_sequence);
    request_sequence = sequence;
    taskEXIT_CRITICAL();
}
unsigned Os_TargetTestPending(TaskType id) {
    size_t index = task_index(id);
    configASSERT(InterlockedCompareExchange(&Os_Closing, 0, 0) != 0);
    configASSERT(index < Os_Config->task_count);
    return activations[index].count;
}
#endif
typedef struct {
    TaskFunction_t entry;
    void *argument;
    HANDLE registered, gate;
    char role;
    const StackType_t *buffer_top;
} NativeTaskStart;
static NativeTaskStart native_starts[OS_MAX_TASKS + 2u];
#ifdef OS_STACK_TESTS
static unsigned failed_operation;
void Os_StackTestFailOperation(unsigned operation) { failed_operation = operation; }
#endif
static int operation_fails(unsigned operation) {
#ifdef OS_STACK_TESTS
    if (failed_operation == operation) {
        failed_operation = 0u;
        Os_TargetTrace('F');
        return 1;
    }
#else
    (void)operation;
#endif
    return 0;
}
DWORD Os_PortSuspendThread(HANDLE thread) {
    return operation_fails(1u) ? (DWORD)-1 : SuspendThread(thread);
}
BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context) {
    return operation_fails(2u) ? FALSE : GetThreadContext(thread, context);
}

void Os_PortPostInterrupt(uint32_t interrupt) {
#ifdef _WIN32
    vPortGenerateSimulatedInterruptFromWindowsThread(interrupt);
#else
    Os_PosixPostInterrupt(interrupt);
#endif
}
void Os_TargetTrace(char marker) {
    if (marker == '\0') {
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
int Os_TargetReady(void) {
    return InterlockedCompareExchange(&ready, 0, 0) != 0 &&
           InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
}
static void report_and_exit(void) {
    int output_failed = 0;
    Os_StackCheck();
    if (Os_StackHasFault() != 0) {
        shutdown_reason = E_OS_STACKFAULT;
    }
    Os_ShutdownHookInvoke(shutdown_reason);
#ifdef OS_ARTI_TESTS
    Os_ArtiTestShutdownObserved();
#endif
    if (Os_StackReport() != 0) {
        output_failed = 1;
    }
    if (printf("lifecycle=Closed state=%s reason=%u trace=%s threads=%u events=%u mutexes=%u "
#ifdef __linux__
               "resource_calls=%u hidden=2 controllers=2 heap=posix static=freertos "
#else
               "resource_calls=%u hidden=2 controllers=2 heap=windows static=freertos "
#endif
               "input_closed=1 tick_closed=1 time_signal_failed=%d trace_dropped=%llu\n",
               shutdown_reason == E_OK ? "Ready" : "Failed", shutdown_reason, trace, threads,
               events, mutexes, resource_calls, Os_TimeSignalFailed(),
               (unsigned long long)trace_dropped) < 0) {
        output_failed = 1;
    }
    if (fflush(stdout) != 0) {
        output_failed = 1;
    }
    ExitProcess((output_failed != 0 && shutdown_reason == E_OK) ? E_OS_STATE : shutdown_reason);
}
static void stop_thread(HANDLE thread) {
    CONTEXT context;
#ifdef __linux__
    if (Os_StackHasFault() != 0 && Os_StackThreadHasFault(GetThreadId(thread)) != 0) {
        return; /* The guard handler parked the damaged actor on its altstack. */
    }
    if (Os_HostStopActor(thread, 2000u) == 0) {
        ExitProcess(E_OS_STATE);
    }
#else
    if (SuspendThread(thread) == (DWORD)-1) {
        ExitProcess(E_OS_STATE);
    }
#endif
    context.ContextFlags = CONTEXT_CONTROL;
    if (!Os_PortGetThreadContext(thread, &context)) {
        ExitProcess(E_OS_STATE);
    }
}
static DWORD WINAPI control(void *argument) {
    HANDLE event = argument == NULL ? close_event : backup_event;
    unsigned i;
    DWORD current = GetCurrentThreadId();
    DWORD waited;
    unsigned index = argument == NULL ? 0u : 1u;
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
#if defined(__linux__) && defined(OS_STACK_TESTS)
    if (index == 1u && InterlockedExchange(&backup_fault_requested, 0) != 0) {
        void Os_StackTestBackupFault(void);
        Os_StackTestBackupFault();
        Os_BackendShutdown(E_OS_STATE);
    }
#endif
#ifdef OS_STACK_TESTS
    {
        void Os_StackTestBeforeClose(void);
        Os_StackTestBeforeClose();
    }
#endif
#ifdef OS_ACTIVATION_TESTS
    Os_TestBeforeClose();
#endif
#ifdef __linux__
    if (Os_StackHasFault() != 0) {
        Os_TimeClose();
        Os_MailboxClose();
    }
#endif
    vPortEndScheduler();
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
    if (Os_StackHasFault()) {
        ShutdownOS(E_OS_STACKFAULT);
    }
    report_and_exit();
    return 0u;
}
void Os_BackendStackFault(char failed_role) {
    Os_TimeClose();
    Os_MailboxClose();
    InterlockedOr(&activation_admission, 2);
    InterlockedExchange(&Os_Closing, 1);
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
    Sleep(INFINITE);
    ExitProcess(E_OS_STACKFAULT);
}
#ifdef __linux__
void Os_BackendSignalStackFault(char failed_role) {
    (void)InterlockedOr(&activation_admission, 2);
    (void)InterlockedExchange(&Os_Closing, 1);
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
}
#endif
void Os_BackendShutdown(StatusType error) {
    const Os_NativeStack *current = Os_StackCurrent();
    if (error == E_OS_STACKFAULT && Os_StackHasFault() && current != NULL &&
        (current->role == 'C' || current->role == 'D') &&
        current->thread_id != Os_Fault.thread_id) {
        report_and_exit();
    }
    Os_BackendRequestShutdown(error);
    Sleep(INFINITE);
    ExitProcess(E_OS_STATE);
}
void Os_BackendRequestShutdown(StatusType error) {
    Os_TimeClose();
    Os_MailboxClose();
    if ((InterlockedOr(&activation_admission, 2) & 2) == 0) {
        InterlockedExchange(&Os_Closing, 1);
        shutdown_reason = error;
        InterlockedExchange(&ready, 0);
        if (controller_healthy[0] == 0 && controller_healthy[1] == 0) {
            report_and_exit();
        }
        if (!Os_HostSetEvent((controller_healthy[0] != 0) ? close_event : backup_event)) {
            ExitProcess(E_OS_STATE);
        }
    }
}
void Os_BackendAssert(const char *file, int line) {
    (void)file;
    (void)line;
    Os_BackendShutdown(E_OS_STATE);
}
static int fail_next(void) {
    ++resource_calls;
#ifdef OS_HOST_FAILURE_TESTS
    return fail_resource != 0u && resource_calls == fail_resource;
#else
    return 0;
#endif
}
HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name) {
    HANDLE result = fail_next() ? NULL : CreateEventA(attributes, manual, initial, name);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++events;
    return result;
}
HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name) {
    HANDLE result = fail_next() ? NULL : CreateMutexA(attributes, owner, name);
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++mutexes;
    return result;
}
HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                     LPVOID argument, DWORD flags, LPDWORD id) {
#ifdef __linux__
    HANDLE result = (attributes != NULL || fail_next())
                        ? NULL
                        : Os_HostCreateActor(stack, start, argument, flags, id, 0);
#else
    HANDLE result =
        fail_next() ? NULL : CreateThread(attributes, stack, start, argument, flags, id);
#endif
    if (result == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++threads;
    return result;
}
static DWORD WINAPI native_task_start(void *argument) {
    NativeTaskStart *start = argument;
    if (!Os_StackRegister(start->role)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_StackRecordBuffer(start->buffer_top);
    if (start->entry == task_entry) {
        const Os_TaskConfig *task = start->argument;
        const size_t index = task_index(task->id);
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
HANDLE Os_PortTaskThread(TaskFunction_t code, void *argument, const StackType_t *buffer_top) {
    NativeTaskStart *start;
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
    thread = Os_PortThread(NULL, Os_Config->host_stack_reserve, native_task_start, start,
                           STACK_SIZE_PARAM_IS_A_RESERVATION, NULL);
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
void Os_PortResumeThread(HANDLE thread) {
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
void vApplicationGetIdleTaskMemory(StaticTask_t **task, StackType_t **stack,
                                   configSTACK_DEPTH_TYPE *depth) {
    *task = &task_memory[OS_MAX_TASKS + 1u];
    *stack = task_buffers[OS_MAX_TASKS + 1u];
    *depth = 512;
}
void vApplicationIdleHook(void) {
    Os_StackCheck();
#ifdef OS_IDLE_TESTS
    Os_TestIdleObserved();
#endif
#ifdef OS_STACK_TESTS
    void Os_StackTestIdle(void);
    Os_StackTestIdle();
#endif
}
static void task_entry(void *argument) {
    const Os_TaskConfig *task = argument;
    const size_t index = task_index(task->id);
    configASSERT(index < Os_Config->task_count);
    /* The private backend trampoline discards completed application frames.
     * No application or generated Runnable uses setjmp/longjmp. */
    (void)setjmp(restart_frames[index]);
    (void)Os_ArtiCallerSave();
    configASSERT(activations[index].count != 0u);
    task->entry();
    /* A returned entry is an erroneous activation, not a whole-OS shutdown. */
    Os_BackendMissingEnd();
}
static void bootstrap(void *argument) {
    size_t i;
    (void)argument;
    if ((Os_Config->interrupts != NULL) && (Os_Config->interrupts->entries != NULL)) {
        for (i = 2u; i < OS_MAX_INTERRUPTS; ++i) {
            if (Os_Config->interrupts->entries[i] != NULL) {
                vPortSetInterruptHandler((uint32_t)i, configured_interrupt);
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
void Os_BackendStart(AppModeType mode) {
    size_t i;
    Os_TargetTrace('I');
#ifdef OS_HOST_FAILURE_TESTS
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
        fail_resource = (unsigned)value;
    }
#endif
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
#ifdef __linux__
    Os_StackPrepare();
#else
    Os_StackInit();
#endif
    Os_MailboxInstall();
#ifndef __linux__
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &main_thread,
                         0u, FALSE, DUPLICATE_SAME_ACCESS)) {
        Os_BackendShutdown(E_OS_STATE);
    }
#endif
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
        handles[i] = xTaskCreateStatic(task_entry, Os_Config->tasks[i].name, 512,
                                       (void *)&Os_Config->tasks[i], Os_Config->tasks[i].priority,
                                       task_buffers[i], &task_memory[i]);
        configASSERT(handles[i] != NULL);
        vTaskOsSetReadySequence(handles[i], (activations[i].count == 0u)
                                                ? 0u
                                                : activations[i].requests[activations[i].head]);
        vTaskSuspend(handles[i]);
    }
    configASSERT(xTaskCreateStatic(bootstrap, "Os_Bootstrap", 512, NULL, 31,
                                   task_buffers[OS_MAX_TASKS], &task_memory[OS_MAX_TASKS]) != NULL);
    vTaskStartScheduler();
    Os_BackendShutdown(E_OS_STATE);
}
#ifdef __linux__
void Os_BackendSetDispatcher(void) {
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &main_thread,
                         0u, FALSE, DUPLICATE_SAME_ACCESS)) {
        Os_BackendShutdown(E_OS_STATE);
    }
    Os_HostSetDispatcher(main_thread);
    Os_StackInit();
}
#endif
StatusType Os_BackendState(TaskType id, TaskStateRefType state) {
    size_t i;
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
StatusType Os_BackendActivate(TaskType id) {
    size_t index;
    const ActivationQueue *queue;
    const Os_NativeStack *stack;
    if (!Os_BackendServiceContext()) {
        return E_OS_CALLEVEL;
    }
    index = task_index(id);
    if (index == OS_MAX_TASKS) {
        return E_OS_ID;
    }
    queue = &activations[index];
    stack = Os_StackCurrent();
#ifdef OS_ACTIVATION_TESTS
    Os_TestBeforeActivationLock();
#endif
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (!Os_TargetReady()) {
        taskEXIT_CRITICAL();
        return E_OS_STATE;
    }
    if (queue->count >= Os_Config->tasks[index].activation_limit) {
        taskEXIT_CRITICAL();
        return E_OS_LIMIT;
    }
#ifdef OS_ACTIVATION_TESTS
    Os_TestAtActivationAdmission(0u);
#endif
    if (InterlockedCompareExchange(&activation_admission, 1, 0) != 0) {
        /* Closing already owns admission; no record or ready key is changed. */
        Os_BackendShutdown(E_OS_STATE);
    }
#ifdef OS_ACTIVATION_TESTS
    Os_TestAtActivationAdmission(1u);
#endif
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
static size_t current_task_index(void) {
    const Os_NativeStack *stack = Os_StackCurrent();
    size_t index;
    if ((stack == NULL) || (stack->role != 'T') || (Os_TargetReady() == 0)) {
        return OS_MAX_TASKS;
    }
    for (index = 0u; index < Os_Config->task_count; ++index) {
        if (handles[index] == xTaskGetCurrentTaskHandle()) {
            return index;
        }
    }
    return OS_MAX_TASKS;
}
static StatusType complete_activation(TaskType id, int chain, int returned) {
    const Os_NativeStack *stack = Os_StackCurrent();
    size_t index;
    TaskHandle_t current;
    ActivationQueue *queue;
    size_t target = OS_MAX_TASKS;
    Os_BackendGuardService();
    if ((Os_TargetReady() == 0) || (stack == NULL) || (stack->role != 'T')) {
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
            const size_t resource = owned_resources[index][resource_depth[index] - 1u];
            configASSERT(resource < Os_Config->resource_count);
            const StatusType released = Os_BackendResource(Os_Config->resources[resource].id, 0);
            configASSERT(released == E_OK);
        }
    }
#ifdef OS_FINISH_TESTS
    Os_TestFinishBoundary(0u);
#endif
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
#ifdef OS_FINISH_TESTS
    Os_TestFinishBoundary(1u);
#endif
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
StatusType Os_BackendFinish(void) { return complete_activation(0u, 0, 0); }
StatusType Os_BackendChain(TaskType id) { return complete_activation(id, 1, 0); }
void Os_BackendMissingEnd(void) {
    const Os_ErrorParameters arguments = {.service_TerminateTask = {0u}};
    Os_StackCheck();
    /* Report before leaving RUNNING, while the application's leaked resource
     * state remains observable. Normal completion then cleans this activation. */
    (void)Os_ErrorResult(OSServiceId_TaskMissingEnd, E_OS_MISSINGEND, &arguments);
    (void)complete_activation(0u, 0, 1);
    Os_BackendShutdown(E_OS_STATE);
}
StatusType Os_BackendResource(ResourceType id, int acquire) {
    static UBaseType_t saved_priorities[OS_RESOURCE_ACTORS][OS_MAX_RESOURCES];
    static LONG saved_interrupt_ceilings[OS_RESOURCE_ACTORS][OS_MAX_RESOURCES];
    const Os_NativeStack *stack = Os_StackCurrent();
    size_t task;
    size_t resource;
    unsigned depth;
    StatusType status = E_OK;
    const Os_ResourceConfig *config;
    Os_BackendGuardService();
    task = current_task_index();
    if (task == OS_MAX_TASKS) {
        if ((Os_TargetReady() == 0) || (stack == NULL) || (stack->role != 'S') ||
            (current_interrupt == OS_KERNEL_YIELD_INTERRUPT) || (current_interrupt >= 32u) ||
            ((Os_Config->category1_isrs & ((uint32_t)1u << current_interrupt)) != 0u)) {
            return E_OS_CALLEVEL;
        }
        task = OS_MAX_TASKS + current_interrupt;
    }
    for (resource = 0u; resource < Os_Config->resource_count; ++resource) {
        if (Os_Config->resources[resource].id == id) {
            break;
        }
    }
    if (resource == Os_Config->resource_count) {
        return E_OS_ID;
    }
    config = &Os_Config->resources[resource];
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
             ((config->isr_access & ((uint32_t)1u << current_interrupt)) == 0u))) {
            status = E_OS_ACCESS;
        } else {
            saved_priorities[task][depth] =
                (task >= OS_MAX_TASKS) ? 0u : uxTaskPriorityGet(handles[task]);
            saved_interrupt_ceilings[task][depth] =
                InterlockedCompareExchange(&interrupt_ceiling, 0, 0);
            owned_resources[task][depth] = resource;
            resource_depth[task] = depth + 1u;
            resource_owner[resource] = (unsigned)task + 1u;
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
               (resource_owner[resource] != (unsigned)task + 1u)) {
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
StatusType Os_BackendSchedule(void) {
    size_t index = current_task_index();
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
StatusType Os_BackendWait(EventMaskType mask) {
    size_t index = current_task_index();
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
StatusType Os_BackendClear(EventMaskType mask) {
    size_t index = current_task_index();
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
StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output) {
    size_t index;
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
            const Os_NativeStack *stack = Os_StackCurrent();
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
StatusType Os_BackendInspect(TaskType id, Os_ActivationInfo *info) {
    size_t index;
    unsigned j;
    Os_ActivationInfo snapshot;
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
    snapshot.effective_priority = (uint8_t)uxTaskPriorityGet(handles[index]);
    snapshot.resource_count = resource_depth[index];
    snapshot.internal_held = internal_held[index];
    snapshot.internal_ceiling = internal_ceiling(index);
    snapshot.waiting = task_waiting[index];
    snapshot.wait_mask = wait_masks[index];
    for (j = 0u; j < snapshot.resource_count; ++j) {
        size_t resource = owned_resources[index][j];
        snapshot.resources[j] = (resource < Os_Config->resource_count)
                                    ? Os_Config->resources[resource].id
                                    : RES_SCHEDULER;
    }
    *info = snapshot;
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}
