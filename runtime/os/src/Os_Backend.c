#include "Os_Backend.h"
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

static StaticTask_t task_memory[OS_MAX_TASKS + 2u];
static StackType_t task_buffers[OS_MAX_TASKS + 2u][512];
static TaskHandle_t handles[OS_MAX_TASKS];
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
static StatusType shutdown_reason;
static AppModeType startup_mode;
static char trace[128];
static size_t trace_length;
static unsigned resource_calls, threads, events, mutexes;
static unsigned fail_resource;
static volatile LONG started;
static void task_entry(void *argument);
static void bootstrap(void *argument);
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
    if (!SetEvent(controller_registered[index])) {
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
    if (!SetEvent(controller_healthy[0] != 0 ? close_event : backup_event)) {
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
    if (InterlockedCompareExchange(&Os_Closing, 1, 0) == 0) {
        shutdown_reason = error;
        InterlockedExchange(&ready, 0);
        if (controller_healthy[0] == 0 && controller_healthy[1] == 0) {
            report_and_exit();
        }
        if (!SetEvent(controller_healthy[0] != 0 ? close_event : backup_event)) {
            ExitProcess(E_OS_STATE);
        }
    }
    Sleep(INFINITE);
    ExitProcess(E_OS_STATE);
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
    if (!SetEvent(start->registered) ||
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
    if (!SetEvent(start->gate)) {
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
    task->entry();
    /* Full missing-end handling belongs to 4.18; never silently keep running. */
    Os_BackendShutdown(E_OS_STATE);
}
static void bootstrap(void *argument) {
    size_t i;
    (void)argument;
    StartupHook();
    InterlockedExchange(&ready, 1);
    Os_TargetTrace('R');
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if ((Os_Config->tasks[i].autostart_modes & startup_mode) != 0u) {
            vTaskResume(handles[i]);
        }
    }
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
        handles[i] = xTaskCreateStatic(task_entry, Os_Config->tasks[i].name, 512,
                                       (void *)&Os_Config->tasks[i], Os_Config->tasks[i].priority,
                                       task_buffers[i], &task_memory[i]);
        configASSERT(handles[i] != NULL);
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
    for (i = 0u; i < Os_Config->task_count; ++i) {
        if (Os_Config->tasks[i].id == id) {
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
                return E_OS_STATE;
            }
            return E_OK;
        }
    }
    return E_OS_ID;
}
