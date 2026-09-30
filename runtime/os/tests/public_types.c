#ifdef OS_PUBLIC_TYPES_WINDOWS_BEFORE
#include "Os_Windows.h"
#endif
#ifdef OS_PUBLIC_RTE_BEFORE
#include "Rte_Os_Type.h"
#endif
#include "Os.h"
#ifndef OS_PUBLIC_RTE_BEFORE
#include "Rte_Os_Type.h"
#endif
#ifdef OS_PUBLIC_TYPES_WINDOWS_AFTER
#include "Os_Windows.h"
#endif
#include <stdio.h>

/* Independent consumer constraints from R24-11 sections 8.3.1/8.3.18/8.3.19
 * and 8.3.22, not sizes inferred from this implementation's typedefs. */
typedef char ApplicationRange[(sizeof(ApplicationType) == 4u) ? 1 : -1];
typedef char CoreRange[(sizeof(CoreIdType) >= 2u) ? 1 : -1];
typedef char SpinlockRange[(sizeof(SpinlockIdType) >= 2u) ? 1 : -1];
typedef char AreaRange[(sizeof(AreaIdType) >= 2u) ? 1 : -1];
typedef char CounterRange[(sizeof(CounterType) == 4u) ? 1 : -1];
typedef char MicrosecondsRange[(sizeof(TimeInMicrosecondsType) == 8u) ? 1 : -1];

int main(void) {
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
