#include "Os_Backend.h"
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
static unsigned resource_depth[OS_MAX_TASKS];
static size_t owned_resources[OS_MAX_TASKS][OS_MAX_RESOURCES];
static uint64_t request_sequence;
typedef struct {
    uint64_t *slot;
    uint64_t previous;
} OrderReference;
static jmp_buf restart_frames[OS_MAX_TASKS];
static volatile LONG isr_reschedule;
int Os_BackendTakeIsrReschedule(void) { return InterlockedExchange(&isr_reschedule, 0) != 0; }
static HANDLE close_event;
static HANDLE control_thread;
static HANDLE backup_thread;
#ifdef OS_STACK_TESTS
HANDLE Os_StackTestBackupThread(void) { return backup_thread; }
#endif
static HANDLE backup_event;
static HANDLE main_thread;
static HANDLE controller_registered[2];
static volatile LONG controller_healthy[2];
static HANDLE task_threads[OS_MAX_TASKS + 2u];
static unsigned task_thread_count;
#ifdef OS_STACK_TESTS
HANDLE Os_StackTestTaskThread(unsigned index) { return task_threads[index]; }
#endif
static volatile LONG ready;
volatile LONG Os_Closing;
/* Bit 0: an accepted activation transaction; bit 1: admission permanently closed.
 * Both admission and close linearize on this atomic word. Close never waits for
 * a transaction or a damaged actor holding the ordinary interrupt mutex. */
static volatile LONG activation_admission;
static StatusType shutdown_reason;
static AppModeType startup_mode;
static char trace[128];
static size_t trace_length;
static unsigned resource_calls, threads, events, mutexes;
static unsigned fail_resource;
static volatile LONG started;
static void task_entry(void *argument);
static void bootstrap(void *argument);
static size_t task_index(TaskType id) {
    size_t i;
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (Os_Config->tasks[i].id == id) {
            return i;
        }
    }
    return OS_MAX_TASKS;
}
static int activation_context(void) {
    const Os_NativeStack *stack = Os_StackCurrent();
    Os_BackendGuardService();
    return (Os_TargetReady() != 0) && (stack != NULL) &&
           ((stack->role == 'T') || (stack->role == 'S'));
}
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
    static OrderReference rebase_entries[OS_MAX_TASKS * OS_MAX_ACTIVATIONS];
    size_t i;
    unsigned j;
    unsigned count = 0u;
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
    }
    for (j = 0u; j < count; ++j) {
        *rebase_entries[j].slot = (uint64_t)j + 1u;
    }
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (activations[i].count != 0u) {
            vTaskOsSetReadySequence(handles[i], activations[i].requests[activations[i].head]);
        }
    }
    request_sequence = count;
}
static void append_activation(size_t index) {
    ActivationQueue *queue = &activations[index];
    unsigned tail = (queue->head + queue->count) % OS_MAX_ACTIVATIONS;
    if (queue->count == 0u) {
        task_events[index] = 0u;
    }
    ++request_sequence;
    queue->requests[tail] = request_sequence;
    ++queue->count;
}
void Os_BackendObserveSwitch(void) {
#if defined(OS_ACTIVATION_TESTS) || defined(OS_FINISH_TESTS)
    const Os_NativeStack *stack = Os_StackCurrent();
    if ((Os_TargetReady() != 0) && (stack != NULL) &&
        ((stack->role == 'T') || (stack->role == 'S'))) {
        Os_TestObserve();
    }
#endif
}
static void observe_transition(void) { Os_BackendObserveSwitch(); }
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

void Os_TargetTrace(char marker) {
    if (trace_length + 1u >= sizeof(trace) || marker == '\0') {
        if (InterlockedCompareExchange(&Os_Closing, 0, 0) != 0) {
            return;
        }
        Os_BackendShutdown(E_OS_STATE);
    }
    trace[trace_length++] = marker;
}
int Os_TargetReady(void) {
    return InterlockedCompareExchange(&ready, 0, 0) != 0 &&
           InterlockedCompareExchange(&Os_Closing, 0, 0) == 0;
}
static void report_and_exit(void) {
    Os_StackCheck();
    if (Os_StackHasFault() != 0) {
        shutdown_reason = E_OS_STACKFAULT;
    }
    ShutdownHook(shutdown_reason);
    Os_StackReport();
    printf("lifecycle=Closed state=%s reason=%u trace=%s threads=%u events=%u mutexes=%u "
           "resource_calls=%u hidden=2 controllers=2 heap=windows static=freertos "
           "input_closed=1 tick_closed=1\n",
           shutdown_reason == E_OK ? "Ready" : "Failed", shutdown_reason, trace, threads, events,
           mutexes, resource_calls);
    fflush(stdout);
    ExitProcess(shutdown_reason == E_OK ? 0u : shutdown_reason);
}
static void stop_thread(HANDLE thread) {
    CONTEXT context;
    if (SuspendThread(thread) == (DWORD)-1) {
        ExitProcess(E_OS_STATE);
    }
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
#ifdef OS_STACK_TESTS
    {
        void Os_StackTestBeforeClose(void);
        Os_StackTestBeforeClose();
    }
#endif
#ifdef OS_ACTIVATION_TESTS
    Os_TestBeforeClose();
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
    return fail_resource != 0u && resource_calls == fail_resource;
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
    HANDLE result =
        fail_next() ? NULL : CreateThread(attributes, stack, start, argument, flags, id);
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
    configASSERT(activations[index].count != 0u);
    task->entry();
    /* Full missing-end handling belongs to 4.18; never silently keep running. */
    Os_BackendShutdown(E_OS_STATE);
}
static void bootstrap(void *argument) {
    size_t i;
    (void)argument;
    StartupHook();
    taskENTER_CRITICAL();
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if ((Os_Config->tasks[i].autostart_modes & startup_mode) != 0u) {
            vTaskResume(handles[i]);
        }
    }
    InterlockedExchange(&ready, 1);
    Os_TargetTrace('R');
    observe_transition();
    taskEXIT_CRITICAL();
    vTaskSuspend(NULL);
    Os_BackendShutdown(E_OS_STATE);
}
void Os_BackendStart(AppModeType mode) {
    size_t i;
    const char *failure = getenv("AUTOSAR_OS_FAIL_RESOURCE");
    Os_TargetTrace('I');
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
    if (InterlockedCompareExchange(&started, 1, 0) != 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    if (Os_Config == NULL || (mode != 1u && mode != 2u)) {
        Os_BackendShutdown(E_OS_VALUE);
    }
    startup_mode = mode;
    Os_StackInit();
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &main_thread,
                         0u, FALSE, DUPLICATE_SAME_ACCESS)) {
        Os_BackendShutdown(E_OS_STATE);
    }
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
StatusType Os_BackendState(TaskType id, TaskStateRefType state) {
    size_t i;
    Os_StackCheck();
    if (!activation_context()) {
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
                *state = SUSPENDED;
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
    if (!activation_context()) {
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
        vTaskOsSetReadySequence(handles[index], queue->requests[queue->head]);
        if (stack->role == 'S') {
            BaseType_t wake = xTaskResumeFromISR(handles[index]);
            if (wake != pdFALSE) {
                InterlockedExchange(&isr_reschedule, 1);
            }
        } else {
            vTaskResume(handles[index]);
        }
    }
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
static StatusType complete_activation(TaskType id, int chain) {
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
    if (resource_depth[index] != 0u) {
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
    queue->requests[queue->head] = 0u;
    queue->head = (queue->head + 1u) % OS_MAX_ACTIVATIONS;
    --queue->count;
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
    vTaskOsSetReadySequence(current, (queue->count == 0u) ? 0u : queue->requests[queue->head]);
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
StatusType Os_BackendFinish(void) { return complete_activation(0u, 0); }
StatusType Os_BackendChain(TaskType id) { return complete_activation(id, 1); }
StatusType Os_BackendResource(ResourceType id, int acquire) {
    static UBaseType_t saved_priorities[OS_MAX_TASKS][OS_MAX_RESOURCES];
    static unsigned resource_owner[OS_MAX_RESOURCES];
    size_t task;
    size_t resource;
    unsigned depth;
    StatusType status = E_OK;
    Os_BackendGuardService();
    task = current_task_index();
    if (task == OS_MAX_TASKS) {
        return E_OS_CALLEVEL;
    }
    for (resource = 0u; resource < Os_Config->resource_count; ++resource) {
        if (Os_Config->resources[resource].id == id) {
            break;
        }
    }
    if (resource == Os_Config->resource_count) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    depth = resource_depth[task];
    if (acquire != 0) {
        const Os_ResourceConfig *config = &Os_Config->resources[resource];
        if ((depth >= OS_MAX_RESOURCES) || (resource_owner[resource] != 0u) ||
            ((config->task_access & (1u << Os_Config->tasks[task].id)) == 0u) ||
            (Os_Config->tasks[task].priority > config->ceiling)) {
            status = E_OS_ACCESS;
        } else {
            saved_priorities[task][depth] = uxTaskPriorityGet(handles[task]);
            owned_resources[task][depth] = resource;
            resource_depth[task] = depth + 1u;
            resource_owner[resource] = (unsigned)task + 1u;
            if (config->ceiling > saved_priorities[task][depth]) {
                vTaskPrioritySet(handles[task], config->ceiling);
            }
        }
    } else if ((depth == 0u) || (depth > OS_MAX_RESOURCES) ||
               (owned_resources[task][depth - 1u] != resource) ||
               (resource_owner[resource] != (unsigned)task + 1u)) {
        status = E_OS_NOFUNC;
    } else {
        resource_depth[task] = depth - 1u;
        resource_owner[resource] = 0u;
        vTaskPrioritySet(handles[task], saved_priorities[task][depth - 1u]);
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
StatusType Os_BackendEvent(TaskType id, EventMaskType mask, EventMaskRefType output) {
    size_t index;
    StatusType status = E_OK;
    if (!activation_context()) {
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
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
StatusType Os_BackendInspect(TaskType id, Os_ActivationInfo *info) {
    size_t index;
    unsigned j;
    Os_ActivationInfo snapshot;
    if (!activation_context()) {
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
    for (j = 0u; j < snapshot.resource_count; ++j) {
        snapshot.resources[j] = Os_Config->resources[owned_resources[index][j]].id;
    }
    *info = snapshot;
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}
