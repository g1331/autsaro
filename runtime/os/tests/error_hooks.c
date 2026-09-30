#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#if !OS_USE_GET_SERVICE_ID && defined(OSErrorGetServiceId)
#error Service ID macro must be absent when disabled
#endif
#if !OS_USE_PARAMETER_ACCESS && defined(OSError_ActivateTask_TaskID)
#error Parameter macro must be absent when disabled
#endif

typedef struct {
    unsigned vector;
    StatusType status;
    OSServiceIdType service;
    TaskType caller;
    unsigned checked;
    char actor;
} Record;
static Record records[40];
static unsigned count;
static unsigned vector;
static bool configured = true;
static TaskStateType state = 99u;
static EventMaskType events = UINT32_C(0xdead);
static TickType value = UINT64_MAX;
static TickType elapsed = UINT64_MAX;
static AlarmBaseType base = {61u, 62u, 63u};
static ScheduleTableStatusType table_status = 99u;

static void check(bool condition) {
    if (!condition) {
        ShutdownOS(E_OS_STATE);
    }
}
#if OS_USE_PARAMETER_ACCESS
static void check_parameters(void) {
    switch (vector) {
    case 0u:
    case 28u:
        check(OSError_ActivateTask_TaskID() == 99u);
        break;
    case 1u:
        break;
    case 2u:
        check(OSError_ChainTask_TaskID() == 99u);
        break;
    case 3u:
        check(OSError_GetTaskID_TaskID() == NULL);
        break;
    case 4u:
        check(OSError_GetTaskState_TaskID() == 99u && OSError_GetTaskState_State() == &state);
        break;
    case 5u:
        check(OSError_GetResource_ResID() == 99u);
        break;
    case 6u:
        check(OSError_ReleaseResource_ResID() == 0u);
        break;
    case 7u:
        break;
    case 8u:
        check(OSError_WaitEvent_Mask() == UINT32_C(0x80000000));
        break;
    case 9u:
        check(OSError_ClearEvent_Mask() == UINT32_C(0x80000000));
        break;
    case 10u:
        check(OSError_SetEvent_TaskID() == 99u && OSError_SetEvent_Mask() == UINT32_C(0x80000001));
        break;
    case 11u:
        check(OSError_GetEvent_TaskID() == 99u && OSError_GetEvent_Event() == &events);
        break;
    case 12u:
        check(OSError_IncrementCounter_CounterID() == 99u);
        break;
    case 13u:
        check(OSError_GetCounterValue_CounterID() == 99u &&
              OSError_GetCounterValue_Value() == &value);
        break;
    case 14u:
        check(OSError_GetElapsedValue_CounterID() == 0u &&
              OSError_GetElapsedValue_Value() == NULL &&
              OSError_GetElapsedValue_ElapsedValue() == &elapsed);
        break;
    case 15u:
        check(OSError_GetAlarmBase_AlarmID() == 99u && OSError_GetAlarmBase_Info() == &base);
        break;
    case 16u:
        check(OSError_GetAlarm_AlarmID() == 0u && OSError_GetAlarm_Tick() == &value);
        break;
    case 17u:
        check(OSError_SetRelAlarm_AlarmID() == 0u && OSError_SetRelAlarm_Increment() == 0u &&
              OSError_SetRelAlarm_Cycle() == 0u);
        break;
    case 18u:
        check(OSError_SetAbsAlarm_AlarmID() == 0u &&
              OSError_SetAbsAlarm_Start() == (UINT64_C(1) << 40u) &&
              OSError_SetAbsAlarm_Cycle() == 0u);
        break;
    case 19u:
        check(OSError_CancelAlarm_AlarmID() == 0u);
        break;
    case 20u:
        check(OSError_StartScheduleTableRel_ScheduleTableID() == 0u &&
              OSError_StartScheduleTableRel_Offset() == 0u);
        break;
    case 21u:
        check(OSError_StartScheduleTableAbs_ScheduleTableID() == 0u &&
              OSError_StartScheduleTableAbs_Start() == (UINT64_C(1) << 40u));
        break;
    case 22u:
        check(OSError_StopScheduleTable_ScheduleTableID() == 0u);
        break;
    case 23u:
        check(OSError_NextScheduleTable_ScheduleTableID_From() == 0u &&
              OSError_NextScheduleTable_ScheduleTableID_To() == 1u);
        break;
    case 24u:
        check(OSError_GetScheduleTableStatus_ScheduleTableID() == 99u &&
              OSError_GetScheduleTableStatus_ScheduleStatus() == &table_status);
        break;
    case 25u:
        check(OSError_GetElapsedValue_CounterID() == 0u &&
              OSError_GetElapsedValue_Value() == &value &&
              OSError_GetElapsedValue_ElapsedValue() == &elapsed);
        break;
    case 26u:
        check(OSError_IncrementCounter_CounterID() == 1u);
        break;
    case 27u:
        check(OSError_ActivateTask_TaskID() == 0u);
        break;
    default:
        ShutdownOS(E_OS_STATE);
        break;
    }
}
#endif
void ErrorHook(StatusType Error) {
    TaskType caller = INVALID_TASK;
    TaskStateType actual_state = 99u;
    OSServiceIdType service = OSServiceId_Unknown;
    unsigned checked = 0u;
    check(count < 40u);
    check(GetTaskID(&caller) == E_OK &&
          caller == ((vector == 28u) ? INVALID_TASK : ((vector == 9u) ? 1u : 0u)));
    if (vector != 28u) {
        check(GetTaskState(caller, &actual_state) == E_OK && actual_state == RUNNING);
    }
#if OS_USE_GET_SERVICE_ID
    service = OSErrorGetServiceId();
#endif
#if OS_USE_PARAMETER_ACCESS
    check_parameters();
    checked = 1u;
#endif
    /* A forbidden mutating call and a failing permitted query must neither
     * recursively call ErrorHook nor replace the outer service/parameters. */
    check(ActivateTask(99u) == E_OS_CALLEVEL);
    check(GetTaskState(99u, &actual_state) == E_OS_ID &&
          actual_state == ((vector == 28u) ? 99u : RUNNING));
#if OS_USE_GET_SERVICE_ID
    check(OSErrorGetServiceId() == service);
#endif
#if OS_USE_PARAMETER_ACCESS
    check_parameters();
#endif
    records[count++] = (Record){vector, Error, service, caller, checked, Os_StackCurrent()->role};
}
void StartupHook(void) {
    vector = 28u;
    check(GetActiveApplicationMode() == 1u);
    check(ActivateTask(99u) == E_OS_CALLEVEL);
    check(count == (configured ? 1u : 0u));
    /* After the nested ErrorHook, restore StartupHook's restrictive phase. */
    check(ActivateTask(99u) == E_OS_CALLEVEL);
    check(count == (configured ? 2u : 0u));
}
void ShutdownHook(StatusType Error) {
    printf("errors calls=%u service_access=%u parameter_access=%u configured=%u reason=%u\n", count,
           OS_USE_GET_SERVICE_ID, OS_USE_PARAMETER_ACCESS, configured ? 1u : 0u, Error);
    for (unsigned i = 0u; i < count; ++i) {
        const Record *r = &records[i];
        printf("error vector=%u status=%u service=%u caller=%u parameters=%u actor=%c\n", r->vector,
               r->status, r->service, r->caller, r->checked, r->actor);
    }
}
static void basic(void) {
    check(vector == 9u && ClearEvent(UINT32_C(0x80000000)) == E_OS_ACCESS);
    (void)TerminateTask();
    ShutdownOS(E_OS_STATE);
}
static StatusType operation(unsigned index) {
    StatusType result = E_OK;
    switch (index) {
    case 0u:
        result = ActivateTask(99u);
        break;
    case 1u:
        check(GetResource(0u) == E_OK);
        result = TerminateTask();
        check(ReleaseResource(0u) == E_OK);
        break;
    case 2u:
        result = ChainTask(99u);
        break;
    case 3u:
        result = GetTaskID(NULL);
        break;
    case 4u:
        result = GetTaskState(99u, &state);
        break;
    case 5u:
        result = GetResource(99u);
        break;
    case 6u:
        result = ReleaseResource(0u);
        break;
    case 7u:
        check(GetResource(0u) == E_OK);
        result = Schedule();
        check(ReleaseResource(0u) == E_OK);
        break;
    case 8u:
        check(GetResource(0u) == E_OK);
        result = WaitEvent(UINT32_C(0x80000000));
        check(ReleaseResource(0u) == E_OK);
        break;
    case 9u:
        check(ActivateTask(1u) == E_OK);
        result = E_OS_ACCESS;
        break;
    case 10u:
        result = SetEvent(99u, UINT32_C(0x80000001));
        break;
    case 11u:
        result = GetEvent(99u, &events);
        break;
    case 12u:
        result = IncrementCounter(99u);
        break;
    case 13u:
        result = GetCounterValue(99u, &value);
        break;
    case 14u:
        result = GetElapsedValue(0u, NULL, &elapsed);
        break;
    case 15u:
        result = GetAlarmBase(99u, &base);
        break;
    case 16u:
        result = GetAlarm(0u, &value);
        break;
    case 17u:
        result = SetRelAlarm(0u, 0u, 0u);
        break;
    case 18u:
        result = SetAbsAlarm(0u, UINT64_C(1) << 40u, 0u);
        break;
    case 19u:
        result = CancelAlarm(0u);
        break;
    case 20u:
        result = StartScheduleTableRel(0u, 0u);
        break;
    case 21u:
        result = StartScheduleTableAbs(0u, UINT64_C(1) << 40u);
        break;
    case 22u:
        result = StopScheduleTable(0u);
        break;
    case 23u:
        result = NextScheduleTable(0u, 1u);
        break;
    case 24u:
        result = GetScheduleTableStatus(99u, &table_status);
        break;
    case 25u:
        value = 16u;
        result = GetElapsedValue(0u, &value, &elapsed);
        check(value == 16u);
        break;
    case 26u:
        result = IncrementCounter(1u);
        break;
    case 27u:
        check(SetRelAlarm(0u, 1u, 0u) == E_OK);
        check(IncrementCounter(0u) == E_OK);
        result = E_OS_LIMIT;
        break;
    default:
        ShutdownOS(E_OS_STATE);
        break;
    }
    return result;
}
static uint32_t error_isr(void) {
    check(ActivateTask(99u) == E_OS_ID);
    return 0u;
}
static void owner(void) {
    static const StatusType expected[28] = {3u, 6u, 3u, 10u, 3u,  3u, 5u, 6u, 6u, 1u,
                                            3u, 3u, 3u, 3u,  10u, 3u, 5u, 8u, 8u, 5u,
                                            8u, 8u, 5u, 5u,  3u,  8u, 3u, 4u};
    for (vector = 0u; vector < 28u; ++vector) {
        const unsigned before = count;
        check(operation(vector) == expected[vector]);
        check(count == before + (configured ? 1u : 0u));
        check(state == 99u && events == UINT32_C(0xdead) && elapsed == UINT64_MAX &&
              table_status == 99u);
        check(base.maxallowedvalue == 61u && base.ticksperbase == 62u && base.mincycle == 63u);
#if OS_USE_GET_SERVICE_ID
        check(OSErrorGetServiceId() == OSServiceId_Unknown);
#endif
    }
    {
        const unsigned before = count;
        vector = 0u;
        vPortSetInterruptHandler(4u, &error_isr);
        vPortGenerateSimulatedInterrupt(4u);
        check(count == before + (configured ? 1u : 0u));
    }
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    const Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "basic", basic, 20u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_ResourceConfig resource = {0u, 10u, 1u, 0u};
    const Os_CounterConfig counters[2] = {{0u, 15u, 1u, 1u, 1u}, {1u, 15u, 1u, 1u, 0u}};
    const Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_ACTIVATE, 0u, 0u, NULL, 0u, 0u, 0u, 0u, 0u};
    const Os_ExpiryAction action = {OS_ALARM_ACTIVATE, 1u, 0u};
    const Os_ExpiryPoint point = {0u, &action, 1u};
    const Os_ScheduleTableConfig tables[2] = {
        {0u, 0u, 4u, &point, 1u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 0u},
        {1u, 0u, 4u, &point, 1u, 0u, OS_SCHEDULE_SYNC_NONE, 0u, 0u, 0u},
    };
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    Os_TimeConfig time = {counters,   2u,     &alarm, 1u, 0u, 0u, UINT32_C(0x80000000),
                          &ErrorHook, tables, 2u};
    Os_TargetConfig target = {tasks, 2u, 262144u, &resource, 1u,     NULL, 0u,
                              0u,    0u, 0u,      &time,     &hooks, NULL};
    if (argc == 2 && strcmp(argv[1], "unconfigured") == 0) {
        target.hooks = NULL;
        time.error_hook = NULL;
        configured = false;
    }
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
