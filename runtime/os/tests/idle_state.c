#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static volatile LONG requested;
static volatile LONG observed;
static unsigned wakes;
static unsigned errors;
static StatusType error_status[5];
static OSServiceIdType error_service[5];
static IdleModeType rejected_mode;
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
void ErrorHook(StatusType error) {
    const OSServiceIdType service = OSErrorGetServiceId();
    check(errors < 5u && isOsStarted() == TRUE);
    if (service == 29u) {
        check(OSError_ControlIdle_CoreID() == UINT16_MAX);
        check(OSError_ControlIdle_IdleMode() == rejected_mode);
    } else {
        check(service == 54u && error == 9u);
    }
    error_status[errors] = error;
    error_service[errors++] = service;
}
void Os_TestIdleObserved(void) {
    if (InterlockedCompareExchange(&requested, 0, 0) != 0 &&
        InterlockedCompareExchange(&observed, 1, 0) == 0) {
        check(Os_StackCurrent()->role == 'I' && isOsStarted() == TRUE);
        vPortGenerateSimulatedInterruptFromWindowsThread(5u);
    }
}
static uint32_t interrupt(void) {
    TaskType task = 42u;
    TaskStateType state = SUSPENDED;
    check(GetISRID() == 5u && isOsStarted() == TRUE);
    check(GetTaskID(&task) == E_OK && task == INVALID_TASK);
    check(GetTaskState(0u, &state) == E_OK && state == WAITING);
    check(InterlockedCompareExchange(&observed, 0, 0) == 1);
    if (strcmp(scenario, "isr") == 0) {
        check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OK);
    }
    check(SetEvent(0u, 1u) == E_OK);
    ++wakes;
    return 0u;
}
void StartupHook(void) {
    check(isOsStarted() == TRUE);
    rejected_mode = 0u;
    check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OS_CALLEVEL);
    vPortSetInterruptHandler(5u, &interrupt);
}
void ShutdownHook(StatusType error) {
    check(isOsStarted() == TRUE);
    printf("idle_state scenario=%s observed=%ld wakes=%u errors=%u reason=%u\n", scenario,
           InterlockedCompareExchange(&observed, 0, 0), wakes, errors, error);
    for (unsigned i = 0u; i < errors; ++i) {
        printf("idle_error status=%u service=%u\n", error_status[i], error_service[i]);
    }
}
static void owner(void) {
    check(isOsStarted() == TRUE);
    if (strcmp(scenario, "invalid") == 0 || strcmp(scenario, "unconfigured") == 0) {
        rejected_mode = 99u;
        check(ControlIdle(UINT16_MAX, 99u) == E_OS_ID);
    } else if (strcmp(scenario, "masked") == 0) {
        DisableAllInterrupts();
        rejected_mode = 0u;
        check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == 9u);
        check(isOsStarted() == FALSE);
        EnableAllInterrupts();
        check(isOsStarted() == TRUE);
    }
    check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OK);
    check(ControlIdle(0u, IDLE_NO_HALT) == E_OK);
    InterlockedExchange(&requested, 1);
    check(WaitEvent(1u) == E_OK && wakes == 1u);
    check(ClearEvent(1u) == E_OK);
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    check(isOsStarted() == FALSE);
    const Os_TaskConfig task = {0u, "owner",          owner, 10u, 1u, OS_EXTENDED_TASK,
                                1u, OS_SCHEDULE_FULL, 0u};
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfig target = {
        &task, 1u, 262144u, NULL, 0u,   NULL,
        0u,    0u, 0u,      0u,   NULL, strcmp(scenario, "unconfigured") == 0 ? NULL : &hooks,
        NULL};
    check(Os_TargetPrepare(&target) == E_OK && isOsStarted() == FALSE);
    StartOS(1u);
    return 99;
}
