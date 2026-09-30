#include "Os_Target.h"

/* Standard configured callbacks are bounded and do not perform host I/O.
 * Standard ARTI events and object/state binding are supplied by the OS tool binding. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
void ErrorHook(StatusType Error) {
    (void)Error;
    Os_TargetTrace('e');
}
void PreTaskHook(void) { Os_TargetTrace('p'); }
void PostTaskHook(void) { Os_TargetTrace('q'); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
