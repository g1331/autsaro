/** @file Host SchM area of Dcm, mapped to the shared legacy state lock. */
/** Dcm is not yet migrated (ADR 0018): its area maps to the shared legacy resource. */
#include "ComStack_Cfg.h"
#include "SchM_Dcm.h"
#include "Can_HostLock.h"
void SchM_Enter_Dcm_DCM_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_Dcm_DCM_STATE(void) { Can_HostStateUnlock(); }
