/** @file CanTp for one channel of physical, padded Classical CAN N-SDUs.
 * Reception and transmission are independent (full duplex, SWS_CanTp_00057) and share the
 * Tx N-PDU, which its TxConfirmation serializes (SWS_CanTp_00248). CanTp enters only its
 * own CANTP_STATE area and calls PduR, LSduR and Det outside it (ADR 0018): events are
 * recorded in the area, and one context at a time makes the calls they require.
 */
#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
#define CANTP_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"

#define CANTP_MODULE_ID       35u
#define CANTP_SID_MAIN        0x06u
#define CANTP_SID_RX_INDICATE 0x42u
#define CANTP_SID_CANCEL_TX   0x4au
#define CANTP_SID_CANCEL_RX   0x4cu
/* Runtime error values (SWS_CanTp_00352); zero means none. */
#define CANTP_E_PADDING       0x70u
#define CANTP_E_OPER_NOT_SUPP 0xa0u
#define CANTP_E_COM           0xb0u
#define CANTP_E_RX_COM        0xc0u
#define CANTP_E_TX_COM        0xd0u
/* RX_DL = TX_DL = 8 (SWS_CanTp_00350, 00351, 00354). */
#define CANTP_FRAME_BYTES  8u
#define CANTP_SF_MAX       7u
#define CANTP_FF_DATA      6u
#define CANTP_CF_DATA      7u
#define CANTP_PCI_SF       0u
#define CANTP_PCI_FF       1u
#define CANTP_PCI_CF       2u
#define CANTP_PCI_FC       3u
#define CANTP_FS_CTS       0u
#define CANTP_FS_WAIT      1u
#define CANTP_FS_OVFLW     2u
#define CANTP_STMIN_MAX_MS 0x7fu
/* SF, FF and CF waiting while another context makes calls; full means dropped. */
#define CANTP_RX_QUEUE 4u

typedef enum {
    CANTP_TX_IDLE,
    CANTP_TX_COPY,
    CANTP_TX_SEND,
    CANTP_TX_CONFIRM,
    CANTP_TX_FLOW,
    CANTP_TX_DONE /* Finished; the upper confirmation is owed. */
} CanTp_TxStateType;
typedef enum {
    CANTP_RX_IDLE,
    CANTP_RX_START, /* Admission calls of an SF or FF in progress. */
    CANTP_RX_BUFFER,
    CANTP_RX_FLOW_SEND,
    CANTP_RX_FLOW_CONFIRM,
    CANTP_RX_DATA,
    CANTP_RX_DONE /* Finished; the upper indication is owed. */
} CanTp_RxStateType;
typedef enum { CANTP_LOWER_FREE, CANTP_LOWER_TX, CANTP_LOWER_RX } CanTp_LowerType;
typedef enum {
    CANTP_WORK_NONE,
    CANTP_WORK_TX_DONE,
    CANTP_WORK_RX_DONE,
    CANTP_WORK_FRAME,
    CANTP_WORK_COPY_TX,
    CANTP_WORK_LOWER,
    CANTP_WORK_POLL
} CanTp_WorkKindType;
/* One call made outside the area, prepared inside it. A generation that changed meanwhile
 * (finish, Init, Shutdown) makes its result stale. */
typedef struct {
    CanTp_WorkKindType kind;
    CanTp_LowerType direction;
    uint32 generation;
    PduIdType id;
    Std_ReturnType result;
    PduLengthType offset;
    PduLengthType length;
    uint8 frame[CANTP_FRAME_BYTES];
} CanTp_WorkType;
typedef struct {
    CanTp_TxStateType state;
    uint32 generation;
    uint32 timer;
    uint32 delay;
    uint32 separation;
    boolean due;
    PduLengthType total;
    PduLengthType position;
    PduLengthType payload;
    uint8 sequence;
    uint8 block_size;
    uint8 block_sent;
    Std_ReturnType result;
    uint8 frame[CANTP_FRAME_BYTES];
} CanTp_TxType;
typedef struct {
    CanTp_RxStateType state;
    uint32 generation;
    uint32 timer;
    boolean due;
    uint8 flow;
    uint8 waits;
    PduLengthType total;
    PduLengthType position;
    uint8 sequence;
    Std_ReturnType result;
    uint8 frame[CANTP_FRAME_BYTES];
} CanTp_RxType;

static const CanTp_ConfigType *configuration CANTP_VAR_CLEARED;
static CanTp_TxType tx CANTP_VAR_CLEARED;
static CanTp_RxType rx CANTP_VAR_CLEARED;
/* The Tx N-PDU in flight keeps its owner until its actual confirmation (SWS_CanTp_00075);
 * Init and Shutdown do not reset it. */
static CanTp_LowerType lower_owner CANTP_VAR_CLEARED;
static uint32 lower_generation CANTP_VAR_CLEARED;
static PduIdType pending_lower_id CANTP_VAR_CLEARED;
static uint8 queued[CANTP_RX_QUEUE][CANTP_FRAME_BYTES] CANTP_VAR_CLEARED;
static uint8 queued_length[CANTP_RX_QUEUE] CANTP_VAR_CLEARED;
static uint8 queue_head CANTP_VAR_CLEARED;
static uint8 queue_count CANTP_VAR_CLEARED;
/* A context is making CanTp's calls; never reset, so it outlives a reinitialization. */
static boolean running CANTP_VAR_CLEARED;
#define CANTP_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"

#define CANTP_START_SEC_CODE
#include "CanTp_MemMap.h"
static CANTP_CODE void CanTp_Report(uint8 ApiId, uint8 ErrorId)
{
    if (ErrorId != 0u) {
        (void)Det_ReportRuntimeError(CANTP_MODULE_ID, 0u, ApiId, ErrorId);
    }
}
/* The helpers below up to CanTp_Run run inside CANTP_STATE. */
static CANTP_CODE void CanTp_TxFinish(Std_ReturnType result)
{
    tx.state = CANTP_TX_DONE;
    tx.result = result;
    ++tx.generation;
}
static CANTP_CODE void CanTp_RxFinish(Std_ReturnType result)
{
    rx.state = CANTP_RX_DONE;
    rx.result = result;
    ++rx.generation;
}
/* Ends a reception the upper layer never accepted: no indication (SWS_CanTp_00081). */
static CANTP_CODE void CanTp_RxDrop(void)
{
    rx.state = CANTP_RX_IDLE;
    ++rx.generation;
}
static CANTP_CODE void CanTp_Reset(void)
{
    tx.state = CANTP_TX_IDLE;
    ++tx.generation;
    CanTp_RxDrop();
    queue_count = 0u;
}
static CANTP_CODE boolean CanTp_Receiving(void)
{
    return ((rx.state == CANTP_RX_BUFFER) || (rx.state == CANTP_RX_FLOW_SEND) ||
            (rx.state == CANTP_RX_FLOW_CONFIRM) || (rx.state == CANTP_RX_DATA))
               ? TRUE
               : FALSE;
}
/* A well-formed SF or FF that starts a reception (SWS_CanTp_00344, ISO 15765-2 DL rules). */
static CANTP_CODE boolean CanTp_Starts(const uint8 *data, uint8 length)
{
    const uint8 kind = data[0] / 16u;
    const PduLengthType sf_dl = (PduLengthType)data[0] & 0x0fu;
    const PduLengthType ff_dl = (PduLengthType)((sf_dl * 256u) + data[1]);
    return ((length == CANTP_FRAME_BYTES) &&
            (((kind == CANTP_PCI_SF) && (sf_dl > 0u) && (sf_dl <= CANTP_SF_MAX)) ||
             ((kind == CANTP_PCI_FF) && (ff_dl > CANTP_SF_MAX))))
               ? TRUE
               : FALSE;
}
static CANTP_CODE void CanTp_Pad(uint8 *frame)
{
    uint8 i;
    for (i = 0u; i < CANTP_FRAME_BYTES; ++i) {
        frame[i] = configuration->padding;
    }
}
/* Ticks before the next CF: one more than STmin needs, because the first tick may follow
 * the confirmation at once and STmin is a minimum. Reserved values mean 127 ms and
 * 100-900 us one tick (ISO 15765-2). */
static CANTP_CODE uint32 CanTp_Separation(uint8 stmin)
{
    uint32 milliseconds = stmin;
    uint32 ticks;
    if ((stmin >= 0xf1u) && (stmin <= 0xf9u)) {
        milliseconds = 1u;
    } else if (stmin > CANTP_STMIN_MAX_MS) {
        milliseconds = CANTP_STMIN_MAX_MS;
    } else {
        /* Whole milliseconds. */
    }
    ticks = (milliseconds + configuration->main_period_ms - 1u) / configuration->main_period_ms;
    return (ticks == 0u) ? 0u : (ticks + 1u);
}
/* An FC for the transmission (SWS_CanTp_00315, 00309, 00317, 00349); returns the error. */
static CANTP_CODE uint8 CanTp_ReceiveFlow(const uint8 *data, uint8 length)
{
    uint8 error = 0u;
    const uint8 status = data[0] & 0x0fu;
    if (tx.state != CANTP_TX_FLOW) {
        /* An FC that is not awaited is ignored (SWS_CanTp_00057). */
    } else if (length < CANTP_FRAME_BYTES) {
        CanTp_TxFinish(E_NOT_OK);
        error = CANTP_E_PADDING;
    } else if (status == CANTP_FS_CTS) {
        tx.block_size = data[1];
        tx.block_sent = 0u;
        tx.separation = CanTp_Separation(data[2]);
        tx.delay = 0u; /* STmin separates CFs; the first follows the FC at once. */
        tx.due = TRUE;
        tx.state = CANTP_TX_COPY;
        tx.timer = configuration->n_cs;
    } else if (status == CANTP_FS_WAIT) {
        tx.timer = configuration->n_bs;
    } else {
        CanTp_TxFinish(E_NOT_OK);
        error = CANTP_E_COM;
    }
    return error;
}
static CANTP_CODE void CanTp_TxConfirmed(Std_ReturnType result)
{
    if (result != E_OK) {
        CanTp_TxFinish(E_NOT_OK); /* SWS_CanTp_00355 */
    } else {
        const boolean first = (tx.position == 0u) ? TRUE : FALSE;
        tx.position += tx.payload;
        if (tx.position == tx.total) {
            CanTp_TxFinish(E_OK);
        } else if ((first == TRUE) ||
                   ((tx.block_size > 0u) && ((tx.block_sent + 1u) >= tx.block_size))) {
            tx.state = CANTP_TX_FLOW;
            tx.timer = configuration->n_bs;
            if (first == FALSE) {
                tx.sequence = (uint8)((tx.sequence + 1u) % 16u);
            }
        } else {
            ++tx.block_sent;
            tx.sequence = (uint8)((tx.sequence + 1u) % 16u);
            tx.state = CANTP_TX_COPY;
            tx.timer = configuration->n_cs;
            tx.delay = tx.separation;
            tx.due = (tx.delay == 0u) ? TRUE : FALSE;
        }
    }
}
static CANTP_CODE void CanTp_FlowConfirmed(Std_ReturnType result)
{
    if (rx.flow == CANTP_FS_OVFLW) {
        CanTp_RxDrop(); /* The reception was refused at admission (SWS_CanTp_00318). */
    } else if (result != E_OK) {
        CanTp_RxFinish(E_NOT_OK);
    } else if (rx.flow == CANTP_FS_WAIT) {
        rx.state = CANTP_RX_BUFFER;
        rx.timer = configuration->n_br;
    } else {
        rx.state = CANTP_RX_DATA;
        rx.timer = configuration->n_cr; /* SWS_CanTp_00312 */
    }
}
static CANTP_CODE uint8 CanTp_TxTick(void)
{
    uint8 error = 0u;
    if ((tx.state == CANTP_TX_COPY) && (tx.delay > 0u)) {
        --tx.delay;
        tx.due = (tx.delay == 0u) ? TRUE : FALSE;
    } else if ((tx.state != CANTP_TX_IDLE) && (tx.state != CANTP_TX_DONE)) {
        if (tx.timer > 0u) {
            --tx.timer;
        }
        if (tx.timer == 0u) {
            /* N_As, N_Bs or N_Cs (SWS_CanTp_00229, 00280, 00310, 00316). */
            CanTp_TxFinish(E_NOT_OK);
            error = CANTP_E_TX_COM;
        } else if (tx.state == CANTP_TX_COPY) {
            tx.due = TRUE; /* Retry a busy upper copy within N_Cs (SWS_CanTp_00184). */
        } else {
            /* Waiting for the N-PDU, its confirmation or an FC. */
        }
    } else {
        /* Nothing is timed. */
    }
    return error;
}
static CANTP_CODE uint8 CanTp_RxTick(void)
{
    uint8 error = 0u;
    if (CanTp_Receiving() == TRUE) {
        if (rx.timer > 0u) {
            --rx.timer;
        }
        if ((rx.timer == 0u) && (rx.state == CANTP_RX_BUFFER) &&
            (rx.waits < configuration->wait_frame_max)) {
            ++rx.waits; /* N_Br expired: suspend with FC(WAIT) (SWS_CanTp_00341). */
            rx.flow = CANTP_FS_WAIT;
            rx.state = CANTP_RX_FLOW_SEND;
            rx.timer = configuration->n_ar;
        } else if (rx.timer == 0u) {
            /* N_Ar, N_Br after WFTmax FC(WAIT), or N_Cr (SWS_CanTp_00223, 00311, 00313). */
            if ((rx.state != CANTP_RX_DATA) && (rx.state != CANTP_RX_BUFFER) &&
                (rx.flow == CANTP_FS_OVFLW)) {
                CanTp_RxDrop();
            } else {
                CanTp_RxFinish(E_NOT_OK);
            }
            error = CANTP_E_RX_COM;
        } else if (rx.state == CANTP_RX_BUFFER) {
            rx.due = TRUE; /* Ask the upper layer for buffer again (SWS_CanTp_00222). */
        } else {
            /* Waiting for the N-PDU, its confirmation or a CF. */
        }
    }
    return error;
}
/* Claims the Tx N-PDU for the frame of `direction`; the caller sends it after the area. */
static CANTP_CODE void CanTp_Claim(CanTp_WorkType *work, CanTp_LowerType direction)
{
    uint8 i;
    const uint8 *frame = (direction == CANTP_LOWER_TX) ? tx.frame : rx.frame;
    lower_owner = direction;
    lower_generation = (direction == CANTP_LOWER_TX) ? tx.generation : rx.generation;
    pending_lower_id = configuration->lower_transmit;
    for (i = 0u; i < CANTP_FRAME_BYTES; ++i) {
        work->frame[i] = frame[i];
    }
    work->kind = CANTP_WORK_LOWER;
    work->direction = direction;
    work->generation = lower_generation;
    work->id = pending_lower_id;
}
/* Builds the next SF, FF or CF header and asks for its data (SWS_CanTp_00086, 00231/00232). */
static CANTP_CODE void CanTp_PrepareCopy(CanTp_WorkType *work)
{
    PduLengthType capacity = CANTP_CF_DATA;
    CanTp_Pad(tx.frame);
    work->offset = 1u;
    if (tx.total <= CANTP_SF_MAX) {
        tx.frame[0] = (uint8)tx.total;
    } else if (tx.position == 0u) {
        tx.frame[0] = (uint8)(0x10u | (uint8)(tx.total / 256u));
        tx.frame[1] = (uint8)(tx.total % 256u);
        work->offset = 2u;
        capacity = CANTP_FF_DATA;
    } else {
        tx.frame[0] = (uint8)(0x20u | tx.sequence);
    }
    work->length = tx.total - tx.position;
    if (work->length > capacity) {
        work->length = capacity;
    }
    tx.due = FALSE;
    work->kind = CANTP_WORK_COPY_TX;
    work->generation = tx.generation;
    work->id = configuration->upper_transmit;
}
static CANTP_CODE void CanTp_PrepareFlow(CanTp_WorkType *work)
{
    CanTp_Pad(rx.frame); /* SWS_CanTp_00347 */
    /* The whole N-SDU is requested at once: BS 0 and STmin 0 for every FC (SWS_CanTp_00067). */
    rx.frame[0] = (uint8)(0x30u | rx.flow);
    rx.frame[1] = 0u;
    rx.frame[2] = 0u;
    rx.state = CANTP_RX_FLOW_CONFIRM;
    CanTp_Claim(work, CANTP_LOWER_RX);
}
/* Takes the oldest queued frame. */
static CANTP_CODE void CanTp_TakeFrame(CanTp_WorkType *work)
{
    uint8 i;
    for (i = 0u; i < CANTP_FRAME_BYTES; ++i) {
        work->frame[i] = queued[queue_head][i];
    }
    work->length = queued_length[queue_head];
    work->kind = CANTP_WORK_FRAME;
    work->generation = rx.generation;
    queue_head = (uint8)((queue_head + 1u) % CANTP_RX_QUEUE);
    --queue_count;
}
/* Chooses the next call: owed notifications first, then frames in arrival order; frames
 * for the Tx N-PDU only when `send` allows them. */
static CANTP_CODE void CanTp_Next(CanTp_WorkType *work, boolean send)
{
    work->kind = CANTP_WORK_NONE;
    if ((configuration != NULL_PTR) && (queue_count > 0u) && (CanTp_Receiving() == TRUE) &&
        (CanTp_Starts(queued[queue_head], queued_length[queue_head]) == TRUE)) {
        /* A valid SF or FF ends the reception in progress first (SWS_CanTp_00057 table 1). */
        if (rx.flow == CANTP_FS_OVFLW) {
            CanTp_RxDrop();
        } else {
            CanTp_RxFinish(E_NOT_OK);
        }
    }
    if (configuration == NULL_PTR) {
        /* OFF: nothing is owed (SWS_CanTp_00200). */
    } else if (tx.state == CANTP_TX_DONE) {
        /* Free before the confirmation, which may already request the next N-SDU. */
        tx.state = CANTP_TX_IDLE;
        work->kind = CANTP_WORK_TX_DONE;
        work->id = configuration->upper_transmit;
        work->result = tx.result;
    } else if (rx.state == CANTP_RX_DONE) {
        rx.state = CANTP_RX_IDLE;
        work->kind = CANTP_WORK_RX_DONE;
        work->id = configuration->upper_receive;
        work->result = rx.result;
    } else if (queue_count > 0u) {
        CanTp_TakeFrame(work);
    } else if ((send == TRUE) && (tx.state == CANTP_TX_COPY) && (tx.due == TRUE)) {
        CanTp_PrepareCopy(work);
    } else if ((send == TRUE) && (rx.state == CANTP_RX_FLOW_SEND) &&
               (lower_owner == CANTP_LOWER_FREE)) {
        CanTp_PrepareFlow(work);
    } else if ((send == TRUE) && (tx.state == CANTP_TX_SEND) && (lower_owner == CANTP_LOWER_FREE)) {
        tx.state = CANTP_TX_CONFIRM; /* Published before a synchronous confirmation. */
        CanTp_Claim(work, CANTP_LOWER_TX);
    } else if ((rx.state == CANTP_RX_BUFFER) && (rx.due == TRUE)) {
        rx.due = FALSE;
        work->kind = CANTP_WORK_POLL;
        work->generation = rx.generation;
        work->id = configuration->upper_receive;
    } else {
        /* Nothing to call now. */
    }
    if (work->kind == CANTP_WORK_NONE) {
        running = FALSE;
    }
}
static CANTP_CODE void CanTp_Copied(const CanTp_WorkType *work, BufReq_ReturnType result)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == tx.generation) && (tx.state == CANTP_TX_COPY)) {
        if (result == BUFREQ_OK) {
            tx.payload = work->length;
            tx.state = CANTP_TX_SEND;
            tx.timer = configuration->n_as;
        } else if (result != BUFREQ_E_BUSY) {
            CanTp_TxFinish(E_NOT_OK); /* SWS_CanTp_00087 */
        } else {
            /* Retried at the next main function within N_Cs (SWS_CanTp_00184, 00280). */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
/* A refused N-PDU ends its connection (SWS_CanTp_00342, 00343). */
static CANTP_CODE void CanTp_Refused(const CanTp_WorkType *work)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((lower_owner == work->direction) && (lower_generation == work->generation)) {
        lower_owner = CANTP_LOWER_FREE;
        if ((work->direction == CANTP_LOWER_TX) && (tx.generation == work->generation)) {
            CanTp_TxFinish(E_NOT_OK);
        } else if ((work->direction == CANTP_LOWER_RX) && (rx.generation == work->generation)) {
            if (rx.flow == CANTP_FS_OVFLW) {
                CanTp_RxDrop();
            } else {
                CanTp_RxFinish(E_NOT_OK);
            }
        } else {
            /* The connection already ended. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
static CANTP_CODE void CanTp_Polled(const CanTp_WorkType *work,
                                    BufReq_ReturnType result,
                                    PduLengthType available)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == rx.generation) && (rx.state == CANTP_RX_BUFFER)) {
        if (result != BUFREQ_OK) {
            CanTp_RxFinish(E_NOT_OK);
        } else if (available >= (rx.total - rx.position)) {
            rx.flow = CANTP_FS_CTS; /* SWS_CanTp_00224 */
            rx.state = CANTP_RX_FLOW_SEND;
            rx.timer = configuration->n_ar;
        } else {
            /* Keep asking until N_Br expires. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
/* Copies the rest of an admitted SF or FF; returns the error. */
static CANTP_CODE uint8 CanTp_AdmitData(const CanTp_WorkType *work, const PduInfoType *payload)
{
    uint8 error = 0u;
    PduLengthType available = 0u;
    const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, payload, &available);
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == rx.generation) && (rx.state == CANTP_RX_START)) {
        if (result != BUFREQ_OK) {
            CanTp_RxFinish(E_NOT_OK);
            error = CANTP_E_COM;
        } else if (rx.total <= CANTP_SF_MAX) {
            CanTp_RxFinish(E_OK);
        } else {
            rx.position = payload->SduLength;
            if (available >= (rx.total - rx.position)) {
                rx.flow = CANTP_FS_CTS; /* SWS_CanTp_00224 */
                rx.state = CANTP_RX_FLOW_SEND;
                rx.timer = configuration->n_ar;
            } else {
                rx.state = CANTP_RX_BUFFER; /* N_Br keeps running (SWS_CanTp_00082). */
            }
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    return error;
}
/* Starts a reception from a valid SF or FF (SWS_CanTp_00079, 00081, 00318, 00339, 00353). */
static CANTP_CODE uint8 CanTp_Admit(CanTp_WorkType *work)
{
    uint8 error = 0u;
    boolean copy = FALSE;
    const boolean first = ((work->frame[0] / 16u) == CANTP_PCI_FF) ? TRUE : FALSE;
    const PduLengthType sf_dl = (PduLengthType)work->frame[0] & 0x0fu;
    const PduLengthType length =
        (first == TRUE) ? (PduLengthType)((sf_dl * 256u) + work->frame[1]) : sf_dl;
    const PduInfoType payload = {
        &work->frame[(first == TRUE) ? 2u : 1u], NULL_PTR, (first == TRUE) ? CANTP_FF_DATA : sf_dl};
    PduLengthType available = 0u;
    const BufReq_ReturnType result =
        PduR_CanTpStartOfReception(work->id, &payload, length, &available);
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == rx.generation) && (rx.state == CANTP_RX_START)) {
        if ((result == BUFREQ_OK) && (available < payload.SduLength)) {
            CanTp_RxFinish(E_NOT_OK);
            error = CANTP_E_COM;
        } else if (result == BUFREQ_OK) {
            copy = TRUE;
        } else if ((result == BUFREQ_E_OVFL) && (first == TRUE)) {
            rx.flow = CANTP_FS_OVFLW;
            rx.state = CANTP_RX_FLOW_SEND;
            rx.timer = configuration->n_ar;
        } else {
            CanTp_RxDrop();
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    if (copy == TRUE) {
        error = CanTp_AdmitData(work, &payload);
    }
    return error;
}
/* A received CF of the reception (SWS_CanTp_00269, 00271, 00314, 00346). */
static CANTP_CODE uint8 CanTp_Consecutive(CanTp_WorkType *work)
{
    uint8 error = 0u;
    boolean copy = FALSE;
    PduInfoType payload = {&work->frame[1], NULL_PTR, 0u};
    SchM_Enter_CanTp_CANTP_STATE();
    if ((rx.state != CANTP_RX_DATA) || (work->generation != rx.generation)) {
        /* A CF is only expected after the FC(CTS) was confirmed, by the same reception. */
    } else if (work->length < CANTP_FRAME_BYTES) {
        CanTp_RxFinish(E_NOT_OK);
        error = CANTP_E_PADDING;
    } else if ((work->frame[0] & 0x0fu) != rx.sequence) {
        CanTp_RxFinish(E_NOT_OK);
        error = CANTP_E_COM;
    } else {
        payload.SduLength = rx.total - rx.position;
        if (payload.SduLength > CANTP_CF_DATA) {
            payload.SduLength = CANTP_CF_DATA;
        }
        work->id = configuration->upper_receive;
        copy = TRUE;
    }
    SchM_Exit_CanTp_CANTP_STATE();
    if (copy == TRUE) {
        PduLengthType available = 0u;
        const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, &payload, &available);
        SchM_Enter_CanTp_CANTP_STATE();
        if ((work->generation == rx.generation) && (rx.state == CANTP_RX_DATA)) {
            if (result != BUFREQ_OK) {
                CanTp_RxFinish(E_NOT_OK);
                error = CANTP_E_COM;
            } else {
                rx.position += payload.SduLength;
                rx.sequence = (uint8)((rx.sequence + 1u) % 16u);
                rx.timer = configuration->n_cr;
                if (rx.position == rx.total) {
                    CanTp_RxFinish(E_OK);
                }
            }
        }
        SchM_Exit_CanTp_CANTP_STATE();
    }
    return error;
}
/* A received SF, FF or CF (SWS_CanTp_00057 table 1, 00124, 00345). */
static CANTP_CODE uint8 CanTp_Frame(CanTp_WorkType *work)
{
    uint8 error = 0u;
    const uint8 kind = work->frame[0] / 16u;
    if (kind == CANTP_PCI_CF) {
        error = CanTp_Consecutive(work);
    } else {
        boolean admit = FALSE;
        SchM_Enter_CanTp_CANTP_STATE();
        if (work->generation != rx.generation) {
            /* Received before Init or Shutdown lost the connections. */
        } else if ((kind == CANTP_PCI_SF) && (work->length < CANTP_FRAME_BYTES)) {
            error = CANTP_E_PADDING; /* Rejected; a reception in progress continues. */
        } else if ((CanTp_Starts(work->frame, (uint8)work->length) == TRUE) &&
                   (rx.state == CANTP_RX_IDLE)) {
            const PduLengthType sf_dl = (PduLengthType)work->frame[0] & 0x0fu;
            rx.total =
                (kind == CANTP_PCI_FF) ? (PduLengthType)((sf_dl * 256u) + work->frame[1]) : sf_dl;
            rx.position = 0u;
            rx.sequence = 1u;
            rx.waits = 0u;
            rx.due = FALSE;
            rx.flow = CANTP_FS_CTS;
            rx.timer = configuration->n_br; /* Before the upper call (SWS_CanTp_00166). */
            rx.state = CANTP_RX_START;
            ++rx.generation;
            work->generation = rx.generation;
            work->id = configuration->upper_receive;
            admit = TRUE;
        } else {
            /* Malformed SF or FF: ignored (ISO 15765-2). */
        }
        SchM_Exit_CanTp_CANTP_STATE();
        if (admit == TRUE) {
            error = CanTp_Admit(work);
        }
    }
    return error;
}
static CANTP_CODE void CanTp_Perform(CanTp_WorkType *work)
{
    if (work->kind == CANTP_WORK_TX_DONE) {
        PduR_CanTpTxConfirmation(work->id, work->result);
    } else if (work->kind == CANTP_WORK_RX_DONE) {
        PduR_CanTpRxIndication(work->id, work->result);
    } else if (work->kind == CANTP_WORK_FRAME) {
        CanTp_Report(CANTP_SID_RX_INDICATE, CanTp_Frame(work));
    } else if (work->kind == CANTP_WORK_COPY_TX) {
        /* No retry information: ISO 15765-2 has no recovery (SWS_CanTp_00272). */
        PduLengthType available = 0u;
        const PduInfoType data = {&tx.frame[work->offset], NULL_PTR, work->length};
        CanTp_Copied(work, PduR_CanTpCopyTxData(work->id, &data, NULL_PTR, &available));
    } else if (work->kind == CANTP_WORK_LOWER) {
        const PduInfoType frame = {work->frame, NULL_PTR, CANTP_FRAME_BYTES};
        if (LSduR_CanTpTransmit(work->id, &frame) != E_OK) {
            CanTp_Refused(work);
        }
    } else {
        PduLengthType available = 0u;
        const PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
        const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, &query, &available);
        CanTp_Polled(work, result, available);
    }
}
/* Makes the calls owed so far unless another context is making them (ADR 0018); that
 * context continues until nothing is left, so no recorded event is lost. Without `send`,
 * frames wait for the next run that allows them, at the latest the main function. */
static CANTP_CODE void CanTp_Run(boolean send)
{
    boolean more = FALSE;
    SchM_Enter_CanTp_CANTP_STATE();
    if (running == FALSE) {
        running = TRUE;
        more = TRUE;
    }
    SchM_Exit_CanTp_CANTP_STATE();
    while (more == TRUE) {
        CanTp_WorkType work;
        SchM_Enter_CanTp_CANTP_STATE();
        CanTp_Next(&work, send);
        SchM_Exit_CanTp_CANTP_STATE();
        more = (work.kind != CANTP_WORK_NONE) ? TRUE : FALSE;
        if (more == TRUE) {
            CanTp_Perform(&work);
        }
    }
}
CANTP_CODE void CanTp_Init(const CanTp_ConfigType *CfgPtr)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((CfgPtr != NULL_PTR) && (CfgPtr->maximum_length > CANTP_SF_MAX) &&
        (CfgPtr->maximum_length <= 256u) && (CfgPtr->n_as > 0u) && (CfgPtr->n_ar > 0u) &&
        (CfgPtr->n_bs > 0u) && (CfgPtr->n_br > 0u) && (CfgPtr->n_cr > 0u) && (CfgPtr->n_cs > 0u) &&
        (CfgPtr->main_period_ms > 0u)) {
        configuration = CfgPtr;
        CanTp_Reset(); /* All connections are lost without notification (SWS_CanTp_00273). */
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
CANTP_CODE void CanTp_Shutdown(void)
{
    SchM_Enter_CanTp_CANTP_STATE();
    configuration = NULL_PTR;
    CanTp_Reset(); /* No notification (SWS_CanTp_00200). */
    SchM_Exit_CanTp_CANTP_STATE();
}
CANTP_CODE Std_ReturnType CanTp_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanTp_CANTP_STATE();
    /* Only the length is used (SWS_CanTp_00225, 00299); a busy channel refuses (00123). */
    if ((configuration != NULL_PTR) && (TxPduId == configuration->transmit) &&
        (PduInfoPtr != NULL_PTR) && (PduInfoPtr->MetaDataPtr == NULL_PTR) &&
        (PduInfoPtr->SduLength > 0u) && (PduInfoPtr->SduLength <= configuration->maximum_length) &&
        (tx.state == CANTP_TX_IDLE)) {
        tx.total = PduInfoPtr->SduLength;
        tx.position = 0u;
        tx.sequence = 1u;
        tx.block_size = 0u;
        tx.block_sent = 0u;
        tx.separation = 0u;
        tx.delay = 0u;
        tx.due = TRUE;
        tx.timer = configuration->n_cs; /* Before the first copy (SWS_CanTp_00167). */
        tx.state = CANTP_TX_COPY;
        result = E_OK;
    }
    SchM_Exit_CanTp_CANTP_STATE();
    if (result == E_OK) {
        CanTp_Run(TRUE);
    }
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_CanTp_00246) that no
 * module of the selected program calls; see the CanTp deviation record. */
CANTP_CODE Std_ReturnType CanTp_CancelTransmit(PduIdType TxPduId)
{
    boolean known;
    SchM_Enter_CanTp_CANTP_STATE();
    known = ((configuration != NULL_PTR) && (TxPduId == configuration->transmit)) ? TRUE : FALSE;
    SchM_Exit_CanTp_CANTP_STATE();
    /* CanTpTc is false: no transmission of this N-SDU can be cancelled (SWS_CanTp_00242). */
    if (known == TRUE) {
        CanTp_Report(CANTP_SID_CANCEL_TX, CANTP_E_OPER_NOT_SUPP);
    }
    return E_NOT_OK;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_CanTp_00257) that no
 * module of the selected program calls; see the CanTp deviation record. */
CANTP_CODE Std_ReturnType CanTp_CancelReceive(PduIdType RxPduId)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 error = 0u;
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (RxPduId == configuration->receive)) {
        if ((CanTp_Receiving() == TRUE) && (rx.flow != CANTP_FS_OVFLW)) {
            /* Refused while the last CF is awaited (SWS_CanTp_00262). The FC in flight
             * keeps the N-PDU until its own confirmation (SWS_CanTp_00075). */
            if ((rx.state != CANTP_RX_DATA) ||
                (((uint32)rx.total - (uint32)rx.position) > CANTP_CF_DATA)) {
                CanTp_RxFinish(E_NOT_OK);
                result = E_OK;
            }
        } else if (rx.state != CANTP_RX_START) {
            error = CANTP_E_OPER_NOT_SUPP;
        } else {
            /* An SF or FF is being admitted: not cancellable yet. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    CanTp_Report(CANTP_SID_CANCEL_RX, error);
    if (result == E_OK) {
        CanTp_Run(TRUE);
    }
    return result;
}
CANTP_CODE void CanTp_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    uint8 error = 0u;
    SchM_Enter_CanTp_CANTP_STATE();
    if ((configuration != NULL_PTR) && (RxPduId == configuration->lower_receive) &&
        (PduInfoPtr != NULL_PTR) && (PduInfoPtr->SduDataPtr != NULL_PTR) &&
        (PduInfoPtr->MetaDataPtr == NULL_PTR) && (PduInfoPtr->SduLength > 0u) &&
        (PduInfoPtr->SduLength <= CANTP_FRAME_BYTES)) {
        const uint8 length = (uint8)PduInfoPtr->SduLength;
        const uint8 kind = PduInfoPtr->SduDataPtr[0] / 16u;
        if (kind == CANTP_PCI_FC) {
            error = CanTp_ReceiveFlow(PduInfoPtr->SduDataPtr, length);
        } else if ((kind <= CANTP_PCI_CF) && (queue_count < CANTP_RX_QUEUE)) {
            const uint8 slot = (uint8)((queue_head + queue_count) % CANTP_RX_QUEUE);
            uint8 i;
            for (i = 0u; i < CANTP_FRAME_BYTES; ++i) {
                queued[slot][i] = (i < length) ? PduInfoPtr->SduDataPtr[i] : 0u;
            }
            queued_length[slot] = length;
            ++queue_count;
        } else if (kind <= CANTP_PCI_CF) {
            error = CANTP_E_COM; /* Arrived faster than the calls it needs could be made. */
        } else {
            /* Unknown N-PDU type: ignored (SWS_CanTp_00057). */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    CanTp_Report(CANTP_SID_RX_INDICATE, error);
    CanTp_Run(TRUE);
}
CANTP_CODE void CanTp_TxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((lower_owner != CANTP_LOWER_FREE) && (TxPduId == pending_lower_id)) {
        const CanTp_LowerType owner = lower_owner;
        lower_owner = CANTP_LOWER_FREE;
        if ((owner == CANTP_LOWER_TX) && (lower_generation == tx.generation) &&
            (tx.state == CANTP_TX_CONFIRM)) {
            CanTp_TxConfirmed(result);
        } else if ((owner == CANTP_LOWER_RX) && (lower_generation == rx.generation) &&
                   (rx.state == CANTP_RX_FLOW_CONFIRM)) {
            CanTp_FlowConfirmed(result);
        } else {
            /* Late confirmation after a timeout, Init or Shutdown: only frees the N-PDU. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    /* The driver may hold its slot until this confirmation returns, so the next frame
     * waits for the main function, as STmin 0 allows. */
    CanTp_Run(FALSE);
}
CANTP_CODE void CanTp_MainFunction(void)
{
    uint8 tx_error = 0u;
    uint8 rx_error = 0u;
    SchM_Enter_CanTp_CANTP_STATE();
    if (configuration != NULL_PTR) {
        tx_error = CanTp_TxTick();
        rx_error = CanTp_RxTick();
    }
    SchM_Exit_CanTp_CANTP_STATE();
    CanTp_Report(CANTP_SID_MAIN, tx_error);
    CanTp_Report(CANTP_SID_MAIN, rx_error);
    CanTp_Run(TRUE);
}
#define CANTP_STOP_SEC_CODE
#include "CanTp_MemMap.h"
