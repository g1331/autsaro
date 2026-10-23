/** @file Counter IDs beyond 8 and 16 bits are never truncated into other counters. */
#include "Rte_Os_Type.h"
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const uint8 task_name_0[] = "owner"; /**< Process-lifetime Task name bytes. */

static const char *scenario;    /**< Main-owned scenario selection before StartOS; no Init reset. */
static unsigned checks, errors; /**< Observed checks/errors; fixture-owned, no Init reset. */
static uint32_t expected_id; /**< Caller-owned expected id for synchronous checks; no Init reset. */
static TickRefType expected_value,
    expected_elapsed; /**< Caller-owned expected elapsed for synchronous checks; no Init reset. */
static TickType final_value,
    other_value; /**< Queried Counter-ticks outputs; fixture-owned, no Init reset. */
static void check(bool condition)
{
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
void ErrorHook(StatusType error)
{
    const OSServiceIdType service = OSErrorGetServiceId();
    check(error == 3u);
    if (service == 15u) {
        check(OSError_IncrementCounter_CounterID() == expected_id);
    } else if (service == 16u) {
        check(OSError_GetCounterValue_CounterID() == expected_id);
        check(OSError_GetCounterValue_Value() == expected_value);
    } else {
        check(service == 17u && OSError_GetElapsedValue_CounterID() == expected_id);
        check(OSError_GetElapsedValue_Value() == expected_value);
        check(OSError_GetElapsedValue_ElapsedValue() == expected_elapsed);
    }
    ++errors;
}
void StartupHook(void) {}
void ShutdownHook(StatusType reason)
{
    printf("counter_types scenario=%s checks=%u errors=%u value=%llu other=%llu reason=%u\n",
           scenario,
           checks,
           errors,
           (unsigned long long)final_value,
           (unsigned long long)other_value,
           reason);
    if (fflush(stdout) != 0) {
        ExitProcess(99u);
    }
}
static void owner(void)
{
    const uint32_t identifiers[3] = {UINT32_C(256), UINT32_C(65536), UINT32_MAX};
    for (unsigned i = 0u; i < 7u; ++i) {
        check(IncrementCounter(0u) == E_OK);
    }
    check(GetCounterValue(0u, &final_value) == E_OK && final_value == 7u);
    for (unsigned i = 0u; i < 3u; ++i) {
        TickType value = UINT64_MAX - UINT64_C(1);
        TickType previous = 3u;
        TickType elapsed = UINT64_C(4294967296);
        expected_id = identifiers[i];
        check(IncrementCounter((CounterType)expected_id) == 3u);
        ++checks;
        expected_value = &value;
        check(GetCounterValue((CounterType)expected_id, &value) == 3u);
        check(value == UINT64_MAX - UINT64_C(1));
        ++checks;
        expected_value = &previous;
        expected_elapsed = &elapsed;
        check(GetElapsedValue((CounterType)expected_id, &previous, &elapsed) == 3u);
        check(previous == 3u && elapsed == UINT64_C(4294967296));
        ++checks;
        check(GetCounterValue(0u, &final_value) == E_OK && final_value == 7u);
        check(GetCounterValue(1u, &other_value) == E_OK && other_value == 0u);
    }
    check(errors == (strcmp(scenario, "configured") == 0 ? 9u : 0u));
    ShutdownOS(E_OK);
}
int main(int argc, char **argv)
{
    check(argc == 2);
    scenario = argv[1];
    const Os_TaskConfigType task = {
        0u, task_name_0, owner, 10u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u};
    Os_CounterConfigType counters[2] = {{0u, 31u, 1u, 1u, 1u}, {1u, 31u, 1u, 1u, 1u}};
    Os_AlarmConfigType alarm = {.id = 0u, .counter = 0u, .action = OS_ALARM_ACTIVATE, .task = 0u};
    const Os_ExpiryActionType action = {OS_ALARM_ACTIVATE, 0u, 0u};
    const Os_ExpiryPointType point = {2u, &action, 1u};
    Os_ScheduleTableConfigType table = {
        .id = 0u, .counter = 0u, .duration = 4u, .points = &point, .point_count = 1u};
    Os_TimeConfigType time = {
        .counters = counters, .counter_count = 2u, .system_counter = 0u, .owner = INVALID_TASK};
    const Os_HookConfigType hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfigType target = {&task,
                                        1u,
                                        262144u,
                                        NULL,
                                        0u,
                                        NULL,
                                        0u,
                                        0u,
                                        0u,
                                        0u,
                                        &time,
                                        strcmp(scenario, "configured") == 0 ? &hooks : NULL,
                                        NULL,
                                        0u};
    const char *suffix = strchr(scenario, '-');
    if (suffix != NULL) {
        uint32_t identifier;
        StatusType expected = 3u;
        if (strcmp(suffix + 1, "256") == 0) {
            identifier = UINT32_C(256);
        } else if (strcmp(suffix + 1, "65536") == 0) {
            identifier = UINT32_C(65536);
        } else if (strcmp(suffix + 1, "max") == 0) {
            identifier = UINT32_MAX;
        } else {
            return 98;
        }
        if (strncmp(scenario, "counter-", 8u) == 0) {
            counters[0].id = (CounterType)identifier;
            expected = 8u;
        } else if (strncmp(scenario, "system-", 7u) == 0) {
            time.system_counter = (CounterType)identifier;
        } else if (strncmp(scenario, "alarm-", 6u) == 0) {
            alarm.counter = (CounterType)identifier;
            time.alarms = &alarm;
            time.alarm_count = 1u;
        } else if (strncmp(scenario, "increment-", 10u) == 0) {
            alarm.counter = 1u;
            alarm.action = OS_ALARM_INCREMENT_COUNTER;
            alarm.increment_counter = (CounterType)identifier;
            time.alarms = &alarm;
            time.alarm_count = 1u;
        } else if (strncmp(scenario, "schedule-", 9u) == 0) {
            table.counter = (CounterType)identifier;
            time.schedule_tables = &table;
            time.schedule_table_count = 1u;
        } else {
            return 98;
        }
        const StatusType status = Os_TargetPrepare(&target);
        printf("counter_rejection scenario=%s status=%u configured=%u\n",
               scenario,
               status,
               Os_Config != NULL ? 1u : 0u);
        return status == expected && Os_Config == NULL ? 0 : 98;
    }
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
