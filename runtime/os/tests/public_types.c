/** @file Selected SC1 public types meet independent R24-11 constraints; unsupported types are
 * absent. */
#include "Std_Types.h"
#if defined(OS_PUBLIC_TYPES_HOST_BEFORE) && (OS_PUBLIC_TYPES_HOST_BEFORE == STD_ON)
#include "Os_Host.h"
#endif
#if defined(OS_PUBLIC_RTE_BEFORE) && (OS_PUBLIC_RTE_BEFORE == STD_ON)
#include "Rte_Os_Type.h"
#endif
#include "Os.h"
#if !defined(OS_PUBLIC_RTE_BEFORE) || !(OS_PUBLIC_RTE_BEFORE == STD_ON)
#include "Rte_Os_Type.h"
#endif
#if defined(OS_PUBLIC_TYPES_HOST_AFTER) && (OS_PUBLIC_TYPES_HOST_AFTER == STD_ON)
#include "Os_Host.h"
#endif
#include "Os_Target.h"
#include <stdio.h>

/* Independent SC1 consumer constraints from R24-11 and the inherited OSEK services. */
typedef char CoreRange[(sizeof(CoreIdType) >= 2u) ? 1 : -1];
typedef char CounterRange[(sizeof(CounterType) == 4u) ? 1 : -1];
typedef char MicrosecondsRange[(sizeof(TimeInMicrosecondsType) == 8u) ? 1 : -1];

/* Verifies[Os]: SWS_BSW_00120.
 * Integration ABI expectations
 * are fixed independently of the implementation's choice of count typedef. */
typedef char TaskCountWidth[(sizeof(((Os_TargetConfigType *)0)->task_count) == 4u) ? 1 : -1];
typedef char
    ResourceCountWidth[(sizeof(((Os_TargetConfigType *)0)->resource_count) == 4u) ? 1 : -1];
typedef char InternalCountWidth[(sizeof(((Os_TargetConfigType *)0)->internal_resource_count) == 4u)
                                    ? 1
                                    : -1];
typedef char CounterCountWidth[(sizeof(((Os_TimeConfigType *)0)->counter_count) == 4u) ? 1 : -1];
typedef char AlarmCountWidth[(sizeof(((Os_TimeConfigType *)0)->alarm_count) == 4u) ? 1 : -1];
typedef char
    TableCountWidth[(sizeof(((Os_TimeConfigType *)0)->schedule_table_count) == 4u) ? 1 : -1];
typedef char
    PointCountWidth[(sizeof(((Os_ScheduleTableConfigType *)0)->point_count) == 4u) ? 1 : -1];
typedef char ActionCountWidth[(sizeof(((Os_ExpiryPointType *)0)->action_count) == 4u) ? 1 : -1];
/* OSEK 13.2.4 types INVALID_TASK as TaskType; the selected ISR sentinel also keeps its type. */
typedef char InvalidTaskTypeWidth[(sizeof(INVALID_TASK) == sizeof(TaskType)) ? 1 : -1];
typedef char InvalidIsrTypeWidth[(sizeof(INVALID_ISR) == sizeof(ISRType)) ? 1 : -1];

int main(void)
{
    CoreIdType core = 65533u;
    TaskType task = INVALID_TASK;
    TaskRefType task_ref = &task;
    TaskStateType state = READY;
    TaskStateRefType state_ref = &state;
    EventMaskType events = 0u;
    EventMaskRefType event_ref = &events;
    TickType tick = UINT32_MAX;
    TickRefType tick_ref = &tick;
    ISRType interrupt = INVALID_ISR;
    PhysicalTimeType time = UINT64_C(4294967295000000);
    IdleModeType idle = IDLE_NO_HALT;
    OSServiceIdType service = 0x36u;
    boolean clear_pending = TRUE;
    CounterType counter = UINT32_MAX;
    TimeInMicrosecondsType microseconds = UINT64_MAX;
    AppModeType inherited = DONOTCARE;
    *task_ref = 0u;
    *state_ref = SUSPENDED;
    *event_ref = UINT32_C(0x80000000);
    *tick_ref = 1u;
    if ((core != 65533u) || (task != 0u) || (state != SUSPENDED) ||
        (events != UINT32_C(0x80000000)) || (tick != 1u) || (interrupt != INVALID_ISR) ||
        (time != UINT64_C(4294967295000000)) || (idle != IDLE_NO_HALT) || (service != 0x36u) ||
        (sizeof(boolean) != 1u) || (clear_pending != TRUE) || (TRUE == FALSE) ||
        (counter != UINT32_MAX) || (microseconds != UINT64_MAX) || (inherited == 1u) ||
        (inherited == 2u) || (TotalNumberOfCores != 1u) || (OS_CORE_ID_MASTER != 0u) ||
        (OSDEFAULTAPPMODE == DONOTCARE) || (OSDEFAULTAPPMODE == 0u)) {
        return 1;
    }
    puts("public_types sc1_range_and_pointer_contracts=pass");
    return 0;
}
