#include "Os_Target.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static unsigned startup_failure;
static unsigned both_ready;
void StartupHook(void) {
    Os_TargetTrace('S');
    if (startup_failure != 0u) {
        if (startup_failure == 2u) {
            StartOS(1u);
        }
        ShutdownOS(E_OS_STATE);
    }
}
void ShutdownHook(StatusType Error) {
    (void)Error;
    if (Os_TargetReady()) {
        Os_TargetTrace('X');
    }
    Os_TargetTrace('D');
}
static void check_task(TaskType id, TaskType other, char marker) {
    TaskStateType state = 99u;
    if (!Os_TargetReady() || GetTaskState(id, &state) != E_OK || state != RUNNING ||
        GetTaskState(other, &state) != E_OK || state != (both_ready ? READY : SUSPENDED) ||
        GetTaskState(99u, &state) != E_OS_ID || state != (both_ready ? READY : SUSPENDED) ||
        GetTaskState(id, NULL) != E_OS_VALUE) {
        ShutdownOS(E_OS_STATE);
    }
    Os_TargetTrace(marker);
    ShutdownOS(E_OK);
    Os_TargetTrace('X');
}
static void task_a(void) { check_task(0u, 1u, 'A'); }
static void task_b(void) { check_task(1u, 0u, 'B'); }
int main(int argc, char **argv) {
    Os_TaskConfig tasks[2] = {{0u, "A", task_a, 2u, 1u}, {1u, "B", task_b, 2u, 2u}};
    Os_TargetConfig config = {tasks, 2u, 262144u};
    AppModeType mode = 1u;
    StatusType expected = E_OK;
    const char *scenario = argc == 2 ? argv[1] : "normal";
    if (strcmp(scenario, "mode2") == 0) {
        mode = 2u;
    } else if (strcmp(scenario, "priority") == 0) {
        both_ready = 1u;
        tasks[1].autostart_modes = 1u;
        tasks[1].priority = 3u;
    } else if (strcmp(scenario, "bad-mode") == 0) {
        mode = 3u;
    } else if (strcmp(scenario, "zero-count") == 0) {
        config.task_count = 0u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "capacity") == 0) {
        config.task_count = OS_MAX_TASKS + 1u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "duplicate") == 0) {
        tasks[1].id = 0u;
        expected = E_OS_ID;
    } else if (strcmp(scenario, "bad-id") == 0) {
        tasks[0].id = OS_MAX_TASKS;
        expected = E_OS_ID;
    } else if (strcmp(scenario, "zero-priority") == 0) {
        tasks[0].priority = 0u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-priority") == 0) {
        tasks[0].priority = OS_MAX_PRIORITY + 1u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "null-entry") == 0) {
        tasks[0].entry = NULL;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-stack") == 0) {
        config.host_stack_reserve = 65537u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "small-stack") == 0) {
        config.host_stack_reserve = 1u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "large-stack") == 0) {
        config.host_stack_reserve = 16777217u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "null-tasks") == 0) {
        config.tasks = NULL;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-autostart") == 0) {
        tasks[0].autostart_modes = 4u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "startup-failure") == 0) {
        startup_failure = 1u;
    } else if (strcmp(scenario, "repeat-start") == 0) {
        startup_failure = 2u;
    }
    if (Os_TargetPrepare(NULL) != E_OS_VALUE || Os_TargetReady()) {
        return 99;
    }
    if (Os_TargetPrepare(&config) != expected) {
        return 99;
    }
    if (expected != E_OK) {
        printf("state=Failed reason=%u rejected_before_threads=1\n", expected);
        return expected;
    }
    if (Os_TargetPrepare(&config) != E_OS_STATE) {
        return 99;
    }
    {
        TaskStateType state = 99u;
        if (GetTaskState(0u, &state) != E_OS_STATE || state != 99u) {
            return 99;
        }
    }
    StartOS(mode);
    puts("FORBIDDEN_STARTOS_RETURN");
    return 99;
}
