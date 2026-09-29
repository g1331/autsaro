#include "CanTp.h"
#include "LSduR.h"
#include "PduR.h"
#include "Std_Types.h"

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

typedef enum {
    CANTP_FRAME_NONE,
    CANTP_FRAME_SINGLE,
    CANTP_FRAME_FIRST,
    CANTP_FRAME_CONSECUTIVE,
    CANTP_FRAME_FLOW_CONTROL,
    CANTP_FRAME_OVERFLOW
} CanTpFrameKind;

static const EcuDiagnosticConfig *cantp_config;
static CanTpRxState rx;
static CanTpTxState tx;
static CanTpFrameKind frame_kind;
static uint8_t frame_pending;
static uint8_t frame_confirmation_received;
static uint8_t frame_confirmation_ok;
static uint8_t frame_aborted;
static size_t frame_data_count;
static uint64_t frame_started_ms;

static EcuStatus SendFrame(uint8_t dlc, const uint8_t data[8], uint64_t now_ms, CanTpFrameKind kind,
                           size_t data_count) {
    EcuStatus result = ECU_ERR_CAN_BUSY;
    if (frame_pending == 0u) {
        frame_kind = kind;
        frame_data_count = data_count;
        frame_started_ms = now_ms;
        frame_confirmation_received = 0u;
        frame_confirmation_ok = 0u;
        frame_aborted = 0u;
        frame_pending = 1u;
#ifdef ECU_TARGET_EPIC4
        /* The selected DoCAN target has a fixed eight-byte Classical CAN
         * N-PDU. All callers provide a fully initialized eight-byte buffer. */
        (void)dlc;
        result = LSduR_CanTpTransmit(8u, data);
#else
        result = LSduR_CanTpTransmit(dlc, data);
#endif
        if (result != ECU_OK) {
            frame_pending = 0u;
            frame_kind = CANTP_FRAME_NONE;
        }
    }
    return result;
}

static void AbortRx(void) {
    if (rx.active != 0u) {
        rx.active = 0u;
        PduR_CanTpRxAbort();
    }
}

static EcuStatus FinishRx(uint64_t now_ms) {
    rx.active = 0u;
    return PduR_CanTpRxIndication(now_ms);
}

static EcuStatus FinishTx(EcuStatus status, uint64_t now_ms) {
    tx.active = 0u;
    tx.waiting_fc = 0u;
    PduR_CanTpTxConfirmation(status, now_ms);
    return status;
}

static EcuStatus CompletePendingFrame(uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    if ((frame_pending != 0u) && (frame_confirmation_received != 0u)) {
        CanTpFrameKind kind = frame_kind;
        uint8_t confirmed = frame_confirmation_ok;
        uint8_t aborted = frame_aborted;
        size_t count = frame_data_count;
        frame_pending = 0u;
        frame_kind = CANTP_FRAME_NONE;
        frame_confirmation_received = 0u;
        frame_aborted = 0u;
        if (aborted != 0u) {
            /* A late confirmation only releases the reserved CAN N-PDU. */
        } else if (confirmed == 0u) {
            if ((kind == CANTP_FRAME_FLOW_CONTROL) && (rx.active != 0u)) {
                AbortRx();
            } else if ((tx.active != 0u) &&
                       ((kind == CANTP_FRAME_SINGLE) || (kind == CANTP_FRAME_FIRST) ||
                        (kind == CANTP_FRAME_CONSECUTIVE))) {
                (void)FinishTx(ECU_ERR_IO, now_ms);
            } else {
                /* Overflow FC has no active upper-layer reception. */
            }
            result = ECU_ERR_IO;
        } else if (kind == CANTP_FRAME_SINGLE) {
            result = FinishTx(ECU_OK, now_ms);
        } else if (kind == CANTP_FRAME_FIRST) {
            tx.sent = count;
            tx.waiting_fc = 1u;
            tx.wait_started_ms = now_ms;
        } else if (kind == CANTP_FRAME_CONSECUTIVE) {
            tx.sent += count;
            tx.next_sn = (uint8_t)((tx.next_sn + 1u) & 0x0fu);
            tx.last_cf_ms = now_ms;
            tx.sent_cf = 1u;
            if (tx.sent == tx.length) {
                result = FinishTx(ECU_OK, now_ms);
            } else if (tx.block_size != 0u) {
                --tx.block_remaining;
                if (tx.block_remaining == 0u) {
                    tx.waiting_fc = 1u;
                    tx.wait_started_ms = now_ms;
                }
            } else {
                /* No flow-control block limit applies. */
            }
        } else if (kind == CANTP_FRAME_FLOW_CONTROL) {
            rx.last_cf_ms = now_ms;
        } else {
            /* Overflow FC has no active upper-layer reception. */
        }
    }
    return result;
}

void CanTp_TxConfirmation(PduIdType tx_pdu_id, Std_ReturnType result) {
    if ((cantp_config != NULL) && (tx_pdu_id == cantp_config->tx_pdu_id) && (frame_pending != 0u) &&
        (frame_confirmation_received == 0u)) {
        frame_confirmation_ok = (result == E_OK) ? 1u : 0u;
        frame_confirmation_received = 1u;
    }
}

static EcuStatus SendConsecutiveFrames(uint64_t now_ms) {
    EcuStatus outcome = ECU_OK;
    uint8_t stop = 0u;
    while ((tx.active != 0u) && (tx.waiting_fc == 0u) && (tx.sent < tx.length) &&
           (frame_pending == 0u) && (stop == 0u)) {
        uint8_t frame[8] = {0};
        size_t count = tx.length - tx.sent;

        if ((tx.sent_cf != 0u) && ((now_ms - tx.last_cf_ms) < tx.stmin_ms)) {
            stop = 1u;
        } else {
            EcuStatus result;
            if (count > 7u) {
                count = 7u;
            }
            frame[0] = (uint8_t)(0x20u | tx.next_sn);
            result = PduR_CanTpCopyTxData(tx.sent, &frame[1], count);
            if (result != ECU_OK) {
                outcome = FinishTx(result, now_ms);
                stop = 1u;
            } else {
                result =
                    SendFrame((uint8_t)(count + 1u), frame, now_ms, CANTP_FRAME_CONSECUTIVE, count);
                if (result != ECU_OK) {
                    outcome = FinishTx(result, now_ms);
                    stop = 1u;
                } else {
                    outcome = CompletePendingFrame(now_ms);
                    if ((frame_pending != 0u) || (outcome != ECU_OK)) {
                        stop = 1u;
                    }
                }
            }
        }
    }
    return outcome;
}

void CanTp_Init(const EcuDiagnosticConfig *config) {
    AbortRx();
    cantp_config = config;
    rx.active = 0u;
    tx.active = 0u;
    tx.waiting_fc = 0u;
    frame_kind = CANTP_FRAME_NONE;
    frame_pending = 0u;
    frame_confirmation_received = 0u;
    frame_confirmation_ok = 0u;
    frame_aborted = 0u;
}

EcuStatus CanTp_AdvanceTime(uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    if (cantp_config != NULL) {
        if (((frame_pending != 0u) && (now_ms < frame_started_ms)) ||
            ((rx.active != 0u) &&
             ((frame_pending == 0u) || (frame_kind != CANTP_FRAME_FLOW_CONTROL)) &&
             (now_ms < rx.last_cf_ms)) ||
            ((tx.active != 0u) && (tx.waiting_fc != 0u) && (now_ms < tx.wait_started_ms)) ||
            ((tx.active != 0u) && (tx.sent_cf != 0u) && (now_ms < tx.last_cf_ms))) {
            result = ECU_ERR_TIME;
        } else {
            if ((frame_pending != 0u) && (frame_confirmation_received != 0u)) {
                result = CompletePendingFrame(now_ms);
            } else if ((frame_pending != 0u) && (frame_aborted == 0u) &&
                       ((now_ms - frame_started_ms) >= cantp_config->n_as_ms)) {
                frame_aborted = 1u;
                if (frame_kind == CANTP_FRAME_FLOW_CONTROL) {
                    AbortRx();
                } else if (frame_kind == CANTP_FRAME_OVERFLOW) {
                    /* Overflow FC has no active upper-layer reception. */
                } else if (tx.active != 0u) {
                    (void)FinishTx(ECU_ERR_TP_TIMEOUT, now_ms);
                } else {
                    /* The transmit owner may already be inactive. */
                }
                result = ECU_ERR_TP_TIMEOUT;
            } else {
                /* No pending frame completed or expired at this tick. */
            }
            if (result == ECU_OK) {
                uint8_t expired = 0u;
                if ((rx.active != 0u) &&
                    ((frame_pending == 0u) || (frame_kind != CANTP_FRAME_FLOW_CONTROL)) &&
                    ((now_ms - rx.last_cf_ms) >= cantp_config->n_cr_ms)) {
                    AbortRx();
                    expired = 1u;
                }
                if ((tx.active != 0u) && (tx.waiting_fc != 0u) &&
                    ((now_ms - tx.wait_started_ms) >= cantp_config->n_bs_ms)) {
                    (void)FinishTx(ECU_ERR_TP_TIMEOUT, now_ms);
                    expired = 1u;
                }
                result = (expired != 0u) ? ECU_ERR_TP_TIMEOUT : SendConsecutiveFrames(now_ms);
            }
        }
    }
    return result;
}

static EcuStatus ReceiveSingle(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    size_t length = (size_t)(data[0] & 0x0fu);
    EcuStatus result = ECU_ERR_TP_LENGTH;
    AbortRx();
    if ((length != 0u) && (length <= 7u)) {
        if ((size_t)dlc < (length + 1u)) {
            result = ECU_ERR_FRAME_DLC;
        } else {
            result = PduR_CanTpStartOfReception(length);
            if (result == ECU_OK) {
                rx.active = 1u;
                result = PduR_CanTpCopyRxData(&data[1], length);
                if (result != ECU_OK) {
                    AbortRx();
                } else {
                    result = FinishRx(now_ms);
                }
            }
        }
    }
    return result;
}

static EcuStatus ReceiveFirst(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    size_t length = ((size_t)(data[0] & 0x0fu) << 8u) | data[1];
    uint8_t flow_control[8] = {0x30u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    EcuStatus result = ECU_ERR_TP_LENGTH;
    AbortRx();
    if (dlc != 8u) {
        result = ECU_ERR_FRAME_DLC;
    } else if (length > ECU_DIAG_MAX_PAYLOAD) {
        flow_control[0] = 0x32u; /* FC(OVFLW): no N-SDU was delivered. */
        result = SendFrame(3u, flow_control, now_ms, CANTP_FRAME_OVERFLOW, 0u);
        if (result == ECU_OK) {
            result = CompletePendingFrame(now_ms);
        }
        if (result == ECU_OK) {
            result = ECU_ERR_TP_LENGTH;
        }
    } else if (length > 7u) {
        result = PduR_CanTpStartOfReception(length);
        if (result == ECU_OK) {
            rx.active = 1u;
            rx.length = length;
            rx.received = 0u;
            rx.next_sn = 1u;
            result = PduR_CanTpCopyRxData(&data[2], 6u);
            if (result != ECU_OK) {
                AbortRx();
            } else {
                rx.received = 6u;
                result = SendFrame(3u, flow_control, now_ms, CANTP_FRAME_FLOW_CONTROL, 0u);
                if (result != ECU_OK) {
                    AbortRx();
                } else {
                    result = CompletePendingFrame(now_ms);
                }
            }
        }
    } else {
        /* A first frame must carry more than one Classical CAN frame. */
    }
    return result;
}

static EcuStatus ReceiveConsecutive(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_ERR_TP_SEQUENCE;
    if (rx.active == 0u) {
        /* No segmented reception is active. */
    } else if ((frame_pending != 0u) && (frame_kind == CANTP_FRAME_FLOW_CONTROL)) {
        result = ECU_ERR_TP_FLOW;
    } else if ((data[0] & 0x0fu) != rx.next_sn) {
        AbortRx();
    } else {
        size_t remaining = rx.length - rx.received;
        size_t count = remaining;
        if (count > 7u) {
            count = 7u;
        }
        if (((remaining > 7u) && (dlc != 8u)) ||
            ((remaining <= 7u) && ((size_t)dlc < (count + 1u)))) {
            AbortRx();
            result = ECU_ERR_FRAME_DLC;
        } else {
            result = PduR_CanTpCopyRxData(&data[1], count);
            if (result != ECU_OK) {
                AbortRx();
            } else {
                rx.received += count;
                rx.next_sn = (uint8_t)((rx.next_sn + 1u) & 0x0fu);
                if (rx.received == rx.length) {
                    result = FinishRx(now_ms);
                } else {
                    rx.last_cf_ms = now_ms;
                }
            }
        }
    }
    return result;
}

static EcuStatus ReceiveFlowControl(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    uint8_t flow_status = data[0] & 0x0fu;
    EcuStatus result = ECU_ERR_TP_FLOW;
    if ((tx.active != 0u) && (tx.waiting_fc != 0u)) {
        if ((dlc < 3u) || (flow_status > 2u)) {
            result = FinishTx(ECU_ERR_TP_FLOW, now_ms);
        } else if (flow_status == 2u) {
            result = FinishTx(ECU_ERR_TP_FLOW, now_ms);
        } else if (flow_status == 1u) {
            tx.wait_started_ms = now_ms;
            result = ECU_OK;
        } else {
            uint8_t stmin = data[2];
            if ((stmin > 0x7fu) && ((stmin < 0xf1u) || (stmin > 0xf9u))) {
                result = FinishTx(ECU_ERR_TP_FLOW, now_ms);
            } else {
                tx.stmin_ms =
                    (stmin <= 0x7fu)
                        ? (uint32_t)stmin
                        : UINT32_C(1); /* 100-us values round up to the host's ms clock. */
                tx.block_size = data[1];
                tx.block_remaining = data[1];
                tx.waiting_fc = 0u;
                result = SendConsecutiveFrames(now_ms);
            }
        }
    }
    return result;
}

EcuStatus CanTp_RxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_ERR_CONFIG;
    if ((cantp_config != NULL) && (data != NULL)) {
        /* The owner drains the epoch's inputs before its transport deadline
         * phase; an input at the boundary must not expire itself first.
         * The legacy synchronous target still advances before receiving. */
#ifndef ECU_TARGET_EPIC4
        result = CanTp_AdvanceTime(now_ms);
        if (result == ECU_OK)
#endif
        {
            if ((dlc < 1u) || (dlc > 8u)) {
                AbortRx();
                if ((dlc > 8u) && ((data[0] >> 4u) == 3u) && (tx.active != 0u) &&
                    (tx.waiting_fc != 0u)) {
                    result = FinishTx(ECU_ERR_TP_FLOW, now_ms);
                } else {
                    result = ECU_ERR_FRAME_DLC;
                }
            } else {
                switch (data[0] >> 4u) {
                case 0u:
                    result = ReceiveSingle(dlc, data, now_ms);
                    break;
                case 1u:
                    if (dlc < 2u) {
                        AbortRx();
                        result = ECU_ERR_FRAME_DLC;
                    } else {
                        result = ReceiveFirst(dlc, data, now_ms);
                    }
                    break;
                case 2u:
                    result = ReceiveConsecutive(dlc, data, now_ms);
                    break;
                case 3u:
                    result = ReceiveFlowControl(dlc, data, now_ms);
                    break;
                default:
                    AbortRx();
                    result = ECU_ERR_TP_FLOW;
                    break;
                }
            }
        }
    }
    return result;
}

EcuStatus CanTp_Transmit(size_t length, uint64_t now_ms) {
    uint8_t frame[8] = {0};
    EcuStatus result = ECU_ERR_CONFIG;
    if (cantp_config != NULL) {
        if ((tx.active != 0u) || (frame_pending != 0u)) {
            result = ECU_ERR_TP_BUSY;
        } else if ((length == 0u) || (length > ECU_DIAG_MAX_PAYLOAD)) {
            result = ECU_ERR_TP_LENGTH;
        } else {
            size_t count;
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
            result = PduR_CanTpCopyTxData(0u, &frame[(length <= 7u) ? 1u : 2u], count);
            if (result == ECU_OK) {
                result = SendFrame((uint8_t)((length <= 7u) ? (length + 1u) : 8u), frame, now_ms,
                                   (length <= 7u) ? CANTP_FRAME_SINGLE : CANTP_FRAME_FIRST, count);
            }
            if (result != ECU_OK) {
                result = FinishTx(result, now_ms);
            } else {
                result = CompletePendingFrame(now_ms);
            }
        }
    }
    return result;
}
