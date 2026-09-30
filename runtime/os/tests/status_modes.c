#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>

#ifndef EXPECTED_STATUS_EXTENDED
#error The independent consumer requires an expected status mode
#endif
#if OS_STATUS_EXTENDED != EXPECTED_STATUS_EXTENDED
#error The selected status mode differs from the independent input
#endif

static unsigned errors;
static unsigned peers;
static TickType remaining = UINT64_MAX;
void StartupHook(void) {}
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
void ErrorHook(StatusType error) {
    static const StatusType statuses[] = {4u, 4u, 5u, 5u, 7u, 7u, 3u, 9u};
    static const OSServiceIdType services[] = {130u, 132u, 141u, 144u, 142u, 143u, 130u, 130u};
    TaskType caller = INVALID_TASK;
    check(errors < 8u);
    check(error == statuses[errors] && OSErrorGetServiceId() == services[errors]);
    check(GetTaskID(&caller) == E_OK && caller == 0u && GetISRID() == INVALID_ISR);
    switch (errors) {
    case 0u:
        check(OSError_ActivateTask_TaskID() == 1u);
        break;
    case 1u:
        check(OSError_ChainTask_TaskID() == 1u);
        break;
    case 2u:
        check(OSError_GetAlarm_AlarmID() == 0u && OSError_GetAlarm_Tick() == &remaining);
        break;
    case 3u:
        check(OSError_CancelAlarm_AlarmID() == 0u);
        break;
    case 4u:
        check(OSError_SetRelAlarm_AlarmID() == 0u && OSError_SetRelAlarm_Increment() == 1u &&
              OSError_SetRelAlarm_Cycle() == 0u);
        break;
    case 5u:
        check(OSError_SetAbsAlarm_AlarmID() == 0u && OSError_SetAbsAlarm_Start() == 2u &&
              OSError_SetAbsAlarm_Cycle() == 0u);
        break;
    default:
        check(OSError_ActivateTask_TaskID() == 99u);
        break;
    }
    check(ActivateTask(99u) == 2u);
    check(OSErrorGetServiceId() == services[errors]);
    ++errors;
}
void ShutdownHook(StatusType error) {
    (void)printf("status_modes extended=%u errors=%u peers=%u preserved=%u reason=%u\n",
                 (unsigned)OS_STATUS_EXTENDED, errors, peers, (unsigned)(remaining == UINT64_MAX),
                 (unsigned)error);
}
TASK(Peer) {
    TaskType caller = INVALID_TASK;
    check(GetTaskID(&caller) == E_OK && caller == 1u && errors == 8u);
    ++peers;
    ShutdownOS(E_OK);
}
TASK(Owner) {
    check(ActivateTask(1u) == E_OK && peers == 0u);
    check(ActivateTask(1u) == 4u && errors == 1u);
    check(ChainTask(1u) == 4u && errors == 2u && peers == 0u);
    check(GetAlarm(0u, &remaining) == 5u && errors == 3u && remaining == UINT64_MAX);
    check(CancelAlarm(0u) == 5u && errors == 4u);
    check(SetRelAlarm(0u, 1u, 0u) == E_OK);
    check(SetRelAlarm(0u, 1u, 0u) == 7u && errors == 5u);
    check(SetAbsAlarm(0u, 2u, 0u) == 7u && errors == 6u);
    check(CancelAlarm(0u) == E_OK);
    check(ActivateTask(99u) == 3u && errors == 7u);
    DisableAllInterrupts();
    check(ActivateTask(99u) == 9u && errors == 8u && peers == 0u);
    EnableAllInterrupts();
    check(TerminateTask() == E_OK);
    check(false);
}
int main(void) {
    const Os_TaskConfig tasks[] = {
        {0u, "Owner", OS_TASK_ENTRY(Owner), 10u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "Peer", OS_TASK_ENTRY(Peer), 5u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_CounterConfig counter = {0u, 15u, 1u, 1u, 1u};
    const Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_ACTIVATE, 1u, 0u, NULL, 0u, 0u, 0u, 0u, 0u};
    const Os_TimeConfig time = {&counter, 1u, &alarm, 1u, 0u, INVALID_TASK, 0u, NULL, NULL, 0u};
    const Os_HookConfig hooks = {ErrorHook, NULL, NULL};
    const Os_TargetConfig target = {tasks, 2u, 262144u, NULL,  0u,     NULL, 0u,
                                    0u,    0u, 0u,      &time, &hooks, NULL};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
