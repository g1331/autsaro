/** @file PreTaskHook and PostTaskHook ordering and restricted service calls at transitions. */
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

/* ARTI injects a callable observer; ordinary hook vectors need no observer. */
#ifndef TASK_HOOK_OBSERVER
#define TASK_HOOK_OBSERVER() ((void)0)
#endif

static char trace[128];         /**< Execution-order bytes; fixture-owned, no Init reset. */
static unsigned length;         /**< Used execution-order bytes; fixture-owned, no Init reset. */
static unsigned owner_entries;  /**< Owner entries observations; fixture-owned, no Init reset. */
static unsigned helper_entries; /**< Helper entries observations; fixture-owned, no Init reset. */
static const char *scenario;    /**< Main-owned scenario selection before StartOS; no Init reset. */
static bool configured =
    true;             /**< Main-owned configured selection before StartOS; no Init reset. */
static bool internal; /**< Main-owned internal selection before StartOS; no Init reset. */
static void check(bool condition)
{
    if (!condition) {
        ShutdownOS(E_OS_STATE);
    }
}
static void record(char code)
{
    check(length + 1u < sizeof(trace));
    trace[length++] = code;
    trace[length] = '\0';
}
static void task_hook(char code)
{
    TaskType id = INVALID_TASK;
    TaskStateType state = 99u;
    TASK_HOOK_OBSERVER();
    TickType value = UINT64_MAX;
    Os_ActivationInfo info;
    check(GetTaskID(&id) == E_OK && id < 3u);
    check(GetTaskState(id, &state) == E_OK && state == RUNNING);
    check(Os_TargetInspectActivation(id, &info) == E_OK && info.state == RUNNING);
    if (internal && id == 0u) {
        check(info.internal_held == 1u && info.effective_priority == 20u);
    }
    record(code);
    record((char)('0' + id));
    check(ActivateTask(99u) == E_OS_CALLEVEL);
    check(GetCounterValue(0u, &value) == E_OS_CALLEVEL && value == UINT64_MAX);
    check(GetTaskID(&id) == E_OK && GetTaskState(id, &state) == E_OK && state == RUNNING);
}
void PreTaskHook(void) { task_hook('p'); }
void PostTaskHook(void) { task_hook('q'); }
void StartupHook(void) { check(GetActiveApplicationMode() == 1u); }
void ShutdownHook(StatusType error)
{
    printf("task_hooks scenario=%s configured=%u trace=%s reason=%u\n",
           scenario,
           configured ? 1u : 0u,
           trace,
           error);
}
static void owner(void)
{
    ++owner_entries;
    if (strcmp(scenario, "self-chain") == 0 || strcmp(scenario, "queued") == 0) {
        record(owner_entries == 1u ? 'X' : 'Y');
        if (owner_entries == 1u) {
            if (strcmp(scenario, "self-chain") == 0) {
                (void)ChainTask(0u);
            } else {
                check(ActivateTask(0u) == E_OK);
                taskYIELD();
                (void)TerminateTask();
            }
        } else {
            (void)TerminateTask();
        }
        ShutdownOS(E_OS_STATE);
    }
    if (strcmp(scenario, "shutdown") == 0 || strcmp(scenario, "noop-yield") == 0 ||
        strcmp(scenario, "wait-satisfied") == 0) {
        if (strcmp(scenario, "noop-yield") == 0) {
            taskYIELD();
        } else if (strcmp(scenario, "wait-satisfied") == 0) {
            check(SetEvent(0u, 1u) == E_OK && WaitEvent(1u) == E_OK);
        }
        record('F');
        ShutdownOS(E_OK);
    }
    if (owner_entries == 2u) {
        record('D');
        ShutdownOS(E_OK);
    }
    record('A');
    check(ActivateTask(1u) == E_OK);
    record('B');
    check(WaitEvent(1u) == E_OK);
    record('C');
    check(ClearEvent(1u) == E_OK);
    (void)ChainTask(1u);
    ShutdownOS(E_OS_STATE);
}
static void helper(void)
{
    ++helper_entries;
    record(helper_entries == 1u ? 'H' : 'J');
    (void)TerminateTask();
    ShutdownOS(E_OS_STATE);
}
static void low(void)
{
    TaskStateType state = 99u;
    if (strcmp(scenario, "self-chain") == 0 || strcmp(scenario, "queued") == 0) {
        check(owner_entries == 2u && GetTaskState(0u, &state) == E_OK && state == SUSPENDED);
        record('Z');
        ShutdownOS(E_OK);
    }
    check(GetTaskState(0u, &state) == E_OK && state == WAITING);
    record('L');
    check(SetEvent(0u, 1u) == E_OK);
    check(GetTaskState(0u, &state) == E_OK && state == SUSPENDED);
    record('M');
    check(ActivateTask(0u) == E_OK);
    ShutdownOS(E_OS_STATE);
}
int main(int argc, char **argv)
{
    check(argc == 2);
    scenario = argv[1];
    configured = strcmp(scenario, "unconfigured") != 0;
    internal = strcmp(scenario, "internal") == 0;
    Os_TaskConfig tasks[3] = {
        {0u, "owner", owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "helper", helper, 30u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {2u, "low", low, 5u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    if (strcmp(scenario, "queued") == 0) {
        tasks[0].kind = OS_BASIC_TASK;
        tasks[0].activation_limit = 2u;
    }
    if (internal) {
        tasks[0].internal_resource = 1u;
    }
    const Os_InternalResourceConfig resource = {1u, 20u};
    const Os_HookConfig hooks = {NULL, &PreTaskHook, &PostTaskHook};
    const Os_TargetConfig target = {tasks,
                                    3u,
                                    262144u,
                                    NULL,
                                    0u,
                                    internal ? &resource : NULL,
                                    internal ? 1u : 0u,
                                    0u,
                                    0u,
                                    0u,
                                    NULL,
                                    configured ? &hooks : NULL,
                                    NULL,
                                    0u};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
