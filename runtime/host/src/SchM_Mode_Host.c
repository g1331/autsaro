/** @file Mode modules are not yet migrated (ADR 0018): their areas map to the shared
 * legacy resource. Configured exclusive-area declarations/provenance belong to full
 * generation.
 */
#include "SchM_ComM.h"
#include "SchM_BswM.h"
#include "Can_HostLock.h"
void SchM_Enter_ComM_COMM_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_ComM_COMM_STATE(void) { Can_HostStateUnlock(); }
void SchM_Enter_BswM_BSWM_STATE(void) { Can_HostStateLock(); }
void SchM_Exit_BswM_BSWM_STATE(void) { Can_HostStateUnlock(); }
