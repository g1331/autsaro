/** Diagnostic buffers share the existing recursive CAN resource with callbacks. */
#include "SchM_Dcm.h"
#include "SchM_CanTp.h"
#include "Can_HostLock.h"
void SchM_Enter_Dcm_DCM_STATE(void) { Can_Lock(); }
void SchM_Exit_Dcm_DCM_STATE(void) { Can_Unlock(); }
void SchM_Enter_CanTp_CANTP_STATE(void) { Can_Lock(); }
void SchM_Exit_CanTp_CANTP_STATE(void) { Can_Unlock(); }
