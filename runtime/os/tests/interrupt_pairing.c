#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static unsigned category1;
static unsigned category2;
static unsigned source_deliveries;
static TaskType isr_task = INVALID_TASK;
static unsigned errors;
static unsigned startup;
static StatusType statuses[40];
static OSServiceIdType services[40];
static char actors[40];
static char trace[40];
static unsigned length;
static TaskType output_task = 42u;
static TaskStateType output_state = 42u;
static EventMaskType output_event = UINT32_C(0xdead);
static TickType output_value = UINT64_MAX;
static TickType output_elapsed = UINT64_MAX;
static AlarmBaseType output_base = {61u, 62u, 63u};
static ScheduleTableStatusType output_table = 42u;
static void check(bool condition) {
    if (!condition) {
        /* A failing harness assertion must still stop while public shutdown
         * is correctly ignored inside an application-disabled region. */
        Os_BackendShutdown(E_OS_STATE);
    }
}
static void record(char marker) {
    check(length + 1u < sizeof(trace));
    trace[length++] = marker;
    trace[length] = '\0';
}
void ErrorHook(StatusType error) {
    TaskType caller = INVALID_TASK;
    TaskStateType state = 42u;
    const OSServiceIdType service = OSErrorGetServiceId();
    check(errors < 40u && GetTaskID(&caller) == E_OK);
    if (startup == 0u) {
        check(caller == INVALID_TASK);
    } else {
        check(caller == 0u && GetTaskState(caller, &state) == E_OK && state == RUNNING);
        check(GetActiveApplicationMode() == 1u);
    }
    check(GetISRID() == ((Os_StackCurrent()->role == 'S' && Os_BackendCurrentInterrupt() == 5u)
                             ? 5u
                             : INVALID_ISR));
    if (service == OSServiceId_ShutdownOS) {
        check(OSError_ShutdownOS_Error() == 8u);
    } else if (service == OSServiceId_StartOS) {
        check(OSError_StartOS_Mode() == 2u);
    } else if (service == OSServiceId_EnableInterruptSource) {
        check(OSError_EnableInterruptSource_ISRID() == 99u ||
              OSError_EnableInterruptSource_ISRID() == 6u);
        check(OSError_EnableInterruptSource_ClearPending() == TRUE);
    } else if (service == OSServiceId_DisableInterruptSource) {
        check(OSError_DisableInterruptSource_ISRID() == 99u ||
              OSError_DisableInterruptSource_ISRID() == 1u ||
              OSError_DisableInterruptSource_ISRID() == 4u ||
              OSError_DisableInterruptSource_ISRID() == 6u ||
              OSError_DisableInterruptSource_ISRID() == 8u ||
              OSError_DisableInterruptSource_ISRID() == 7u);
    } else if (service == OSServiceId_ClearPendingInterrupt) {
        check(OSError_ClearPendingInterrupt_ISRID() == 99u ||
              OSError_ClearPendingInterrupt_ISRID() == 7u);
    }
    statuses[errors] = error;
    services[errors] = service;
    actors[errors] = Os_StackCurrent()->role;
    ++errors;
    /* Mask services are forbidden in ErrorHook and cannot disturb the outer
     * snapshot or the Task's saved state. No nested error callback occurs. */
    SuspendAllInterrupts();
    EnableAllInterrupts();
    check(OSErrorGetServiceId() == service);
}
static uint32_t cat1(void) {
    check(GetISRID() == INVALID_ISR);
    if (strcmp(scenario, "isr-balanced") == 0) {
        SuspendAllInterrupts();
        SuspendOSInterrupts();
        ResumeOSInterrupts();
        ResumeAllInterrupts();
    }
    ++category1;
    record('C');
    return 0u;
}
static uint32_t cat2(void) {
    check(GetISRID() == 5u);
    ++category2;
    check(GetTaskID(&isr_task) == E_OK);
    record('O');
    if (strcmp(scenario, "isr-balanced") == 0) {
        DisableAllInterrupts();
        EnableAllInterrupts();
        SuspendAllInterrupts();
        SuspendOSInterrupts();
        ResumeAllInterrupts();
        ResumeOSInterrupts();
    } else if (strcmp(scenario, "source-isr") == 0) {
        check(DisableInterruptSource(6u) == E_OK);
        vPortGenerateSimulatedInterruptFromWindowsThread(6u);
        check(ClearPendingInterrupt(6u) == E_OK);
        check(EnableInterruptSource(6u, FALSE) == E_OK);
        check(DisableInterruptSource(7u) == E_OK);
        vPortGenerateSimulatedInterruptFromWindowsThread(7u);
        check(EnableInterruptSource(7u, TRUE) == E_OK);
        check(DisableInterruptSource(8u) == E_OK);
        vPortGenerateSimulatedInterruptFromWindowsThread(8u);
        check(EnableInterruptSource(8u, FALSE) == E_OK);
        check(source_deliveries == 0u && GetISRID() == 5u);
    } else if (strcmp(scenario, "source-outside-isr") == 0) {
        /* Deliberate boundary fault injection on the actual ISR stack: a
         * registered S actor with no active logical interrupt is not Cat2. */
        vPortGenerateSimulatedInterruptFromWindowsThread(6u);
        vPortGenerateSimulatedInterruptFromWindowsThread(7u);
        Os_BackendInterruptLeave();
        check(DisableInterruptSource(8u) == E_OS_CALLEVEL);
        check(Os_PortInterruptSourceEnabled(8u) != 0);
        check(EnableInterruptSource(6u, TRUE) == E_OS_CALLEVEL);
        check(Os_PortInterruptSourceEnabled(6u) != 0);
        check(ClearPendingInterrupt(7u) == E_OS_CALLEVEL);
        check(Os_PortInterruptSourceEnabled(7u) != 0);
        Os_BackendInterruptEnter(5u);
    } else if (strcmp(scenario, "isr-leak-disable") == 0) {
        DisableAllInterrupts();
    } else if (strcmp(scenario, "isr-leak-all") == 0) {
        SuspendAllInterrupts();
        SuspendAllInterrupts();
    } else if (strcmp(scenario, "isr-leak-os") == 0) {
        SuspendOSInterrupts();
        SuspendOSInterrupts();
    } else if (strcmp(scenario, "isr-leak-mixed") == 0) {
        DisableAllInterrupts();
        SuspendAllInterrupts();
        SuspendOSInterrupts();
    }
    return 0u;
}
static uint32_t source_isr(void) {
    const ISRType id = GetISRID();
    check(strcmp(scenario, "source-isr") == 0 ? id == 8u : (id == 6u || id == 7u));
    ++source_deliveries;
    record(strcmp(scenario, "source-isr") == 0 ? 'Q' : (id == 6u ? 'D' : 'E'));
    return 0u;
}
static void pend(void) {
    vPortGenerateSimulatedInterrupt(4u);
    vPortGenerateSimulatedInterrupt(5u);
}
void StartupHook(void) {
    if (strcmp(scenario, "hook-reject") == 0) {
        EnableAllInterrupts();
        DisableAllInterrupts();
        ResumeAllInterrupts();
        SuspendAllInterrupts();
        ResumeOSInterrupts();
        SuspendOSInterrupts();
        check(errors == 6u && Os_InterruptDisabled() == 0);
    }
    if (strcmp(scenario, "source-hook-reject") == 0) {
        check(DisableInterruptSource(99u) == E_OS_CALLEVEL);
        check(EnableInterruptSource(99u, TRUE) == E_OS_CALLEVEL);
        check(ClearPendingInterrupt(99u) == E_OS_CALLEVEL);
        check(errors == 3u);
    }
    vPortSetInterruptHandler(4u, &cat1);
    vPortSetInterruptHandler(5u, &cat2);
    vPortSetInterruptHandler(6u, &source_isr);
    if (strcmp(scenario, "source-isr") == 0 || strcmp(scenario, "source-outside-isr") == 0) {
        vPortSetInterruptHandler(7u, &source_isr);
        vPortSetInterruptHandler(8u, &source_isr);
    }
    startup = 1u;
}
void ShutdownHook(StatusType error) {
    /* These four remain usable after close without a port-mutex dependency. */
    DisableAllInterrupts();
    SuspendAllInterrupts();
    EnableAllInterrupts();
    ResumeAllInterrupts();
    check(Os_InterruptDisabled() == 0);
    printf("pairing scenario=%s startup=%u cat1=%u cat2=%u isr_task=%u errors=%u trace=%s "
           "reason=%u\n",
           scenario, startup, category1, category2, isr_task, errors, trace, error);
    for (unsigned i = 0u; i < errors; ++i) {
        printf("mask_error status=%u service=%u actor=%c\n", statuses[i], services[i], actors[i]);
    }
}
static void begin_mask(void) {
    if (strcmp(scenario, "services-disable") == 0 || strcmp(scenario, "missing-disable") == 0) {
        DisableAllInterrupts();
    } else if (strcmp(scenario, "services-all") == 0 || strcmp(scenario, "missing-all") == 0) {
        SuspendAllInterrupts();
        SuspendAllInterrupts();
    } else {
        SuspendOSInterrupts();
        SuspendOSInterrupts();
    }
}
static void end_mask(void) {
    if (strcmp(scenario, "services-disable") == 0) {
        EnableAllInterrupts();
    } else if (strcmp(scenario, "services-all") == 0) {
        ResumeAllInterrupts();
        ResumeAllInterrupts();
    } else {
        ResumeOSInterrupts();
        ResumeOSInterrupts();
    }
}
static void refused_services(void) {
    check(GetTaskID(&output_task) == 9u);
    check(GetTaskState(99u, &output_state) == 9u);
    check(ActivateTask(99u) == 9u);
    check(TerminateTask() == 9u);
    check(ChainTask(99u) == 9u);
    check(GetResource(99u) == 9u);
    check(ReleaseResource(99u) == 9u);
    check(Schedule() == 9u);
    check(WaitEvent(1u) == 9u);
    check(ClearEvent(1u) == 9u);
    check(SetEvent(99u, 1u) == 9u);
    check(GetEvent(99u, &output_event) == 9u);
    check(IncrementCounter(99u) == 9u);
    check(GetCounterValue(99u, &output_value) == 9u);
    check(GetElapsedValue(99u, &output_value, &output_elapsed) == 9u);
    check(GetAlarmBase(99u, &output_base) == 9u);
    check(GetAlarm(99u, &output_value) == 9u);
    check(SetRelAlarm(99u, 1u, 0u) == 9u);
    check(SetAbsAlarm(99u, UINT64_C(1) << 40u, 0u) == 9u);
    check(CancelAlarm(99u) == 9u);
    check(StartScheduleTableRel(99u, 1u) == 9u);
    check(StartScheduleTableAbs(99u, 1u) == 9u);
    check(StopScheduleTable(99u) == 9u);
    check(NextScheduleTable(99u, 98u) == 9u);
    check(GetScheduleTableStatus(99u, &output_table) == 9u);
    check(GetActiveApplicationMode() == 0u);
    check(GetISRID() == INVALID_ISR);
    ShutdownOS(8u);
    StartOS(2u);
    check(errors == 29u && output_task == 42u && output_state == 42u &&
          output_event == UINT32_C(0xdead) && output_value == UINT64_MAX &&
          output_elapsed == UINT64_MAX && output_table == 42u);
    check(output_base.maxallowedvalue == 61u && output_base.ticksperbase == 62u &&
          output_base.mincycle == 63u);
}
static void owner(void) {
    if (strncmp(scenario, "source-", 7u) == 0) {
        if (strcmp(scenario, "source-isr") == 0 || strcmp(scenario, "source-outside-isr") == 0) {
            vPortGenerateSimulatedInterrupt(5u);
            check(source_deliveries == (strcmp(scenario, "source-isr") == 0 ? 1u : 2u));
            vPortGenerateSimulatedInterrupt(4u);
        } else if (strcmp(scenario, "source-invalid") == 0) {
            check(DisableInterruptSource(99u) == E_OS_ID);
            check(EnableInterruptSource(99u, TRUE) == E_OS_ID);
            check(ClearPendingInterrupt(99u) == E_OS_ID);
            check(DisableInterruptSource(1u) == E_OS_ID);
            check(DisableInterruptSource(4u) == E_OS_ID);
            check(DisableInterruptSource(7u) == E_OS_ID);
            check(errors == 6u);
            pend();
        } else if (strcmp(scenario, "source-hook-reject") == 0) {
            pend();
        } else {
            check(DisableInterruptSource(5u) == E_OK);
            check(DisableInterruptSource(5u) == E_OK);
            vPortGenerateSimulatedInterrupt(5u);
            check(category2 == 0u);
            if (strcmp(scenario, "source-clear") == 0) {
                check(ClearPendingInterrupt(5u) == E_OK);
            } else if (strcmp(scenario, "source-global") == 0) {
                SuspendAllInterrupts();
            }
            check(EnableInterruptSource(5u, strcmp(scenario, "source-enable-clear") == 0) == E_OK);
            if (strcmp(scenario, "source-global") == 0) {
                check(category2 == 0u);
                ResumeAllInterrupts();
            }
            if (strcmp(scenario, "source-clear") == 0 ||
                strcmp(scenario, "source-enable-clear") == 0) {
                check(category2 == 0u);
                vPortGenerateSimulatedInterrupt(5u);
            }
            check(category2 == 1u);
            vPortGenerateSimulatedInterrupt(4u);
        }
    } else if (strncmp(scenario, "services-", 9u) == 0) {
        begin_mask();
        refused_services();
        end_mask();
        check(Os_InterruptDisabled() == 0 && GetTaskID(&output_task) == E_OK && output_task == 0u);
        pend();
    } else if (strncmp(scenario, "missing-", 8u) == 0) {
        if (strcmp(scenario, "missing-mixed") == 0) {
            SuspendOSInterrupts();
            SuspendAllInterrupts();
            DisableAllInterrupts();
        } else {
            begin_mask();
        }
        /* Leave real pending edges for automatic missing-end restoration. */
        vPortGenerateSimulatedInterrupt(5u);
        return;
    } else if (strcmp(scenario, "disable") == 0) {
        DisableAllInterrupts();
        DisableAllInterrupts();
        pend();
        check(category1 == 0u && category2 == 0u);
        EnableAllInterrupts();
    } else if (strcmp(scenario, "all-nested") == 0) {
        SuspendAllInterrupts();
        SuspendAllInterrupts();
        pend();
        ResumeAllInterrupts();
        ResumeOSInterrupts();
        check(category1 == 0u && category2 == 0u);
        record('N');
        ResumeAllInterrupts();
    } else if (strcmp(scenario, "os-nested") == 0) {
        SuspendOSInterrupts();
        SuspendOSInterrupts();
        pend();
        ResumeOSInterrupts();
        ResumeAllInterrupts();
        check(category1 == 1u && category2 == 0u);
        record('N');
        ResumeOSInterrupts();
    } else if (strcmp(scenario, "os-all") == 0) {
        SuspendOSInterrupts();
        SuspendAllInterrupts();
        pend();
        ResumeAllInterrupts();
        check(category1 == 1u && category2 == 0u);
        ResumeOSInterrupts();
    } else if (strcmp(scenario, "all-os") == 0) {
        SuspendAllInterrupts();
        SuspendOSInterrupts();
        pend();
        ResumeOSInterrupts();
        check(category1 == 0u && category2 == 0u);
        ResumeAllInterrupts();
    } else {
        EnableAllInterrupts();
        ResumeAllInterrupts();
        ResumeOSInterrupts();
        pend();
    }
    check(category1 == 1u && category2 == 1u && isr_task == 0u);
    check(GetISRID() == INVALID_ISR && Os_InterruptDisabled() == 0);
    check(Os_InterruptAllows(4u) != 0 && Os_InterruptAllows(5u) != 0);
    ShutdownOS(E_OK);
}
static void monitor(void) {
    TaskType id = INVALID_TASK;
    TaskStateType state = 42u;
    check(strncmp(scenario, "missing-", 8u) == 0 && errors == 1u);
    check(category1 == 0u && category2 == 1u && isr_task == INVALID_TASK);
    check(GetTaskID(&id) == E_OK && id == 1u && GetTaskState(0u, &state) == E_OK &&
          state == SUSPENDED);
    check(Os_InterruptDisabled() == 0 && Os_InterruptAllows(4u) != 0 &&
          Os_InterruptAllows(5u) != 0);
    record('M');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    const Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "monitor", monitor, 5u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfig target = {tasks, 2u, 262144u, NULL,  0u, NULL, 0u, UINT32_C(1) << 4u,
                                    0u,    0u, NULL,    &hooks};
    check(Os_TargetPrepare(&target) == E_OK);
    /* All four services operate before startup, including unmatched restores. */
    DisableAllInterrupts();
    SuspendAllInterrupts();
    EnableAllInterrupts();
    check(Os_InterruptDisabled() != 0);
    ResumeAllInterrupts();
    EnableAllInterrupts();
    ResumeAllInterrupts();
    check(Os_InterruptDisabled() == 0);
    if (strcmp(scenario, "disabled-first-start") == 0) {
        DisableAllInterrupts();
    }
    StartOS(1u);
    return 99;
}
