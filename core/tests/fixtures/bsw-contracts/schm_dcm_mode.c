/* Test SchM for a partial stack without connected mode-switch events: the
 * provided Dcm session mode is stored synchronously under the SchM mode area.
 * Full generation supplies the real SchM with BSW mode-group relations.
 */
#include "SchM_Dcm.h"
#include "Can_HostLock.h"

static Rte_ModeType_DcmDiagnosticSessionControl diagnostic_session_mode;

Std_ReturnType
SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode) {
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    diagnostic_session_mode = mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return E_OK;
}
Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void) {
    Rte_ModeType_DcmDiagnosticSessionControl mode;
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    mode = diagnostic_session_mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return mode;
}
