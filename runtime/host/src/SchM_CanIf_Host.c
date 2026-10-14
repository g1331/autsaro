/** @file Host exclusive areas. The shared CanIf area is the legacy recursive resource that
 * unmigrated modules still use; the driver and Det areas are strict (ADR 0018). The
 * selected target lock provider implements them.
 */
#include "SchM_CanIf.h"
#include "SchM_Can.h"
#include "SchM_Det.h"
#include "Can_HostLock.h"
void SchM_Enter_CanIf_CANIF_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_CanIf_CANIF_STATE(void) { Can_HostStateUnlock(); }
/* Driver and Det areas are strict: separate resources that nest with nothing. */
void SchM_Enter_Can_CAN_STATE(void) { Can_Lock(); }
void SchM_Exit_Can_CAN_STATE(void) { Can_Unlock(); }
void SchM_Enter_Det_DET_STATE(void) { Can_HostDetLock(); }
void SchM_Exit_Det_DET_STATE(void) { Can_HostDetUnlock(); }
