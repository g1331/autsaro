/** @file CanIf admission/state area: a recursive resource distinct from the driver
 * lock and acquired before it. The selected target lock provider implements it.
 */
#include "SchM_CanIf.h"
#include "Can_HostLock.h"
void SchM_Enter_CanIf_CANIF_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_CanIf_CANIF_STATE(void) { Can_HostStateUnlock(); }
