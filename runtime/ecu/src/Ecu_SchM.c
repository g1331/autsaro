#include "Can_HostLock.h"
#include "Ecu_Target.h"

/* The validated profile proves that all CAN/BSW calls execute on StartupHook
 * or the sole Task_Ecu. Native input/output touches only atomic mailboxes. */
void Can_Lock(void) { Ecu_TargetAssertOwner(); }
int Can_TryLock(void) {
    Ecu_TargetAssertOwner();
    return 1;
}
void Can_Unlock(void) { Ecu_TargetAssertOwner(); }
