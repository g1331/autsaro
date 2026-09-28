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
static volatile LONG ready;
volatile LONG Os_Closing;
static StatusType shutdown_reason;
static AppModeType startup_mode;
static char trace[128];
static size_t trace_length;
static unsigned resource_calls, threads, events, mutexes;
static unsigned fail_resource;
static volatile LONG started;

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
    ShutdownHook(shutdown_reason);
    printf("lifecycle=Closed state=%s reason=%u trace=%s threads=%u events=%u mutexes=%u "
           "resource_calls=%u hidden=2 heap=windows static=freertos "
           "input_closed=1 tick_closed=1\n",
           shutdown_reason == E_OK ? "Ready" : "Failed", shutdown_reason, trace, threads, events,
           mutexes, resource_calls);
    fflush(stdout);
    ExitProcess(shutdown_reason == E_OK ? 0u : shutdown_reason);
}
static DWORD WINAPI control(void *argument) {
    (void)argument;
    if (WaitForSingleObject(close_event, INFINITE) != WAIT_OBJECT_0) {
        ExitProcess(E_OS_STATE);
    }
    vPortEndScheduler();
    report_and_exit();
    return 0u;
}
void Os_BackendShutdown(StatusType error) {
    if (InterlockedCompareExchange(&Os_Closing, 1, 0) == 0) {
        shutdown_reason = error;
        InterlockedExchange(&ready, 0);
        if (control_thread == NULL) {
            report_and_exit();
        }
        if (!SetEvent(close_event)) {
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
void vApplicationGetIdleTaskMemory(StaticTask_t **task, StackType_t **stack,
                                   configSTACK_DEPTH_TYPE *depth) {
    *task = &task_memory[OS_MAX_TASKS + 1u];
    *stack = task_buffers[OS_MAX_TASKS + 1u];
    *depth = 512;
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
    close_event = Os_PortEvent(NULL, FALSE, FALSE, NULL);
    control_thread =
        Os_PortThread(NULL, 262144u, control, NULL, STACK_SIZE_PARAM_IS_A_RESERVATION, NULL);
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
