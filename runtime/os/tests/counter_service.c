#include "Rte_Os.h"
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>

static unsigned errors;
static StatusType expected_error;
static OSServiceIdType expected_service;
static CounterType expected_counter;
static TickRefType expected_value, expected_elapsed;

static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
void ErrorHook(StatusType error) {
    check(error == expected_error && OSErrorGetServiceId() == expected_service);
    if (expected_service == 16u) {
        check(OSError_GetCounterValue_CounterID() == expected_counter);
        check(OSError_GetCounterValue_Value() == expected_value);
    } else {
        check(expected_service == 17u);
        check(OSError_GetElapsedValue_CounterID() == expected_counter);
        check(OSError_GetElapsedValue_Value() == expected_value);
        check(OSError_GetElapsedValue_ElapsedValue() == expected_elapsed);
    }
    ++errors;
}
void StartupHook(void) {}
void ShutdownHook(StatusType reason) {
    printf("counter_service ticks=0 wrap=2 errors=%u reason=%u\n", errors, reason);
}
static void owner(void) {
    TimeInMicrosecondsType value = UINT64_MAX, elapsed = UINT64_MAX;
    check(Rte_Call_OsService_GetCounterValue(&value) == E_OK && value == 0u);
    for (unsigned i = 0u; i < 14u; ++i) {
        check(IncrementCounter(0u) == E_OK);
    }
    check(Rte_Call_OsService_GetCounterValue(&value) == E_OK && value == 14u);
    check(IncrementCounter(0u) == E_OK);
    check(IncrementCounter(0u) == E_OK);
    check(Rte_Call_OsService_GetElapsedValue(&value, &elapsed) == E_OK);
    check(value == 0u && elapsed == 2u);
    expected_error = E_OS_ID;
    expected_service = 16u;
    expected_counter = UINT32_MAX;
    expected_value = &value;
    value = UINT64_MAX;
    check(SERVICE_GET(UINT32_MAX, &value) == E_OS_ID && value == UINT64_MAX);
    expected_service = 17u;
    expected_elapsed = &elapsed;
    check(SERVICE_ELAPSED(UINT32_MAX, &value, &elapsed) == E_OS_ID);
    check(value == UINT64_MAX && elapsed == 2u);
    expected_counter = 0u;
    expected_error = E_OS_VALUE;
    value = 16u;
    check(Rte_Call_OsService_GetElapsedValue(&value, &elapsed) == E_OS_VALUE);
    check(value == 16u && elapsed == 2u);
    expected_error = E_OS_ILLEGAL_ADDRESS;
    expected_service = 16u;
    expected_value = NULL;
    check(Rte_Call_OsService_GetCounterValue(NULL) == E_OS_ILLEGAL_ADDRESS);
    expected_service = 17u;
    expected_value = &value;
    expected_elapsed = NULL;
    check(Rte_Call_OsService_GetElapsedValue(&value, NULL) == E_OS_ILLEGAL_ADDRESS);
    check(value == 16u);
    expected_error = E_OS_DISABLEDINT;
    expected_service = 16u;
    expected_elapsed = &elapsed;
    SuspendAllInterrupts();
    check(Rte_Call_OsService_GetCounterValue(&value) == E_OS_DISABLEDINT);
    expected_service = 17u;
    check(Rte_Call_OsService_GetElapsedValue(&value, &elapsed) == E_OS_DISABLEDINT);
    ResumeAllInterrupts();
    check(value == 16u && elapsed == 2u && errors == 7u);
    check(Rte_Call_OsService_GetCounterValue(&value) == E_OK && value == 0u);
    ShutdownOS(E_OK);
}
int main(void) {
    const Os_TaskConfig task = {0u, "owner",          owner, 10u, 1u, OS_BASIC_TASK,
                                1u, OS_SCHEDULE_FULL, 0u};
    const Os_CounterConfig counter = {0u, OSMAXALLOWEDVALUE, 1u, 1u, 1u};
    const Os_TimeConfig time = {
        .counters = &counter, .counter_count = 1u, .system_counter = 0u, .owner = INVALID_TASK};
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfig target = {&task, 1u, 262144u, NULL,  0u,     NULL, 0u,
                                    0u,    0u, 0u,      &time, &hooks, NULL};
    check(OSMAXALLOWEDVALUE == 15u);
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
