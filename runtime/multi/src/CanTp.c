#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
#define CANTP_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"
typedef enum {
    CANTP_IDLE,
    CANTP_TX_COPY,
    CANTP_TX_SEND,
    CANTP_TX_CONFIRM,
    CANTP_TX_FLOW,
    CANTP_RX_FLOW_SEND,
    CANTP_RX_FLOW_CONFIRM,
    CANTP_RX_DATA,
    CANTP_RX_OVERFLOW_SEND,
    CANTP_RX_OVERFLOW_CONFIRM
} CanTp_StateType;
static const CanTp_ConfigType *configuration CANTP_VAR_CLEARED;
static CanTp_StateType state CANTP_VAR_CLEARED;
static uint8 frame[8] CANTP_VAR_CLEARED;
static uint8 sequence CANTP_VAR_CLEARED;
static uint8 block_size CANTP_VAR_CLEARED;
static uint8 block_sent CANTP_VAR_CLEARED;
static uint32 separation CANTP_VAR_CLEARED;
static uint32 delay CANTP_VAR_CLEARED;
static uint32 timer CANTP_VAR_CLEARED;
static PduLengthType total CANTP_VAR_CLEARED;
static PduLengthType position CANTP_VAR_CLEARED;
static PduLengthType frame_payload CANTP_VAR_CLEARED;
static boolean lower_pending CANTP_VAR_CLEARED;
#define CANTP_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"
#define CANTP_START_SEC_CODE
#include "CanTp_MemMap.h"
static CANTP_CODE void CanTp_FinishTx(Std_ReturnType result) {
    state = CANTP_IDLE;
    PduR_CanTpTxConfirmation(configuration->transmit, result);
}
static CANTP_CODE void CanTp_FinishRx(Std_ReturnType result) {
    state = CANTP_IDLE;
    PduR_CanTpRxIndication(configuration->receive, result);
}
CANTP_CODE void CanTp_Init(const CanTp_ConfigType *CfgPtr) {
    SchM_Enter_CanTp_CANTP_STATE();
    if ((CfgPtr != NULL_PTR) && (CfgPtr->maximum_length > 7u) && (CfgPtr->maximum_length <= 256u) &&
        (CfgPtr->n_as > 0u) && (CfgPtr->n_ar > 0u) && (CfgPtr->n_bs > 0u) && (CfgPtr->n_cr > 0u) &&
        (CfgPtr->n_cs > 0u) && (CfgPtr->main_period_ms > 0u)) {
        configuration = CfgPtr;
        state = CANTP_IDLE;
        lower_pending = FALSE;
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
CANTP_CODE void CanTp_Shutdown(void) {
    SchM_Enter_CanTp_CANTP_STATE();
    configuration = NULL_PTR;
    state = CANTP_IDLE;
    lower_pending = FALSE;
    SchM_Exit_CanTp_CANTP_STATE();
}
static CANTP_CODE void CanTp_Pad(void) {
    uint8 i;
    for (i = 0u; i < 8u; ++i) {
        frame[i] = configuration->padding;
    }
}
static CANTP_CODE void CanTp_Copy(void) {
    uint8 header;
    PduLengthType capacity;
    PduLengthType available = 0u;
    BufReq_ReturnType result;
    PduInfoType info;
    CanTp_Pad();
    if (total <= 7u) {
        frame[0] = (uint8)total;
        header = 1u;
        capacity = 7u;
    } else if (position == 0u) {
        frame[0] = 0x10u | (uint8)(total / 256u);
        frame[1] = (uint8)(total % 256u);
        header = 2u;
        capacity = 6u;
    } else {
        frame[0] = 0x20u | sequence;
        header = 1u;
        capacity = 7u;
    }
    frame_payload = total - position;
    if (frame_payload > capacity) {
        frame_payload = capacity;
    }
    info.SduDataPtr = &frame[header];
    info.MetaDataPtr = NULL_PTR;
    info.SduLength = frame_payload;
    result = PduR_CanTpCopyTxData(configuration->transmit, &info, NULL_PTR, &available);
    if (result == BUFREQ_OK) {
        state = CANTP_TX_SEND;
        timer = configuration->n_as;
    } else if (result != BUFREQ_E_BUSY) {
        CanTp_FinishTx(E_NOT_OK);
    } else {
        /* N_Cs limits upper-buffer retries; no whole-message TP copy exists. */
    }
}
static CANTP_CODE void CanTp_Send(void) {
    const PduInfoType info = {frame, NULL_PTR, 8u};
    const CanTp_StateType previous = state;
    if (lower_pending == TRUE) {
        return;
    }
    lower_pending = TRUE;
    if (previous == CANTP_TX_SEND) {
        state = CANTP_TX_CONFIRM;
    } else if (previous == CANTP_RX_FLOW_SEND) {
        state = CANTP_RX_FLOW_CONFIRM;
    } else {
        state = CANTP_RX_OVERFLOW_CONFIRM;
    }
    /* A synchronous lower callback sees the published confirmation state. */
    if (LSduR_CanTpTransmit(configuration->lower_transmit, &info) != E_OK) {
        lower_pending = FALSE;
        if (previous == CANTP_TX_SEND) {
            CanTp_FinishTx(E_NOT_OK);
        } else if (previous == CANTP_RX_FLOW_SEND) {
            CanTp_FinishRx(E_NOT_OK);
        } else {
            state = CANTP_IDLE;
        }
    }
}
CANTP_CODE Std_ReturnType CanTp_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (TxPduId == configuration->transmit) &&
        (PduInfoPtr != NULL_PTR) && (PduInfoPtr->MetaDataPtr == NULL_PTR) &&
        (PduInfoPtr->SduLength > 0u) && (PduInfoPtr->SduLength <= configuration->maximum_length) &&
        (state == CANTP_IDLE) && (lower_pending == FALSE)) {
        total = PduInfoPtr->SduLength;
        position = 0u;
        sequence = 1u;
        block_size = 0u;
        block_sent = 0u;
        separation = 0u;
        delay = 0u;
        timer = configuration->n_cs;
        state = CANTP_TX_COPY;
        result = E_OK;
        CanTp_Copy();
        if (state == CANTP_TX_SEND) {
            CanTp_Send();
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    return result;
}
static CANTP_CODE void CanTp_Flow(boolean overflow) {
    CanTp_Pad();
    frame[0] = (overflow == TRUE) ? 0x32u : 0x30u;
    frame[1] = 0u; /* Entire buffer reserved: unlimited block size, no WAIT. */
    frame[2] = 0u;
    state = (overflow == TRUE) ? CANTP_RX_OVERFLOW_SEND : CANTP_RX_FLOW_SEND;
    timer = configuration->n_ar;
    CanTp_Send();
}
static CANTP_CODE void CanTp_StartRx(const PduInfoType *info, boolean first) {
    const uint8 header = (first == TRUE) ? 2u : 1u;
    const PduLengthType length =
        (first == TRUE)
            ? (((PduLengthType)(info->SduDataPtr[0] & 0x0fu) * 256u) + info->SduDataPtr[1])
            : (info->SduDataPtr[0] & 0x0fu);
    PduLengthType available = 0u;
    BufReq_ReturnType result;
    PduInfoType payload;
    if (((first == TRUE) && (length <= 7u)) ||
        ((first == FALSE) && ((length == 0u) || (length > 7u)))) {
        return;
    }
    payload.SduDataPtr = &info->SduDataPtr[header];
    payload.MetaDataPtr = NULL_PTR;
    payload.SduLength = (first == TRUE) ? 6u : length;
    result = PduR_CanTpStartOfReception(configuration->receive, &payload, length, &available);
    if (result == BUFREQ_OK) {
        total = length;
        position = 0u;
        sequence = 1u;
        state = CANTP_RX_DATA;
        if ((available < length) || (length > configuration->maximum_length) ||
            (PduR_CanTpCopyRxData(configuration->receive, &payload, &available) != BUFREQ_OK)) {
            (void)Det_ReportRuntimeError(35u, 0u, 0x42u, 0xb0u);
            CanTp_FinishRx(E_NOT_OK);
        } else {
            position = payload.SduLength;
            if (first == TRUE) {
                CanTp_Flow(FALSE);
            } else {
                CanTp_FinishRx(E_OK);
            }
        }
    } else if ((result == BUFREQ_E_OVFL) && (first == TRUE)) {
        CanTp_Flow(TRUE);
    } else {
        /* Admission rejection is not an accepted reception: no final callback. */
    }
}
static CANTP_CODE void CanTp_ReceiveFlow(const uint8 *data) {
    const uint8 flow = data[0] & 0x0fu;
    if (flow == 0u) {
        const uint8 stmin = data[2];
        uint16 milliseconds = stmin;
        if ((stmin >= 0xf1u) && (stmin <= 0xf9u)) {
            milliseconds = 1u; /* One configured tick conservatively exceeds 100-900us. */
        } else if (stmin > 0x7fu) {
            milliseconds = 127u;
        } else {
            /* Normal millisecond STmin. */
        }
        block_size = data[1];
        block_sent = 0u;
        separation = ((uint32)milliseconds + configuration->main_period_ms - 1u) /
                     configuration->main_period_ms;
        delay = separation;
        state = CANTP_TX_COPY;
        timer = configuration->n_cs;
    } else if (flow == 1u) {
        timer = configuration->n_bs;
    } else {
        (void)Det_ReportRuntimeError(35u, 0u, 0x42u, 0xb0u);
        CanTp_FinishTx(E_NOT_OK);
    }
}
CANTP_CODE void CanTp_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr) {
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (RxPduId == configuration->lower_receive) &&
        (PduInfoPtr != NULL_PTR) && (PduInfoPtr->SduDataPtr != NULL_PTR) &&
        (PduInfoPtr->MetaDataPtr == NULL_PTR) && (PduInfoPtr->SduLength > 0u) &&
        (PduInfoPtr->SduLength <= 8u)) {
        const uint8 kind = PduInfoPtr->SduDataPtr[0] / 16u;
        if (PduInfoPtr->SduLength < 8u) {
            if (((state == CANTP_TX_FLOW) && (kind == 3u)) ||
                ((state == CANTP_RX_DATA) && (kind == 2u)) || (kind == 0u)) {
                (void)Det_ReportRuntimeError(35u, 0u, 0x42u, 0x70u);
                if ((state == CANTP_TX_FLOW) && (kind == 3u)) {
                    CanTp_FinishTx(E_NOT_OK);
                } else if ((state == CANTP_RX_DATA) && (kind == 2u)) {
                    CanTp_FinishRx(E_NOT_OK);
                } else {
                    /* Rejected SF has never reserved an upper buffer. */
                }
            }
        } else if ((state == CANTP_TX_FLOW) && (kind == 3u)) {
            CanTp_ReceiveFlow(PduInfoPtr->SduDataPtr);
        } else if (((state == CANTP_IDLE) || (state == CANTP_RX_DATA) ||
                    (state == CANTP_RX_FLOW_SEND) || (state == CANTP_RX_FLOW_CONFIRM)) &&
                   ((kind == 0u) || (kind == 1u))) {
            if (state != CANTP_IDLE) {
                CanTp_FinishRx(E_NOT_OK);
            }
            CanTp_StartRx(PduInfoPtr, kind == 1u);
        } else if ((state == CANTP_RX_DATA) && (kind == 2u)) {
            if ((PduInfoPtr->SduDataPtr[0] & 0x0fu) != sequence) {
                (void)Det_ReportRuntimeError(35u, 0u, 0x42u, 0xb0u);
                CanTp_FinishRx(E_NOT_OK);
            } else {
                PduLengthType available = 0u;
                PduInfoType payload = {&PduInfoPtr->SduDataPtr[1], NULL_PTR, total - position};
                if (payload.SduLength > 7u) {
                    payload.SduLength = 7u;
                }
                if (PduR_CanTpCopyRxData(configuration->receive, &payload, &available) !=
                    BUFREQ_OK) {
                    (void)Det_ReportRuntimeError(35u, 0u, 0x42u, 0xb0u);
                    CanTp_FinishRx(E_NOT_OK);
                } else {
                    position += payload.SduLength;
                    sequence = (sequence + 1u) % 16u;
                    timer = configuration->n_cr;
                    if (position == total) {
                        CanTp_FinishRx(E_OK);
                    }
                }
            }
        } else {
            /* Other frames cannot change this physical half-duplex connection. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
CANTP_CODE void CanTp_TxConfirmation(PduIdType TxPduId, Std_ReturnType result) {
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (TxPduId == configuration->lower_transmit)) {
        lower_pending = FALSE;
        if (state == CANTP_TX_CONFIRM) {
            if (result != E_OK) {
                CanTp_FinishTx(E_NOT_OK);
            } else {
                const boolean first = position == 0u;
                position += frame_payload;
                if (position == total) {
                    CanTp_FinishTx(E_OK);
                } else if ((first == TRUE) ||
                           ((block_size > 0u) && ((block_sent + 1u) >= block_size))) {
                    state = CANTP_TX_FLOW;
                    timer = configuration->n_bs;
                    if (first == FALSE) {
                        sequence = (sequence + 1u) % 16u;
                    }
                } else {
                    ++block_sent;
                    sequence = (sequence + 1u) % 16u;
                    state = CANTP_TX_COPY;
                    timer = configuration->n_cs;
                    delay = separation;
                }
            }
        } else if (state == CANTP_RX_FLOW_CONFIRM) {
            if (result == E_OK) {
                state = CANTP_RX_DATA;
                timer = configuration->n_cr;
            } else {
                CanTp_FinishRx(E_NOT_OK);
            }
        } else if (state == CANTP_RX_OVERFLOW_CONFIRM) {
            state = CANTP_IDLE;
        } else {
            /* Late confirmation after shutdown/timeout cannot replay callbacks. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
CANTP_CODE void CanTp_MainFunction(void) {
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (state != CANTP_IDLE)) {
        if ((state == CANTP_TX_COPY) && (delay > 0u)) {
            --delay;
            if (delay == 0u) {
                CanTp_Copy();
                if (state == CANTP_TX_SEND) {
                    CanTp_Send();
                }
            }
        } else {
            if (timer > 0u) {
                --timer;
            }
            if (timer == 0u) {
                if ((state >= CANTP_TX_COPY) && (state <= CANTP_TX_FLOW)) {
                    CanTp_FinishTx(E_NOT_OK);
                } else if ((state == CANTP_RX_OVERFLOW_SEND) ||
                           (state == CANTP_RX_OVERFLOW_CONFIRM)) {
                    state = CANTP_IDLE;
                } else {
                    CanTp_FinishRx(E_NOT_OK);
                }
            } else {
                if (state == CANTP_TX_COPY) {
                    CanTp_Copy();
                }
                if ((state == CANTP_TX_SEND) || (state == CANTP_RX_FLOW_SEND) ||
                    (state == CANTP_RX_OVERFLOW_SEND)) {
                    CanTp_Send();
                }
            }
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
#define CANTP_STOP_SEC_CODE
#include "CanTp_MemMap.h"
