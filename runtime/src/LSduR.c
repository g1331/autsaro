#include "LSduR.h"
#include "Can.h"
#include "CanIf.h"
#include "CanTp.h"
#include "PduR.h"

static const EcuDiagnosticConfig *active_diagnostic;

void LSduR_Init(const EcuDiagnosticConfig *config) { active_diagnostic = config; }

EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (active_diagnostic != NULL) {
        result = CanIf_TransmitDiagnostic(dlc, data);
    }
    return result;
}

EcuStatus LSduR_CanTpRxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (active_diagnostic != NULL) {
        result = CanTp_RxIndication(dlc, data, now_ms);
    }
    return result;
}

EcuStatus LSduR_PduRTransmit(size_t frame_index, const uint8_t data[8]) {
    return CanIf_Transmit(frame_index, data);
}

void LSduR_CanIfTxConfirmation(PduIdType pdu_id, Std_ReturnType result) {
    PduR_CanIfTxConfirmation(pdu_id, result);
}

EcuStatus LSduR_CanIfRxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms) {
    return PduR_RxIndication(frame_index, data, now_ms);
}
