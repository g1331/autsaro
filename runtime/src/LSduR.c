#include "LSduR.h"
#include "Can.h"
#include "CanIf.h"
#include "CanTp.h"
#include "PduR.h"

static const EcuDiagnosticConfig *active_diagnostic;

void LSduR_Init(const EcuDiagnosticConfig *config) { active_diagnostic = config; }

EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]) {
    if (active_diagnostic == NULL) {
        return ECU_ERR_CONFIG;
    }
    return Can_Transmit(active_diagnostic->response_can_id, dlc, data);
}

EcuStatus LSduR_CanTpRxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    if (active_diagnostic == NULL) {
        return ECU_ERR_CONFIG;
    }
    return CanTp_RxIndication(dlc, data, now_ms);
}

EcuStatus LSduR_PduRTransmit(size_t frame_index, const uint8_t data[8]) {
    return CanIf_Transmit(frame_index, data);
}

EcuStatus LSduR_CanIfRxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms) {
    return PduR_RxIndication(frame_index, data, now_ms);
}
