/** @file Controlled-host critical area sharing the actual driver lock.
 * The final generated SchM declarations must come from the selected BSWMD.
 * One recursive resource avoids CanIf/driver callback lock-order inversion.
 */
#include "SchM_CanIf.h"
#include "Can_HostLock.h"
void SchM_Enter_CanIf_CANIF_STATE(void) { Can_Lock(); }
void SchM_Exit_CanIf_CANIF_STATE(void) { Can_Unlock(); }
