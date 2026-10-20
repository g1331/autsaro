/** @file Os.h public types and access macros meet the independent R24-11 constraints. */
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

/* Independent consumer constraints from R24-11 sections 8.3.1/8.3.18/8.3.19
 * and 8.3.22, not sizes inferred from this implementation's typedefs. */
typedef char ApplicationRange[(sizeof(ApplicationType) == 4u) ? 1 : -1];
typedef char CoreRange[(sizeof(CoreIdType) >= 2u) ? 1 : -1];
typedef char SpinlockRange[(sizeof(SpinlockIdType) >= 2u) ? 1 : -1];
typedef char AreaRange[(sizeof(AreaIdType) >= 2u) ? 1 : -1];
typedef char CounterRange[(sizeof(CounterType) == 4u) ? 1 : -1];
typedef char MicrosecondsRange[(sizeof(TimeInMicrosecondsType) == 8u) ? 1 : -1];

/* Verifies[Os]: SWS_BSW_00120.
 * Verifies: SWS_Os_00781. Integration ABI expectations
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
typedef char MemorySizeWidth[(sizeof(MemorySizeType) == 8u) ? 1 : -1];

int main(void)
{
    ApplicationType application = UINT32_C(4294967294);
    ApplicationStateType state = APPLICATION_ACCESSIBLE;
    ApplicationStateRefType state_ref = &state;
    TrustedFunctionIndexType trusted = 0u;
    TrustedFunctionParameterRefType parameters = &trusted;
    MemoryStartAddressType address = &state;
    MemorySizeType size = sizeof(state);
    CoreIdType core = 65533u;
    SpinlockIdType spinlock = 65535u;
    AreaIdType area = 65534u;
    TaskType task = INVALID_TASK;
    TaskRefType task_ref = &task;
    ISRType interrupt = INVALID_ISR;
    ObjectAccessType access = ACCESS;
    ObjectTypeType object = OBJECT_SCHEDULETABLE;
    ProtectionReturnType protection = PRO_PREVENT_ARRIVAL_RATE;
    RestartType restart = OS_OSAPPLICATION_RESTART;
    PhysicalTimeType time = UINT64_C(4294967295000000);
    TryToGetSpinlockType acquired = TRYTOGETSPINLOCK_SUCCESS;
    IdleModeType idle = IDLE_NO_HALT;
    OSServiceIdType service = 0x36u;
    boolean clear_pending = TRUE;
    CounterType counter = UINT32_MAX;
    TimeInMicrosecondsType microseconds = UINT64_MAX;
    AppModeType inherited = DONOTCARE;
    *state_ref = APPLICATION_TERMINATED;
    *task_ref = 0u;
    if ((application == INVALID_OSAPPLICATION) || (state != APPLICATION_TERMINATED) ||
        (parameters != &trusted) || (address != &state) || (size != 1u) || (core != 65533u) ||
        (spinlock != 65535u) || (area != 65534u) || (task != 0u) || (interrupt != INVALID_ISR) ||
        (access == NO_ACCESS) || (object != OBJECT_SCHEDULETABLE) ||
        (protection != PRO_PREVENT_ARRIVAL_RATE) || (restart != OS_OSAPPLICATION_RESTART) ||
        (time != UINT64_C(4294967295000000)) || (acquired == TRYTOGETSPINLOCK_NOSUCCESS) ||
        (idle != IDLE_NO_HALT) || (service != 0x36u) || (sizeof(boolean) != 1u) ||
        (clear_pending != TRUE) || (TRUE == FALSE) || (counter != UINT32_MAX) ||
        (microseconds != UINT64_MAX) || (inherited == 1u) || (inherited == 2u) ||
        (TotalNumberOfCores != 1u) || (OS_CORE_ID_MASTER != 0u) ||
        (INVALID_OSAPPLICATION != UINT32_MAX)) {
        return 1;
    }
    /* SWS_Os_00779 and 00787 name distinct values; INVALID_SPINLOCK is 0 (SWS_Os_00791);
     * OSDEFAULTAPPMODE is a valid StartOS mode (OSEK 13.7.3), not DONOTCARE. */
    {
        const ObjectTypeType objects[] = {OBJECT_TASK,
                                          OBJECT_ISR,
                                          OBJECT_ALARM,
                                          OBJECT_RESOURCE,
                                          OBJECT_COUNTER,
                                          OBJECT_SCHEDULETABLE};
        const ProtectionReturnType reactions[] = {PRO_IGNORE,
                                                  PRO_TERMINATETASKISR,
                                                  PRO_TERMINATEAPPL,
                                                  PRO_SHUTDOWN,
                                                  PRO_PREVENT_ARRIVAL_RATE};
        for (size_t i = 0u; i < 6u; ++i) {
            for (size_t j = i + 1u; j < 6u; ++j) {
                if ((objects[i] == objects[j]) || ((j < 5u) && (reactions[i] == reactions[j]))) {
                    return 3;
                }
            }
        }
        if ((INVALID_SPINLOCK != 0u) || (OSDEFAULTAPPMODE == DONOTCARE) ||
            (OSDEFAULTAPPMODE == 0u)) {
            return 3;
        }
    }
    /* The selected host AccessType representation uses read/write/execute/stack
     * bits 1/2/4/8. All predicates must be independent, including zero access. */
    for (AccessType bits = 0u; bits < 16u; ++bits) {
        if ((OSMEMORY_IS_READABLE(bits) != ((bits % 2u) != 0u)) ||
            (OSMEMORY_IS_WRITEABLE(bits) != (((bits / 2u) % 2u) != 0u)) ||
            (OSMEMORY_IS_EXECUTABLE(bits) != (((bits / 4u) % 2u) != 0u)) ||
            (OSMEMORY_IS_STACKSPACE(bits) != ((bits / 8u) != 0u))) {
            return 2;
        }
    }
    puts("public_types range_and_pointer_contracts=pass access_truth_table=16");
    return 0;
}
