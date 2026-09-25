#include "PduR.h"
#include <string.h>
#include "CanTp.h"
#include "Com.h"
#include "Dcm.h"
#include "LSduR.h"

static const EcuConfig *active_config;
static uint8_t rx_data[ECU_DIAG_MAX_PAYLOAD];
static uint8_t tx_data[ECU_DIAG_MAX_PAYLOAD];
static size_t rx_expected;
static size_t rx_received;
static size_t tx_length;
static uint8_t rx_active;
static uint8_t tx_active;

void PduR_Init(const EcuConfig *config)
{
    active_config = config;
    rx_expected = 0u;
    rx_received = 0u;
    tx_length = 0u;
    rx_active = 0u;
    tx_active = 0u;
}

EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8])
{
    return LSduR_PduRTransmit(frame_index, data);
}

EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms)
{
    return Com_RxIndication(frame_index, data, now_ms);
}

EcuStatus PduR_CanTpStartOfReception(size_t length)
{
    if (active_config->diagnostic == NULL || length == 0u || length > sizeof(rx_data)) {
        return ECU_ERR_TP_LENGTH;
    }
    if (rx_active || tx_active) {
        return ECU_ERR_TP_BUSY;
    }
    rx_expected = length;
    rx_received = 0u;
    rx_active = 1u;
    return ECU_OK;
}

EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length)
{
    if (!rx_active || data == NULL || length > rx_expected - rx_received) {
        return ECU_ERR_TP_LENGTH;
    }
    memcpy(&rx_data[rx_received], data, length);
    rx_received += length;
    return ECU_OK;
}

EcuStatus PduR_CanTpRxIndication(uint64_t now_ms)
{
    if (!rx_active || rx_received != rx_expected) {
        return ECU_ERR_TP_LENGTH;
    }
    rx_active = 0u;
    return Dcm_RxIndication(rx_data, rx_received, now_ms);
}

void PduR_CanTpRxAbort(void)
{
    rx_active = 0u;
    rx_expected = 0u;
    rx_received = 0u;
}

EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length)
{
    if (!tx_active || destination == NULL || offset > tx_length || length > tx_length - offset) {
        return ECU_ERR_TP_LENGTH;
    }
    memcpy(destination, &tx_data[offset], length);
    return ECU_OK;
}

void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms)
{
    tx_active = 0u;
    tx_length = 0u;
    Dcm_TpTxConfirmation(status, now_ms);
}

EcuStatus PduR_DcmTransmit(const uint8_t *data, size_t length, uint64_t now_ms)
{
    EcuStatus status;
    if (active_config->diagnostic == NULL || data == NULL || length == 0u || length > sizeof(tx_data)) {
        return ECU_ERR_TP_LENGTH;
    }
    if (tx_active || rx_active) {
        return ECU_ERR_TP_BUSY;
    }
    memcpy(tx_data, data, length);
    tx_length = length;
    tx_active = 1u;
    status = CanTp_Transmit(length, now_ms);
    if (status != ECU_OK && tx_active) {
        tx_active = 0u;
        tx_length = 0u;
    }
    return status;
}
