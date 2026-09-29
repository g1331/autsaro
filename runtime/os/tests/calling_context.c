#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static bool probed;
static unsigned errors;
static StatusType statuses[29];
static StatusType error_status[64];
static OSServiceIdType error_service[64];
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
static void mask_pairs(void) {
    TaskType unchanged = 42u;
    DisableAllInterrupts();
    check(Os_InterruptDisabled() != 0 && Os_InterruptAllows(4u) == 0 &&
          Os_InterruptAllows(5u) == 0);
    check(GetTaskID(&unchanged) == 9u && unchanged == 42u);
    check(GetActiveApplicationMode() == 0u && GetISRID() == INVALID_ISR);
    check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == 9u && isOsStarted() == FALSE);
    ShutdownOS(8u);
    StartOS(2u);
    EnableAllInterrupts();
    check(Os_InterruptDisabled() == 0);
    SuspendAllInterrupts();
    check(Os_InterruptAllows(4u) == 0 && Os_InterruptAllows(5u) == 0);
    ResumeAllInterrupts();
    check(Os_InterruptDisabled() == 0);
    SuspendOSInterrupts();
    check(Os_InterruptAllows(4u) != 0 && Os_InterruptAllows(5u) == 0);
    ResumeOSInterrupts();
    check(Os_InterruptDisabled() == 0 && Os_InterruptAllows(5u) != 0);
}
static void probe_calls(void) {
    TaskType task = 42u;
    TaskStateType state = 42u;
    EventMaskType event = 42u;
    TickType value = UINT64_MAX;
    TickType elapsed = UINT64_MAX;
    AlarmBaseType base = {61u, 62u, 63u};
    ScheduleTableStatusType table = 42u;
    const bool queries = strcmp(scenario, "pre") == 0 || strcmp(scenario, "post") == 0 ||
                         strncmp(scenario, "error", 5u) == 0;
    probed = true;
    if (strcmp(scenario, "outside-isr") != 0) {
        mask_pairs();
    }
    statuses[0] = GetTaskID(&task);
    statuses[1] = GetTaskState(99u, &state);
    statuses[2] = ActivateTask(99u);
    statuses[3] = TerminateTask();
    statuses[4] = ChainTask(99u);
    statuses[5] = GetResource(99u);
    statuses[6] = ReleaseResource(99u);
    statuses[7] = Schedule();
    statuses[8] = WaitEvent(1u);
    statuses[9] = ClearEvent(1u);
    statuses[10] = SetEvent(99u, 1u);
    statuses[11] = GetEvent(99u, &event);
    statuses[12] = IncrementCounter(99u);
    statuses[13] = GetCounterValue(99u, &value);
    statuses[14] = GetElapsedValue(99u, &value, &elapsed);
    statuses[15] = GetAlarmBase(99u, &base);
    statuses[16] = GetAlarm(99u, &value);
    statuses[17] = SetRelAlarm(99u, 1u, 0u);
    statuses[18] = SetAbsAlarm(99u, 1u, 0u);
    statuses[19] = CancelAlarm(99u);
    statuses[20] = StartScheduleTableRel(99u, 1u);
    statuses[21] = StartScheduleTableAbs(99u, 1u);
    statuses[22] = StopScheduleTable(99u);
    statuses[23] = NextScheduleTable(99u, 98u);
    statuses[24] = GetScheduleTableStatus(99u, &table);
    statuses[25] = DisableInterruptSource(99u);
    statuses[26] = EnableInterruptSource(99u, TRUE);
    statuses[27] = ClearPendingInterrupt(99u);
    statuses[28] = ControlIdle(UINT16_MAX, IDLE_NO_HALT);
    check(isOsStarted() == TRUE);
    check(task == (queries ? 0u : 42u) && state == 42u && event == 42u);
    check(value == UINT64_MAX && elapsed == UINT64_MAX && table == 42u);
    check(base.maxallowedvalue == 61u && base.ticksperbase == 62u && base.mincycle == 63u);
    check(GetActiveApplicationMode() ==
          ((strcmp(scenario, "alarm-callback") == 0 || strcmp(scenario, "outside-isr") == 0) ? 0u
                                                                                             : 1u));
    check(GetISRID() == INVALID_ISR);
    StartOS(2u);
    if (strcmp(scenario, "pre") == 0 || strcmp(scenario, "post") == 0 ||
        strcmp(scenario, "alarm-callback") == 0 || strcmp(scenario, "shutdown") == 0 ||
        strcmp(scenario, "outside-isr") == 0) {
        ShutdownOS(8u);
    }
}
void ErrorHook(StatusType error) {
    const OSServiceIdType service = OSErrorGetServiceId();
    check(errors < 64u && GetActiveApplicationMode() == 1u && GetISRID() == INVALID_ISR &&
          isOsStarted() == TRUE);
    error_status[errors] = error;
    error_service[errors++] = service;
    if (!probed && strncmp(scenario, "error", 5u) == 0) {
        probe_calls();
    }
    check(OSErrorGetServiceId() == service);
    if (strcmp(scenario, "error-shutdown") == 0) {
        ShutdownOS(8u);
    }
}
static void pre(void) {
    if (!probed && strcmp(scenario, "pre") == 0) {
        probe_calls();
    }
}
static void post(void) {
    if (!probed && strcmp(scenario, "post") == 0) {
        probe_calls();
    }
}
static uint32_t outside_isr(void) {
    /* Boundary fault injection on a real ISR stack, without a logical ISR. */
    Os_BackendInterruptLeave();
    probe_calls();
    Os_BackendInterruptEnter(5u);
    return 0u;
}
void StartupHook(void) {
    vPortSetInterruptHandler(5u, &outside_isr);
    if (strncmp(scenario, "startup", 7u) == 0) {
        probe_calls();
        if (strcmp(scenario, "startup-shutdown") == 0) {
            ShutdownOS(8u);
        }
    }
}
void ShutdownHook(StatusType error) {
    if (!probed && strcmp(scenario, "shutdown") == 0) {
        probe_calls();
    }
    check(probed);
    printf("calling scenario=%s errors=%u reason=%u statuses=", scenario, errors, error);
    for (unsigned i = 0u; i < 29u; ++i) {
        printf("%s%u", i != 0u ? "," : "", statuses[i]);
    }
    printf("\n");
    for (unsigned i = 0u; i < errors; ++i) {
        printf("matrix_error status=%u service=%u\n", error_status[i], error_service[i]);
    }
}
static void owner(void) {
    if (strncmp(scenario, "error", 5u) == 0) {
        check(ActivateTask(99u) == E_OS_ID);
    } else if (strcmp(scenario, "outside-isr") == 0) {
        vPortGenerateSimulatedInterrupt(5u);
    } else if (strcmp(scenario, "alarm-callback") == 0) {
        check(IncrementCounter(0u) == E_OK);
    } else if (strcmp(scenario, "post") == 0) {
        (void)TerminateTask();
        Os_BackendShutdown(E_OS_STATE);
    }
    ShutdownOS(E_OK);
}
static void monitor(void) { ShutdownOS(E_OK); }
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    const Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "monitor", monitor, 5u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_HookConfig hooks = {&ErrorHook, &pre, &post};
    const Os_CounterConfig counter = {0u, 1000u, 1u, 1u, 1u};
    const Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_CALLBACK, 0u, 0u, &probe_calls, 1u, 0u, 1u,
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
                                    strcmp(scenario, "alarm-callback") == 0 ? &time : NULL,
                                    &hooks};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
