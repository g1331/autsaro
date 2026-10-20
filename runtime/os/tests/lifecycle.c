/** @file StartOS and ShutdownOS lifecycle: configuration rejection, autostart and hooks. */
#include "Os_Target.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <io.h>
#else
#include <unistd.h>
#endif
static const uint8 task_name_0[] = "A"; /**< Process-lifetime Task name bytes. */
static const uint8 task_name_1[] = "B"; /**< Process-lifetime Task name bytes. */

static unsigned
    startup_failure; /**< Main-owned startup failure selection before StartOS; no Init reset. */
static unsigned both_ready; /**< Startup readiness observations; fixture-owned, no Init reset. */
void StartupHook(void)
{
    Os_TargetTrace('S');
    if (startup_failure != 0u) {
        if (startup_failure == 2u) {
            StartOS(1u);
            /* R24-11 wrong-context StartOS is ignored and returns. The
             * original initialization must not be entered a second time. */
            if (GetActiveApplicationMode() != 1u || isOsStarted() != TRUE) {
                Os_TargetTrace('X');
            }
            Os_TargetTrace('R');
        }
        ShutdownOS(E_OS_STATE);
    }
}
void ShutdownHook(StatusType Error)
{
    (void)Error;
    if (Os_TargetReady()) {
        Os_TargetTrace('X');
    }
    Os_TargetTrace('D');
}
static void check_task(TaskType id, TaskType other, char marker)
{
    TaskStateType state = 99u;
    if (!Os_TargetReady() || GetTaskState(id, &state) != E_OK || state != RUNNING ||
        GetTaskState(other, &state) != E_OK || state != (both_ready ? READY : SUSPENDED) ||
        GetTaskState(99u, &state) != E_OS_ID || state != (both_ready ? READY : SUSPENDED) ||
        GetTaskState(id, NULL) != E_OS_ILLEGAL_ADDRESS) {
        ShutdownOS(E_OS_STATE);
    }
    Os_TargetTrace(marker);
    ShutdownOS(E_OK);
    Os_TargetTrace('X');
}
static void task_a(void) { check_task(0u, 1u, 'A'); }
static void task_b(void) { check_task(1u, 0u, 'B'); }
int main(int argc, char **argv)
{
    Os_TaskConfigType tasks[2] = {
        {0u, task_name_0, task_a, 2u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, task_name_1, task_b, 2u, 2u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_TargetConfigType config = {
        tasks, 2u, 262144u, NULL, 0u, NULL, 0u, 0u, 0u, 0u, NULL, NULL, NULL, 0u};
    Os_CounterConfigType counter = {0u, 7u, 1u, 1u, 1u};
    Os_AlarmConfigType alarm = {0u, 0u, OS_ALARM_ACTIVATE, 0u, 0u, NULL, 0u, 0u, 0u, 0u, 0u};
    Os_ExpiryActionType action = {OS_ALARM_ACTIVATE, 0u, 0u};
    Os_ExpiryPointType point = {0u, &action, 1u};
    Os_ScheduleTableConfigType table = {
        0u, 0u, 2u, &point, 1u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 0u};
    Os_TimeConfigType time = {&counter, 1u, &alarm, 1u, 0u, INVALID_TASK, 0u, NULL, &table, 1u};
    Os_ResourceConfigType resource = {0u, 2u, 1u, 0u, 0u, 0u};
    Os_InternalResourceConfigType internal = {1u, 2u};
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
    /* Verifies: SWS_BSW_00120 [Os]. Each independent count keeps the full uint32
     * input until its own capacity check, before any actor or table traversal. */
    if (strncmp(scenario, "count-", 6u) == 0) {
        uint32 *counts[] = {&config.task_count,
                            &config.resource_count,
                            &config.internal_resource_count,
                            &time.counter_count,
                            &time.alarm_count,
                            &time.schedule_table_count,
                            &table.point_count,
                            &point.action_count};
        const char *names[] = {
            "task", "resource", "internal", "counter", "alarm", "table", "point", "action"};
        config.time = &time;
        config.resources = &resource;
        config.internal_resources = &internal;
        for (size_t i = 0u; i < 8u; ++i) {
            char wide[40], huge[40];
            if (snprintf(wide, sizeof(wide), "count-wide-%s", names[i]) < 0 ||
                snprintf(huge, sizeof(huge), "count-huge-%s", names[i]) < 0) {
                return 99;
            }
            if (strcmp(scenario, wide) == 0 || strcmp(scenario, huge) == 0) {
                *counts[i] = strcmp(scenario, wide) == 0 ? 256u : UINT32_MAX;
                expected = E_OS_VALUE;
            }
        }
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
        if (GetTaskState(0u, &state) != E_OS_CALLEVEL || state != 99u) {
            return 99;
        }
    }
    if ((argc > 1) && (strcmp(argv[1], "output-failure") == 0)) {
#ifdef _WIN32
        if (_close(_fileno(stdout)) != 0) {
#else
        if (close(fileno(stdout)) != 0) {
#endif
            return 99;
        }
    }
    StartOS(mode);
    puts("FORBIDDEN_STARTOS_RETURN");
    return 99;
}
