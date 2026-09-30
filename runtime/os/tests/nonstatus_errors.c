#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static unsigned errors;
static unsigned probes;
static bool configured;
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
static void scalar_probe(void) {
    const unsigned before = errors;
    const bool alarm = strcmp(scenario, "alarm") == 0;
    ++probes;
    DisableAllInterrupts();
    check(GetActiveApplicationMode() == 0u);
    check(GetISRID() == INVALID_ISR);
    check(isOsStarted() == FALSE);
    StartOS(2u);
    ShutdownOS(8u);
    check(errors == before && Os_InterruptDisabled() != 0);
    EnableAllInterrupts();
    check(GetActiveApplicationMode() == (alarm ? 0u : 1u));
    check(isOsStarted() == TRUE);
    check(GetISRID() == (strcmp(scenario, "isr") == 0 ? 5u : INVALID_ISR));
    StartOS(2u);
    if (strcmp(scenario, "pre") == 0 || strcmp(scenario, "post") == 0 || alarm ||
        strcmp(scenario, "shutdown") == 0) {
        ShutdownOS(8u);
    }
    check(errors == before);
}
void ErrorHook(StatusType error) {
    const OSServiceIdType service = OSErrorGetServiceId();
    check(error == E_OS_ID && service == 130u && OSError_ActivateTask_TaskID() == 99u);
    ++errors;
    check(errors == 1u);
    if (strcmp(scenario, "error") == 0) {
        scalar_probe();
        TaskStateType state = 42u;
        check(GetTaskState(99u, &state) == E_OS_ID && state == 42u);
    }
    check(OSErrorGetServiceId() == service && OSError_ActivateTask_TaskID() == 99u);
}
static uint32_t interrupt(void) {
    scalar_probe();
    return 0u;
}
static void pre(void) {
    if (probes == 0u && strcmp(scenario, "pre") == 0) {
        scalar_probe();
    }
}
static void post(void) {
    if (probes == 0u && strcmp(scenario, "post") == 0) {
        scalar_probe();
    }
}
void StartupHook(void) {
    vPortSetInterruptHandler(5u, &interrupt);
    vPortSetInterruptHandler(4u, &interrupt);
    if (strcmp(scenario, "startup") == 0) {
        scalar_probe();
    }
}
void ShutdownHook(StatusType reason) {
    if (reason != E_OK) {
        /* Preserve the first assertion failure and let native shutdown exit;
         * requesting shutdown again from this hook would wait forever. */
        printf("nonstatus_failed scenario=%s probes=%u errors=%u reason=%u\n", scenario, probes,
               errors, reason);
        return;
    }
    if (strcmp(scenario, "shutdown") == 0) {
        scalar_probe();
    }
    check(probes == 1u && errors == (configured ? 1u : 0u));
    printf("nonstatus scenario=%s probes=%u errors=%u snapshot=pass reason=%u\n", scenario, probes,
           errors, reason);
}
static void owner(void) {
    if (strcmp(scenario, "task") == 0 || strcmp(scenario, "unconfigured") == 0) {
        scalar_probe();
    } else if (strcmp(scenario, "isr") == 0 || strcmp(scenario, "cat1") == 0) {
        vPortGenerateSimulatedInterrupt(strcmp(scenario, "cat1") == 0 ? 4u : 5u);
    } else if (strcmp(scenario, "alarm") == 0) {
        check(IncrementCounter(0u) == E_OK);
    }
    check(ActivateTask(99u) == E_OS_ID);
    if (strcmp(scenario, "post") == 0) {
        (void)TerminateTask();
        Os_BackendShutdown(E_OS_STATE);
    }
    ShutdownOS(E_OK);
}
static void monitor(void) { ShutdownOS(E_OK); }
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    configured = strcmp(scenario, "unconfigured") != 0;
    const Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "monitor", monitor, 5u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_HookConfig hooks = {configured ? &ErrorHook : NULL, &pre, &post};
    const Os_CounterConfig counter = {0u, 1000u, 1u, 1u, 1u};
    const Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_CALLBACK, 0u, 0u, &scalar_probe, 1u, 0u, 1u,
                                  0u, 0u};
    const Os_TimeConfig time = {&counter, 1u, &alarm, 1u, 0u, INVALID_TASK, 0u, NULL, NULL, 0u};
    const Os_TargetConfig target = {tasks,
                                    2u,
                                    262144u,
                                    NULL,
                                    0u,
                                    NULL,
                                    0u,
                                    UINT32_C(1) << 4u,
                                    0u,
                                    0u,
                                    strcmp(scenario, "alarm") == 0 ? &time : NULL,
                                    &hooks,
                                    NULL};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
