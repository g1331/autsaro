#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    char kind;
    CounterType counter;
    TickType value;
} Observation;
static Observation observations[256];
static unsigned observation_count;
static unsigned checks;
static unsigned rejected;
static unsigned errors;
static StatusType last_error;
static const char *scenario;
static CounterType observed_counter;
static volatile LONG interrupt_done;
static void check(bool condition) {
    ++checks;
    if (!condition) {
        ShutdownOS(E_OS_STATE);
    }
}
static void refuse(StatusType status, StatusType expected) {
    check(status == expected);
    ++rejected;
}
static void record(char kind, CounterType counter) {
    TickType value;
    check(GetCounterValue(counter, &value) == E_OK);
    taskENTER_CRITICAL();
    check(observation_count < 256u);
    observations[observation_count].kind = kind;
    observations[observation_count].counter = counter;
    observations[observation_count].value = value;
    ++observation_count;
    taskEXIT_CRITICAL();
}
static void task_a(void) {
    record('A', observed_counter);
    (void)TerminateTask();
}
static void task_b(void) {
    record('B', 1u);
    (void)TerminateTask();
}
static void task_e(void) {
    EventMaskType events;
    if (strcmp(scenario, "same-point") == 0) {
        check(GetEvent(2u, &events) == E_OK);
        check(events == 1u);
        record('E', 0u);
        (void)TerminateTask();
    }
    for (;;) {
        check(WaitEvent(1u) == E_OK);
        check(ClearEvent(1u) == E_OK);
        record('E', 0u);
    }
}
static void action_error(StatusType status) {
    ++errors;
    last_error = status;
}
static void inc(CounterType id, unsigned count) {
    for (unsigned i = 0u; i < count; ++i) {
        check(IncrementCounter(id) == E_OK);
    }
}
static void state(ScheduleTableType id, ScheduleTableStatusType expected) {
    ScheduleTableStatusType value = 99u;
    check(GetScheduleTableStatus(id, &value) == E_OK);
    check(value == expected);
}
static void counter(CounterType id, TickType expected) {
    TickType value = UINT64_C(999);
    check(GetCounterValue(id, &value) == E_OK);
    check(value == expected);
}
static uint32_t service_interrupt(void) {
    ScheduleTableStatusType value = 99u;
    if (strcmp(scenario, "category1") == 0) {
        refuse(StartScheduleTableRel(0u, 1u), E_OS_CALLEVEL);
        refuse(StartScheduleTableAbs(0u, 1u), E_OS_CALLEVEL);
        refuse(StopScheduleTable(0u), E_OS_CALLEVEL);
        refuse(NextScheduleTable(0u, 2u), E_OS_CALLEVEL);
        refuse(GetScheduleTableStatus(0u, &value), E_OS_CALLEVEL);
        check(value == 99u);
    } else {
        check(GetScheduleTableStatus(0u, &value) == E_OK);
        check(value == SCHEDULETABLE_STOPPED);
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        check(StopScheduleTable(0u) == E_OK);
        check(StartScheduleTableAbs(0u, 1u) == E_OK);
        check(StopScheduleTable(0u) == E_OK);
    }
    (void)InterlockedExchange(&interrupt_done, 1);
    return 0u;
}
static void owner(void) {
    if (strcmp(scenario, "capacity") == 0) {
        check(StartScheduleTableRel(0u, 2u) == E_OK);
        check(StartScheduleTableRel(1u, 1u) == E_OK);
        state(0u, SCHEDULETABLE_RUNNING);
        state(1u, SCHEDULETABLE_RUNNING);
        inc(0u, 7u);
        inc(1u, 6u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(1u, SCHEDULETABLE_STOPPED);
        for (CounterType i = 2u; i < 8u; ++i) {
            counter(i, 0u);
        }
    } else if (strcmp(scenario, "wrap") == 0) {
        for (CounterType i = 0u; i < 8u; ++i) {
            TickType previous = 14u;
            TickType elapsed = UINT64_C(999);
            inc(i, 14u);
            counter(i, 14u);
            inc(i, 3u);
            check(GetElapsedValue(i, &previous, &elapsed) == E_OK);
            check((previous == 1u) && (elapsed == 3u));
            counter(i, 1u);
            for (CounterType other = (CounterType)(i + 1u); other < 8u; ++other) {
                counter(other, 0u);
            }
        }
    } else if (strcmp(scenario, "eight-tables") == 0) {
        for (ScheduleTableType i = 0u; i < 8u; ++i) {
            check(StartScheduleTableRel(i, 1u) == E_OK);
        }
        for (CounterType i = 0u; i < 8u; ++i) {
            observed_counter = i;
            inc(i, 4u);
            counter(i, 4u);
            state(i, SCHEDULETABLE_STOPPED);
        }
        check(observation_count == 8u);
    } else if (strcmp(scenario, "relative-max") == 0) {
        check(StartScheduleTableRel(0u, 14u) == E_OK);
        inc(0u, 19u);
        state(0u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "absolute") == 0) {
        inc(0u, 14u);
        check(StartScheduleTableAbs(0u, 14u) == E_OK);
        inc(0u, 16u);
        check(observation_count == 0u);
        inc(0u, 1u);
        check(observation_count == 1u);
        inc(0u, 4u);
        state(0u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "relative-zero") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        inc(0u, 1u);
        state(0u, SCHEDULETABLE_STOPPED);
        check(observation_count == 1u);
    } else if (strcmp(scenario, "same-point") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        inc(0u, 1u);
        check((observation_count == 1u) && (errors == 0u));
        state(0u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "repeat") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        inc(0u, 11u);
        state(0u, SCHEDULETABLE_RUNNING);
        check(StopScheduleTable(0u) == E_OK);
        inc(0u, 4u);
        check(observation_count == 4u);
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        inc(0u, 2u);
        check(observation_count == 5u);
    } else if (strcmp(scenario, "link-zero-final") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        inc(0u, 4u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_STOPPED);
        check(observation_count == 3u);
    } else if (strcmp(scenario, "link-backward") == 0) {
        check(StartScheduleTableRel(2u, 1u) == E_OK);
        check(NextScheduleTable(2u, 0u) == E_OK);
        inc(0u, 11u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "link") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        state(2u, SCHEDULETABLE_NEXT);
        inc(0u, 6u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_RUNNING);
        inc(0u, 5u);
        state(2u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "replace-next") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        check(NextScheduleTable(0u, 3u) == E_OK);
        state(2u, SCHEDULETABLE_STOPPED);
        state(3u, SCHEDULETABLE_NEXT);
        inc(0u, 11u);
        state(3u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "stop-next") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        check(StopScheduleTable(2u) == E_OK);
        inc(0u, 6u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_STOPPED);
        check(observation_count == 2u);
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        check(NextScheduleTable(0u, 2u) == E_OK);
        check(StopScheduleTable(0u) == E_OK);
        state(2u, SCHEDULETABLE_STOPPED);
        inc(0u, 6u);
        check(observation_count == 2u);
    } else if ((strcmp(scenario, "autostart") == 0) || (strcmp(scenario, "autostart-mode2") == 0)) {
        state(0u, SCHEDULETABLE_RUNNING);
        inc(0u, 6u);
        state(0u, SCHEDULETABLE_STOPPED);
        check(observation_count == 2u);
    } else if (strcmp(scenario, "autostart-absolute") == 0) {
        state(0u, SCHEDULETABLE_RUNNING);
        inc(0u, 16u);
        check(observation_count == 0u);
        inc(0u, 5u);
        state(0u, SCHEDULETABLE_STOPPED);
        check(observation_count == 2u);
    } else if ((strcmp(scenario, "category1") == 0) || (strcmp(scenario, "category2") == 0)) {
        vPortGenerateSimulatedInterrupt((strcmp(scenario, "category1") == 0) ? 31u : 30u);
        check(InterlockedCompareExchange(&interrupt_done, 0, 0) == 1);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "counter-chain") == 0) {
        check(SetRelAlarm(0u, 2u, 2u) == E_OK);
        check(SetRelAlarm(1u, 1u, 1u) == E_OK);
        inc(2u, 6u);
        counter(2u, 6u);
        counter(3u, 3u);
        counter(4u, 3u);
        counter(0u, 0u);
        counter(1u, 0u);
        counter(7u, 0u);
    } else if (strcmp(scenario, "action-error") == 0) {
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        inc(0u, 6u);
        check((errors == 1u) && (last_error == E_OS_LIMIT));
        state(0u, SCHEDULETABLE_STOPPED);
    } else if (strcmp(scenario, "rejects") == 0) {
        ScheduleTableStatusType sentinel = 99u;
        refuse(StartScheduleTableRel(99u, 1u), E_OS_ID);
        refuse(StartScheduleTableAbs(99u, 1u), E_OS_ID);
        refuse(StartScheduleTableRel(0u, 0u), E_OS_VALUE);
        refuse(StartScheduleTableRel(0u, 15u), E_OS_VALUE);
        refuse(StartScheduleTableAbs(0u, 16u), E_OS_VALUE);
        refuse(StopScheduleTable(0u), E_OS_NOFUNC);
        refuse(StopScheduleTable(99u), E_OS_ID);
        refuse(NextScheduleTable(0u, 2u), E_OS_NOFUNC);
        refuse(NextScheduleTable(0u, 1u), E_OS_ID);
        refuse(NextScheduleTable(99u, 2u), E_OS_ID);
        refuse(NextScheduleTable(0u, 99u), E_OS_ID);
        refuse(GetScheduleTableStatus(99u, &sentinel), E_OS_ID);
        check(sentinel == 99u);
        refuse(GetScheduleTableStatus(0u, NULL), E_OS_ILLEGAL_ADDRESS);
        check(StartScheduleTableRel(0u, 1u) == E_OK);
        refuse(StartScheduleTableRel(0u, 1u), E_OS_STATE);
        refuse(StartScheduleTableAbs(0u, 0u), E_OS_STATE);
        refuse(NextScheduleTable(0u, 0u), E_OS_STATE);
        check(NextScheduleTable(0u, 2u) == E_OK);
        refuse(StartScheduleTableRel(2u, 1u), E_OS_STATE);
        refuse(NextScheduleTable(2u, 3u), E_OS_NOFUNC);
        refuse(NextScheduleTable(0u, 2u), E_OS_STATE);
        state(0u, SCHEDULETABLE_RUNNING);
        state(2u, SCHEDULETABLE_NEXT);
        inc(0u, 11u);
        state(0u, SCHEDULETABLE_STOPPED);
        state(2u, SCHEDULETABLE_STOPPED);
    } else {
        check(false);
    }
    ShutdownOS(E_OK);
}
void StartupHook(void) { Os_TargetTrace('S'); }
void ShutdownHook(StatusType error) {
    for (unsigned i = 0u; i < observation_count; ++i) {
        printf("timing_action %c counter=%u value=%llu\n", observations[i].kind,
               (unsigned)observations[i].counter, (unsigned long long)observations[i].value);
    }
    printf("sc1_timing case=%s checks=%u rejects=%u actions=%u errors=%u last_error=%u status=%u\n",
           scenario, checks, rejected, observation_count, errors, (unsigned)last_error,
           (unsigned)error);
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {
        {0u, "Owner", owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "A", task_a, 20u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {2u, "E", task_e, 21u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {3u, "B", task_b, 19u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_CounterConfig counters[9];
    Os_AlarmConfig alarms[] = {
        {0u, 2u, OS_ALARM_INCREMENT_COUNTER, 0u, 0u, NULL, 0u, 0u, 1u, 1u, 3u},
        {1u, 3u, OS_ALARM_INCREMENT_COUNTER, 0u, 0u, NULL, 0u, 0u, 1u, 1u, 4u}};
    Os_ExpiryAction activate_a[] = {{OS_ALARM_ACTIVATE, 1u, 0u}};
    Os_ExpiryAction set_e[] = {{OS_ALARM_EVENT, 2u, 1u}};
    const Os_ExpiryAction activate_b[] = {{OS_ALARM_ACTIVATE, 3u, 0u}};
    const Os_ExpiryAction same[] = {{OS_ALARM_EVENT, 2u, 1u}, {OS_ALARM_ACTIVATE, 2u, 0u}};
    Os_ExpiryPoint points[] = {{1u, activate_a, 1u}, {3u, set_e, 1u}};
    const Os_ExpiryPoint zero_point[] = {{0u, activate_a, 1u}};
    const Os_ExpiryPoint bpoints[] = {{1u, activate_b, 1u}};
    Os_ScheduleTableConfig tables[9] = {
        {0u, 0u, 5u, points, 2u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 1u},
        {1u, 1u, 5u, bpoints, 1u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 1u},
        {2u, 0u, 5u, points, 2u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 1u},
        {3u, 0u, 5u, points, 2u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 1u}};
    Os_TimeConfig time = {counters, 8u, alarms, 2u, 0u, 0u, 4u, action_error, tables, 4u};
    Os_TargetConfig config = {tasks, 4u, 262144u, NULL, 0u, NULL, 0u, 0u, 0u, 0u, &time};
    StatusType status;
    scenario = (argc == 2) ? argv[1] : "capacity";
    for (unsigned i = 0u; i < 9u; ++i) {
        counters[i] = (Os_CounterConfig){(CounterType)i, 15u, 1u, 1u, 1u};
    }
    if (strcmp(scenario, "eight-tables") == 0) {
        time.schedule_table_count = 8u;
        for (unsigned i = 0u; i < 8u; ++i) {
            tables[i] = (Os_ScheduleTableConfig){(ScheduleTableType)i,
                                                 (CounterType)i,
                                                 3u,
                                                 points,
                                                 1u,
                                                 0u,
                                                 OS_SCHEDULE_SYNC_NONE,
                                                 0u,
                                                 0u,
                                                 1u};
        }
    }
    if (strcmp(scenario, "link-zero-final") == 0) {
        tables[0].duration = 3u;
        tables[2].duration = 0u;
        tables[2].points = zero_point;
        tables[2].point_count = 1u;
    }
    if (strcmp(scenario, "autostart-mode2") == 0) {
        tables[0].autostart_modes = 2u;
        tasks[0].autostart_modes = 3u;
        tasks[2].autostart_modes = 3u;
    }
    if (strcmp(scenario, "autostart-absolute") == 0) {
        tables[0].autostart_modes = 1u;
        tables[0].absolute = 1u;
        tables[0].start = 0u;
    }
    if (strcmp(scenario, "category1") == 0) {
        config.category1_isrs = UINT32_C(1) << 31u;
    }
    if (strcmp(scenario, "relative-zero") == 0 || strcmp(scenario, "same-point") == 0) {
        points[0].offset = 0u;
        tables[0].duration = 0u;
        tables[0].point_count = 1u;
        if (strcmp(scenario, "same-point") == 0) {
            points[0].actions = same;
            points[0].action_count = 2u;
            tasks[2].autostart_modes = 0u;
        }
    }
    if (strcmp(scenario, "repeat") == 0) {
        tables[0].repeating = 1u;
    }
    if (strcmp(scenario, "autostart") == 0) {
        tables[0].autostart_modes = 1u;
    }
    if (strcmp(scenario, "action-error") == 0) {
        activate_a[0].task = 0u;
    }
    if (strcmp(scenario, "bad-cycle") == 0) {
        alarms[1].increment_counter = 2u;
    }
    if (strcmp(scenario, "bad-counter-count") == 0) {
        time.counter_count = 9u;
    }
    if (strcmp(scenario, "bad-table-count") == 0) {
        time.schedule_table_count = 9u;
    }
    if (strcmp(scenario, "bad-null-tables") == 0) {
        time.schedule_tables = NULL;
    }
    if (strcmp(scenario, "bad-duplicate-id") == 0) {
        tables[1].id = 0u;
    }
    if (strcmp(scenario, "bad-counter") == 0) {
        tables[0].counter = 99u;
    }
    if (strcmp(scenario, "bad-zero-points") == 0) {
        tables[0].point_count = 0u;
    }
    if (strcmp(scenario, "bad-many-points") == 0) {
        tables[0].point_count = 33u;
    }
    if (strcmp(scenario, "bad-null-points") == 0) {
        tables[0].points = NULL;
    }
    if (strcmp(scenario, "bad-duplicate-offset") == 0) {
        points[1].offset = 1u;
    }
    if (strcmp(scenario, "bad-duration") == 0) {
        tables[0].duration = 2u;
    }
    if (strcmp(scenario, "bad-empty-action") == 0) {
        points[0].action_count = 0u;
    }
    if (strcmp(scenario, "bad-null-action") == 0) {
        points[0].actions = NULL;
    }
    if (strcmp(scenario, "bad-many-actions") == 0) {
        points[0].action_count = 17u;
    }
    if (strcmp(scenario, "bad-task") == 0) {
        activate_a[0].task = 99u;
    }
    if (strcmp(scenario, "bad-event") == 0) {
        set_e[0].event = 0u;
    }
    if (strcmp(scenario, "bad-repeat-final") == 0) {
        tables[0].duration = 3u;
        tables[0].repeating = 1u;
    }
    if (strcmp(scenario, "bad-sync") == 0) {
        tables[0].synchronization = 1u;
    }
    if (strcmp(scenario, "bad-autostart") == 0) {
        tables[0].autostart_modes = 1u;
        tables[0].start = 0u;
    }
    if (strcmp(scenario, "bad-mincycle") == 0) {
        counters[0].minimum_cycle = 2u;
    }
    status = Os_TargetPrepare(&config);
    if (strncmp(scenario, "bad-", 4u) == 0) {
        printf("timing_prepare case=%s status=%u ready=%d\n", scenario, (unsigned)status,
               Os_TargetReady());
        return (status == E_OK) ? 99 : 0;
    }
    if (status != E_OK) {
        return 98;
    }
    if ((strcmp(scenario, "category1") == 0) || (strcmp(scenario, "category2") == 0)) {
        vPortSetInterruptHandler((strcmp(scenario, "category1") == 0) ? 31u : 30u,
                                 service_interrupt);
    }
    StartOS((strcmp(scenario, "autostart-mode2") == 0) ? 2u : 1u);
    return 97;
}
