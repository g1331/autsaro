/** @file Host SchM areas of Dcm and CanTp, mapped to the shared legacy state lock. */
/** Diagnostic modules are not yet migrated (ADR 0018): their areas map to the shared
 * legacy resource. */
#include "ComStack_Cfg.h"
#include "SchM_Dcm.h"
#include "SchM_CanTp.h"
#include "Can_HostLock.h"
void SchM_Enter_Dcm_DCM_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_Dcm_DCM_STATE(void) { Can_HostStateUnlock(); }
void SchM_Enter_CanTp_CANTP_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_CanTp_CANTP_STATE(void) { Can_HostStateUnlock(); }
