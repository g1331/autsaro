#include "Dcm.h"
#include "PduR.h"
#include "Rte.h"

static const EcuDiagnosticConfig *active_config;
static uint8_t active_session;
static uint8_t pending_session;
static uint64_t last_request_ms;

static EcuStatus NegativeResponse(uint8_t service, uint8_t code, uint64_t now_ms)
{
    const uint8_t response[3] = {0x7fu, service, code};
    return PduR_DcmTransmit(response, sizeof(response), now_ms);
}

void Dcm_Init(const EcuDiagnosticConfig *config)
{
    active_config = config;
    active_session = 0x01u;
    pending_session = 0u;
    last_request_ms = 0u;
}

void Dcm_TpTxConfirmation(EcuStatus status, uint64_t now_ms)
{
    if (pending_session != 0u) {
        if (status == ECU_OK) {
            active_session = pending_session;
            last_request_ms = now_ms;
        }
        pending_session = 0u;
    }
}

void Dcm_AdvanceTime(uint64_t now_ms)
{
    if (active_config != NULL && active_session != 0x01u &&
        now_ms - last_request_ms >= active_config->s3_ms) {
        active_session = 0x01u;
        pending_session = 0u;
    }
}

EcuStatus Dcm_RxIndication(const uint8_t *request, size_t length, uint64_t now_ms)
{
    uint8_t response[3u + 4u * ECU_DIAG_MAX_DID_SIGNALS];
    EcuStatus result;
    size_t i;
    if (active_config == NULL || request == NULL || length == 0u) {
        return ECU_ERR_CONFIG;
    }
    last_request_ms = now_ms;
    switch (request[0]) {
    case 0x10u:
        if (length != 2u) {
            return NegativeResponse(0x10u, 0x13u, now_ms);
        }
        if (request[1] != 0x01u && request[1] != 0x03u) {
            return NegativeResponse(0x10u, 0x12u, now_ms);
        }
        response[0] = 0x50u;
        response[1] = request[1];
        response[2] = 0x00u;
        response[3] = 0x32u; /* P2ServerMax: 50 ms */
        response[4] = 0x00u;
        response[5] = 0x32u; /* P2*ServerMax: 500 ms in 10 ms units */
        pending_session = request[1];
        result = PduR_DcmTransmit(response, 6u, now_ms);
        if (result != ECU_OK) {
            pending_session = 0u;
        }
        return result;
    case 0x3eu:
        if (length != 2u) {
            return NegativeResponse(0x3eu, 0x13u, now_ms);
        }
        if (request[1] == 0x80u) {
            return ECU_OK;
        }
        if (request[1] != 0x00u) {
            return NegativeResponse(0x3eu, 0x12u, now_ms);
        }
        response[0] = 0x7eu;
        response[1] = 0x00u;
        return PduR_DcmTransmit(response, 2u, now_ms);
    case 0x22u:
        if (length != 3u) {
            return NegativeResponse(0x22u, 0x13u, now_ms);
        }
        if (((uint16_t)request[1] << 8u | request[2]) != active_config->did ||
            active_session != 0x03u) {
            return NegativeResponse(0x22u, 0x31u, now_ms);
        }
        response[0] = 0x62u;
        response[1] = request[1];
        response[2] = request[2];
        for (i = 0u; i < active_config->did_signal_count; ++i) {
            uint32_t value;
            uint8_t valid;
            result = Rte_ReadSignal(active_config->did_signal_ids[i], &value, &valid);
            if (result != ECU_OK || valid == 0u) {
                return NegativeResponse(0x22u, 0x22u, now_ms);
            }
            response[3u + 4u * i] = (uint8_t)(value >> 24u);
            response[4u + 4u * i] = (uint8_t)(value >> 16u);
            response[5u + 4u * i] = (uint8_t)(value >> 8u);
            response[6u + 4u * i] = (uint8_t)value;
        }
        return PduR_DcmTransmit(response, 3u + 4u * active_config->did_signal_count, now_ms);
    default:
        return NegativeResponse(request[0], 0x11u, now_ms);
    }
}
