#include "Os_Target.h"
#include "Os_Backend.h"
#include <string.h>
#include <limits.h>

static const char *scenario;
static volatile unsigned checksum;
static unsigned hook_entries;
HANDLE Os_StackTestBackupThread(void);
HANDLE Os_StackTestTaskThread(unsigned index);
void Os_StackTestFailOperation(unsigned operation);
__attribute__((noinline)) static void consume_stack(unsigned depth) {
    volatile unsigned char buffer[4096];
    unsigned i;
    for (i = 0u; i < sizeof(buffer); ++i) {
        buffer[i] = (unsigned char)depth;
    }
    if (depth != UINT_MAX) {
        consume_stack(depth + 1u);
    }
    checksum += buffer[depth % sizeof(buffer)];
}
static uint32_t stack_isr(void) {
    Os_TargetTrace('J');
    consume_stack(0u);
    Os_TargetTrace('X');
    return 0u;
}
static VOID CALLBACK backup_fault(ULONG_PTR argument) {
    (void)argument;
    consume_stack(0u);
    Os_TargetTrace('X');
}
void Os_StackTestIdle(void) {
    if (strcmp(scenario, "idle") == 0) {
        consume_stack(0u);
        Os_TargetTrace('X');
    }
}
void Os_StackTestBeforeClose(void) {
    unsigned wait;
    if (strcmp(scenario, "concurrent") != 0) {
        return;
    }
    for (wait = 0u; wait < 1000u && Os_StackFaultCount() < 2; ++wait) {
        Sleep(1u);
    }
    if (Os_StackFaultCount() != 2) {
        ExitProcess(E_OS_STATE);
    }
}
void StartupHook(void) {
    Os_TargetTrace('S');
    if (strcmp(scenario, "startup") == 0) {
        consume_stack(0u);
        Os_TargetTrace('X');
    }
    vPortSetInterruptHandler(31u, stack_isr);
}
void ShutdownHook(StatusType Error) {
    const Os_NativeStack *current = Os_StackCurrent();
    ++hook_entries;
    if (strcmp(scenario, "double-control-output") == 0 && Error == E_OK) {
        CloseHandle(GetStdHandle(STD_OUTPUT_HANDLE));
    }
    if ((strcmp(scenario, "control") == 0 && Error == E_OK) ||
        strcmp(scenario, "double-control") == 0 || strcmp(scenario, "double-control-output") == 0) {
        consume_stack(0u);
        Os_TargetTrace('X');
    }
    if (current == NULL || (current->role != 'C' && current->role != 'D') ||
        current->thread_id == Os_Fault.thread_id || Os_TargetReady()) {
        Os_TargetTrace('X');
    }
    Os_TargetTrace('D');
}
static void entry(void) {
    Os_TargetTrace('A');
    if (strcmp(scenario, "task") == 0) {
        consume_stack(0u);
    } else if (strcmp(scenario, "sp-corrupt") == 0) {
        HANDLE thread = Os_StackTestTaskThread(1u);
        CONTEXT context;
        TaskHandle_t task;
        context.ContextFlags = CONTEXT_CONTROL;
        if (!GetThreadContext(thread, &context)) {
            ShutdownOS(E_OS_STATE);
        }
        context.Rsp = 1u;
        if (!SetThreadContext(thread, &context)) {
            ShutdownOS(E_OS_STATE);
        }
        task = xTaskGetHandle("Inactive");
        vTaskResume(task);
    } else if (strncmp(scenario, "api-", 4u) == 0) {
        unsigned operation = strcmp(scenario, "api-suspend") == 0
                                 ? 1u
                                 : (strcmp(scenario, "api-context") == 0 ? 2u : 3u);
        Os_StackTestFailOperation(operation);
        vTaskResume(xTaskGetHandle("Inactive"));
    } else if (strcmp(scenario, "boundary") == 0 || strcmp(scenario, "boundary-read") == 0) {
        const Os_NativeStack *stack = Os_StackCurrent();
        volatile unsigned char *outside = (volatile unsigned char *)(stack->reserve_low + 8u);
        if (strcmp(scenario, "boundary-read") == 0) {
            checksum += *outside;
        } else {
            *outside = 42u;
        }
    } else if (strcmp(scenario, "isr") == 0) {
        vPortGenerateSimulatedInterrupt(31u);
    } else if (strcmp(scenario, "unrelated") == 0) {
        volatile unsigned char *invalid = (volatile unsigned char *)1u;
        *invalid = 42u;
    } else if (strcmp(scenario, "backup") == 0) {
        if (QueueUserAPC(backup_fault, Os_StackTestBackupThread(), 0u) == 0u) {
            ShutdownOS(E_OS_STATE);
        }
        vTaskSuspend(NULL);
    } else if (strcmp(scenario, "concurrent") == 0) {
        if (QueueUserAPC(backup_fault, Os_StackTestBackupThread(), 0u) == 0u) {
            ShutdownOS(E_OS_STATE);
        }
        consume_stack(0u);
    } else if (strcmp(scenario, "idle") == 0) {
        vTaskSuspend(NULL);
    } else {
        ShutdownOS(E_OK);
    }
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[2] = {{0u, "FaultTask", entry, 2u, 1u, OS_BASIC_TASK, 1u},
                              {1u, "Inactive", entry, 3u, 0u, OS_BASIC_TASK, 1u}};
    Os_TargetConfig config = {tasks, 1u, 262144u};
    scenario = argc == 2 ? argv[1] : "normal";
    if (strcmp(scenario, "sp-corrupt") == 0 || strncmp(scenario, "api-", 4u) == 0) {
        config.task_count = 2u;
    }
    if (Os_TargetPrepare(&config) != E_OK) {
        return 99;
    }
    StartOS(1u);
    return 99;
}
