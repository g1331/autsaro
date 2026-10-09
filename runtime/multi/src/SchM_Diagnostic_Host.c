/** Diagnostic buffers share the existing recursive CAN resource with callbacks. */
#include "SchM_Dcm.h"
#include "SchM_CanTp.h"
#include "Can_HostLock.h"
void SchM_Enter_Dcm_DCM_STATE(void) { Can_Lock(); }
void SchM_Exit_Dcm_DCM_STATE(void) { Can_Unlock(); }
void SchM_Enter_CanTp_CANTP_STATE(void) { Can_Lock(); }
void SchM_Exit_CanTp_CANTP_STATE(void) { Can_Unlock(); }

/* No connected mode-switch events in this selected profile. Publication still
 * stores the actual provided mode synchronously under the existing resource.
 * Full generation supplies matching BSW mode-group relations and ownership.
 */
#define DCM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
static Rte_ModeType_DcmDiagnosticSessionControl diagnostic_session_mode DCM_VAR_CLEARED;
#define DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
DCM_CODE Std_ReturnType
SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode) {
    Can_Lock();
    diagnostic_session_mode = mode;
    Can_Unlock();
    return E_OK;
}
DCM_CODE Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void) {
    Rte_ModeType_DcmDiagnosticSessionControl mode;
    Can_Lock();
    mode = diagnostic_session_mode;
    Can_Unlock();
    return mode;
}

#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
