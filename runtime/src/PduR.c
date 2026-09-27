#include "PduR.h"
#include <string.h>
#include "CanTp.h"
#include "Com.h"
#include "Dcm.h"
#include "LSduR.h"

static const EcuConfig *pdur_config;
static uint8_t rx_data[ECU_DIAG_MAX_PAYLOAD];
static uint8_t tx_data[ECU_DIAG_MAX_PAYLOAD];
static size_t rx_expected;
static size_t rx_received;
static size_t pdur_tx_length;
static uint8_t rx_active;
static uint8_t tx_active;

void PduR_Init(const EcuConfig *config) {
    pdur_config = config;
    rx_expected = 0u;
    rx_received = 0u;
    pdur_tx_length = 0u;
    rx_active = 0u;
    tx_active = 0u;
}

EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8]) {
    return LSduR_PduRTransmit(frame_index, data);
}

void PduR_CanIfTxConfirmation(PduIdType pdu_id, Std_ReturnType result) {
    if (pdur_config != NULL) {
        if ((size_t)pdu_id < pdur_config->frame_count) {
            Com_TxConfirmation(pdu_id, result);
        } else if (((size_t)pdu_id == pdur_config->frame_count) &&
                   (pdur_config->diagnostic != NULL)) {
            CanTp_TxConfirmation(pdu_id, result);
        } else {
            /* No generated upper-layer route corresponds to this handle. */
        }
    }
}

EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms) {
    return Com_RxIndication(frame_index, data, now_ms);
}

EcuStatus PduR_CanTpStartOfReception(size_t length) {
    EcuStatus result = ECU_ERR_TP_LENGTH;
    if ((pdur_config->diagnostic != NULL) && (length != 0u) && (length <= sizeof(rx_data))) {
        if ((rx_active != 0u) || (tx_active != 0u)) {
            result = ECU_ERR_TP_BUSY;
        } else {
            rx_expected = length;
            rx_received = 0u;
            rx_active = 1u;
            result = ECU_OK;
        }
    }
    return result;
}

EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length) {
    EcuStatus result = ECU_ERR_TP_LENGTH;
    if ((rx_active != 0u) && (data != NULL) && (length <= (rx_expected - rx_received))) {
        (void)memcpy(&rx_data[rx_received], data, length);
        rx_received += length;
        result = ECU_OK;
    }
    return result;
}

EcuStatus PduR_CanTpRxIndication(uint64_t now_ms) {
    EcuStatus result = ECU_ERR_TP_LENGTH;
    if ((rx_active != 0u) && (rx_received == rx_expected)) {
        rx_active = 0u;
        result = Dcm_RxIndication(rx_data, rx_received, now_ms);
    }
    return result;
}

void PduR_CanTpRxAbort(void) {
    rx_active = 0u;
    rx_expected = 0u;
    rx_received = 0u;
}

EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length) {
    EcuStatus result = ECU_ERR_TP_LENGTH;
    if ((tx_active != 0u) && (destination != NULL) && (offset <= pdur_tx_length) &&
        (length <= (pdur_tx_length - offset))) {
        (void)memcpy(destination, &tx_data[offset], length);
        result = ECU_OK;
    }
    return result;
}

void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms) {
    tx_active = 0u;
    pdur_tx_length = 0u;
    Dcm_TpTxConfirmation(status, now_ms);
}

EcuStatus PduR_DcmTransmit(const uint8_t *data, size_t length, uint64_t now_ms) {
    EcuStatus result = ECU_ERR_TP_LENGTH;
    if ((pdur_config->diagnostic != NULL) && (data != NULL) && (length != 0u) &&
        (length <= sizeof(tx_data))) {
        if ((tx_active != 0u) || (rx_active != 0u)) {
            result = ECU_ERR_TP_BUSY;
        } else {
            (void)memcpy(tx_data, data, length);
            pdur_tx_length = length;
            tx_active = 1u;
            result = CanTp_Transmit(length, now_ms);
            if ((result != ECU_OK) && (tx_active != 0u)) {
                tx_active = 0u;
                pdur_tx_length = 0u;
            }
        }
    }
    return result;
}
