#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static unsigned errors;
static unsigned probes;
static unsigned helpers;
static StatusType statuses[3];
static char trace[32];
static unsigned length;
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
static void record(char marker) {
    check(length + 1u < sizeof(trace));
    trace[length++] = marker;
    trace[length] = '\0';
}
void ErrorHook(StatusType error) {
    TaskType task = INVALID_TASK;
    TaskStateType state = SUSPENDED;
    check(errors < 3u && OSErrorGetServiceId() == 252u && GetISRID() == 5u);
    check(GetTaskID(&task) == E_OK && task == 0u);
    check(GetTaskState(task, &state) == E_OK && state == RUNNING);
    check(Os_InterruptDisabled() == 0);
    if (error == 9u) {
        check(Os_BackendInterruptEnabled(6u) == 0);
        record('D');
    } else {
        check(error == 6u && Os_BackendInterruptEnabled(6u) != 0);
        record('R');
    }
    statuses[errors++] = error;
}
static void acquire_release(void) {
    const unsigned count = strcmp(scenario, "maximum") == 0 ? 7u : 2u;
    for (unsigned resource = 0u; resource < count; ++resource) {
        check(GetResource((ResourceType)resource) == E_OK);
    }
    for (unsigned resource = count; resource > 0u; --resource) {
        check(ReleaseResource((ResourceType)(resource - 1u)) == E_OK);
    }
}
static uint32_t probe(void) {
    check(GetISRID() == 6u);
    acquire_release();
    ++probes;
    record('P');
    return 0u;
}
static uint32_t leaking_isr(void) {
    check(GetISRID() == 5u);
    record('I');
    check(GetResource(0u) == E_OK);
    if (strcmp(scenario, "single") != 0) {
        const unsigned count = strcmp(scenario, "maximum") == 0 ? 7u : 2u;
        for (unsigned resource = 1u; resource < count; ++resource) {
            check(GetResource((ResourceType)resource) == E_OK);
        }
    }
    check(Os_BackendInterruptEnabled(6u) == 0);
    check(ActivateTask(1u) == E_OK);
    vPortGenerateSimulatedInterruptFromWindowsThread(6u);
    if (strcmp(scenario, "balanced") == 0) {
        check(ReleaseResource(1u) == E_OK);
        check(ReleaseResource(0u) == E_OK);
    } else if (strcmp(scenario, "disable") == 0) {
        DisableAllInterrupts();
    } else if (strcmp(scenario, "all") == 0) {
        SuspendAllInterrupts();
        SuspendAllInterrupts();
    } else if (strcmp(scenario, "os") == 0) {
        SuspendOSInterrupts();
        SuspendOSInterrupts();
    } else if (strcmp(scenario, "mixed") == 0 || strcmp(scenario, "unconfigured-mixed") == 0) {
        DisableAllInterrupts();
        SuspendAllInterrupts();
        SuspendOSInterrupts();
    }
    return 0u;
}
void StartupHook(void) {
    vPortSetInterruptHandler(5u, &leaking_isr);
    vPortSetInterruptHandler(6u, &probe);
}
void ShutdownHook(StatusType error) {
    printf("isr_cleanup scenario=%s probes=%u helpers=%u errors=%u first=%u last=%u trace=%s "
           "reason=%u\n",
           scenario, probes, helpers, errors, errors != 0u ? statuses[0] : 0u,
           errors != 0u ? statuses[errors - 1u] : 0u, trace, error);
}
static void helper(void) {
    check(GetISRID() == INVALID_ISR && Os_InterruptDisabled() == 0 && probes == 1u);
    acquire_release();
    ++helpers;
    record('H');
    (void)TerminateTask();
    Os_BackendShutdown(E_OS_STATE);
}
static void owner(void) {
    record('A');
    vPortGenerateSimulatedInterrupt(5u);
    check(GetISRID() == INVALID_ISR && probes == 1u && helpers == 1u);
    acquire_release();
    record('B');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    const Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "helper", helper, 20u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_ResourceConfig resources[7] = {
        {0u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {1u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {2u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {3u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {4u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {5u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
        {6u, 31u, 3u, (UINT32_C(1) << 5u) | (UINT32_C(1) << 6u)},
    };
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfig target = {tasks,
                                    2u,
                                    262144u,
                                    resources,
                                    strcmp(scenario, "maximum") == 0 ? 7u : 2u,
                                    NULL,
                                    0u,
                                    0u,
                                    0u,
                                    0u,
                                    NULL,
                                    strncmp(scenario, "unconfigured", 12u) == 0 ? NULL : &hooks};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
