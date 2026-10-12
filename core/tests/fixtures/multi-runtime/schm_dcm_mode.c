/* Test SchM for a partial stack without connected mode-switch events: the
 * provided Dcm session mode is stored synchronously under the CanIf resource.
 * Full generation supplies the real SchM with BSW mode-group relations.
 */
#include "SchM_Dcm.h"
#include "SchM_CanIf.h"

static Rte_ModeType_DcmDiagnosticSessionControl diagnostic_session_mode;

Std_ReturnType
SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode) {
    SchM_Enter_CanIf_CANIF_STATE();
    diagnostic_session_mode = mode;
    SchM_Exit_CanIf_CANIF_STATE();
    return E_OK;
}
Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void) {
    Rte_ModeType_DcmDiagnosticSessionControl mode;
    SchM_Enter_CanIf_CANIF_STATE();
    mode = diagnostic_session_mode;
    SchM_Exit_CanIf_CANIF_STATE();
    return mode;
}
