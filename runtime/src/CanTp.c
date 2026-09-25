#include "CanTp.h"
#include "LSduR.h"
#include "PduR.h"

/* N-SDU storage belongs to PduR; only one eight-byte N-PDU is staged at a time. */
typedef struct {
    size_t length;
    size_t received;
    uint64_t last_cf_ms;
    uint8_t next_sn;
    uint8_t active;
} CanTpRxState;

typedef struct {
    size_t length;
    size_t sent;
    uint64_t wait_started_ms;
    uint64_t last_cf_ms;
    uint32_t stmin_ms;
    uint8_t next_sn;
    uint8_t block_size;
    uint8_t block_remaining;
    uint8_t waiting_fc;
    uint8_t sent_cf;
    uint8_t active;
} CanTpTxState;

static const EcuDiagnosticConfig *active_config;
static CanTpRxState rx;
static CanTpTxState tx;

static void AbortRx(void)
{
    if (rx.active) {
        rx.active = 0u;
        PduR_CanTpRxAbort();
    }
}

static EcuStatus FinishRx(uint64_t now_ms)
{
    rx.active = 0u;
    return PduR_CanTpRxIndication(now_ms);
}

static EcuStatus FinishTx(EcuStatus status, uint64_t now_ms)
{
    tx.active = 0u;
    tx.waiting_fc = 0u;
    PduR_CanTpTxConfirmation(status, now_ms);
    return status;
}

static EcuStatus SendConsecutiveFrames(uint64_t now_ms)
{
    while (tx.active && !tx.waiting_fc && tx.sent < tx.length) {
        uint8_t frame[8] = {0};
        size_t count = tx.length - tx.sent;
        EcuStatus result;

        if (tx.sent_cf && now_ms - tx.last_cf_ms < tx.stmin_ms) {
            return ECU_OK;
        }
        if (count > 7u) {
            count = 7u;
        }
        frame[0] = (uint8_t)(0x20u | tx.next_sn);
        result = PduR_CanTpCopyTxData(tx.sent, &frame[1], count);
        if (result != ECU_OK) {
            return FinishTx(result, now_ms);
        }
        result = LSduR_CanTpTransmit((uint8_t)(count + 1u), frame);
        if (result != ECU_OK) {
            return FinishTx(result, now_ms);
        }
        tx.sent += count;
        tx.next_sn = (uint8_t)((tx.next_sn + 1u) & 0x0fu);
        tx.last_cf_ms = now_ms;
        tx.sent_cf = 1u;
        if (tx.sent == tx.length) {
            return FinishTx(ECU_OK, now_ms);
        }
        if (tx.block_size != 0u && --tx.block_remaining == 0u) {
            tx.waiting_fc = 1u;
            tx.wait_started_ms = now_ms;
            return ECU_OK;
        }
    }
    return ECU_OK;
}

void CanTp_Init(const EcuDiagnosticConfig *config)
{
    AbortRx();
    active_config = config;
    rx.active = 0u;
    tx.active = 0u;
    tx.waiting_fc = 0u;
}

EcuStatus CanTp_AdvanceTime(uint64_t now_ms)
{
    uint8_t expired = 0u;
    if (active_config == NULL) {
        return ECU_OK;
    }
    if ((rx.active && now_ms < rx.last_cf_ms) ||
        (tx.active && tx.waiting_fc && now_ms < tx.wait_started_ms) ||
        (tx.active && tx.sent_cf && now_ms < tx.last_cf_ms)) {
        return ECU_ERR_TIME;
    }
    if (rx.active && now_ms - rx.last_cf_ms >= active_config->n_cr_ms) {
        AbortRx();
        expired = 1u;
    }
    if (tx.active && tx.waiting_fc && now_ms - tx.wait_started_ms >= active_config->n_bs_ms) {
        (void)FinishTx(ECU_ERR_TP_TIMEOUT, now_ms);
        expired = 1u;
    }
    if (expired) {
        return ECU_ERR_TP_TIMEOUT;
    }
    return SendConsecutiveFrames(now_ms);
}

static EcuStatus ReceiveSingle(uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    size_t length = (size_t)(data[0] & 0x0fu);
    EcuStatus result;
    AbortRx();
    if (length == 0u || length > 7u) {
        return ECU_ERR_TP_LENGTH;
    }
    if ((size_t)dlc < length + 1u) {
        return ECU_ERR_FRAME_DLC;
    }
    result = PduR_CanTpStartOfReception(length);
    if (result != ECU_OK) {
        return result;
    }
    rx.active = 1u;
    result = PduR_CanTpCopyRxData(&data[1], length);
    if (result != ECU_OK) {
        AbortRx();
        return result;
    }
    return FinishRx(now_ms);
}

static EcuStatus ReceiveFirst(uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    size_t length = ((size_t)(data[0] & 0x0fu) << 8u) | data[1];
    uint8_t flow_control[8] = {0x30u, 0u, 0u};
    EcuStatus result;
    AbortRx();
    if (dlc != 8u) {
        return ECU_ERR_FRAME_DLC;
    }
    if (length > ECU_DIAG_MAX_PAYLOAD) {
        flow_control[0] = 0x32u; /* FC(OVFLW): no N-SDU was delivered. */
        result = LSduR_CanTpTransmit(3u, flow_control);
        return result == ECU_OK ? ECU_ERR_TP_LENGTH : result;
    }
    if (length <= 7u) {
        return ECU_ERR_TP_LENGTH;
    }
    result = PduR_CanTpStartOfReception(length);
    if (result != ECU_OK) {
        return result;
    }
    rx.active = 1u;
    rx.length = length;
    rx.received = 0u;
    rx.next_sn = 1u;
    result = PduR_CanTpCopyRxData(&data[2], 6u);
    if (result != ECU_OK) {
        AbortRx();
        return result;
    }
    rx.received = 6u;
    result = LSduR_CanTpTransmit(3u, flow_control);
    if (result != ECU_OK) {
        AbortRx();
        return result;
    }
    rx.last_cf_ms = now_ms;
    return ECU_OK;
}

static EcuStatus ReceiveConsecutive(uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    size_t count;
    EcuStatus result;
    if (!rx.active) {
        return ECU_ERR_TP_SEQUENCE;
    }
    if ((data[0] & 0x0fu) != rx.next_sn) {
        AbortRx();
        return ECU_ERR_TP_SEQUENCE;
    }
    count = rx.length - rx.received;
    if (count > 7u) {
        count = 7u;
        if (dlc != 8u) {
            AbortRx();
            return ECU_ERR_FRAME_DLC;
        }
    } else if ((size_t)dlc < count + 1u) {
        AbortRx();
        return ECU_ERR_FRAME_DLC;
    }
    result = PduR_CanTpCopyRxData(&data[1], count);
    if (result != ECU_OK) {
        AbortRx();
        return result;
    }
    rx.received += count;
    rx.next_sn = (uint8_t)((rx.next_sn + 1u) & 0x0fu);
    if (rx.received == rx.length) {
        return FinishRx(now_ms);
    }
    rx.last_cf_ms = now_ms;
    return ECU_OK;
}

static EcuStatus ReceiveFlowControl(uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    uint8_t flow_status = data[0] & 0x0fu;
    uint8_t stmin;
    if (!tx.active || !tx.waiting_fc) {
        return ECU_ERR_TP_FLOW;
    }
    if (dlc < 3u || flow_status > 2u) {
        return FinishTx(ECU_ERR_TP_FLOW, now_ms);
    }
    if (flow_status == 2u) {
        return FinishTx(ECU_ERR_TP_FLOW, now_ms);
    }
    if (flow_status == 1u) {
        tx.wait_started_ms = now_ms;
        return ECU_OK;
    }
    stmin = data[2];
    if (stmin > 0x7fu && (stmin < 0xf1u || stmin > 0xf9u)) {
        return FinishTx(ECU_ERR_TP_FLOW, now_ms);
    }
    tx.stmin_ms = stmin <= 0x7fu ? stmin : 1u; /* 100-us values round up to the host's ms clock. */
    tx.block_size = data[1];
    tx.block_remaining = data[1];
    tx.waiting_fc = 0u;
    return SendConsecutiveFrames(now_ms);
}

EcuStatus CanTp_RxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    EcuStatus result;
    if (active_config == NULL || data == NULL) {
        return ECU_ERR_CONFIG;
    }
    result = CanTp_AdvanceTime(now_ms);
    if (result != ECU_OK) {
        return result;
    }
    if (dlc < 1u || dlc > 8u) {
        AbortRx();
        if (dlc > 8u && (data[0] >> 4u) == 3u && tx.active && tx.waiting_fc) {
            return FinishTx(ECU_ERR_TP_FLOW, now_ms);
        }
        return ECU_ERR_FRAME_DLC;
    }
    switch (data[0] >> 4u) {
    case 0u:
        return ReceiveSingle(dlc, data, now_ms);
    case 1u:
        if (dlc < 2u) {
            AbortRx();
            return ECU_ERR_FRAME_DLC;
        }
        return ReceiveFirst(dlc, data, now_ms);
    case 2u:
        return ReceiveConsecutive(dlc, data, now_ms);
    case 3u:
        return ReceiveFlowControl(dlc, data, now_ms);
    default:
        AbortRx();
        return ECU_ERR_TP_FLOW;
    }
}

EcuStatus CanTp_Transmit(size_t length, uint64_t now_ms)
{
    uint8_t frame[8] = {0};
    size_t count;
    EcuStatus result;
    if (active_config == NULL) {
        return ECU_ERR_CONFIG;
    }
    if (tx.active) {
        return ECU_ERR_TP_BUSY;
    }
    if (length == 0u || length > ECU_DIAG_MAX_PAYLOAD) {
        return ECU_ERR_TP_LENGTH;
    }
    tx.active = 1u;
    tx.length = length;
    tx.sent = 0u;
    tx.sent_cf = 0u;
    tx.next_sn = 1u;
    tx.waiting_fc = 0u;
    if (length <= 7u) {
        frame[0] = (uint8_t)length;
        count = length;
    } else {
        frame[0] = (uint8_t)(0x10u | (length >> 8u));
        frame[1] = (uint8_t)length;
        count = 6u;
    }
    result = PduR_CanTpCopyTxData(0u, &frame[length <= 7u ? 1u : 2u], count);
    if (result != ECU_OK) {
        return FinishTx(result, now_ms);
    }
    result = LSduR_CanTpTransmit((uint8_t)(length <= 7u ? length + 1u : 8u), frame);
    if (result != ECU_OK) {
        return FinishTx(result, now_ms);
    }
    if (length <= 7u) {
        return FinishTx(ECU_OK, now_ms);
    }
    tx.sent = count;
    tx.waiting_fc = 1u;
    tx.wait_started_ms = now_ms;
    return ECU_OK;
}
