/** @file CanTp for physical, padded Classical CAN and CAN FD N-SDUs.
 * Reception and transmission are independent (full duplex, SWS_CanTp_00057) and share the
 * Tx N-PDU, which its TxConfirmation serializes (SWS_CanTp_00248). CanTp enters only its
 * own CANTP_STATE area and calls PduR, LSduR and Det outside it: events are
 * recorded in the area, and one context at a time makes the calls they require.
 */
#include "CanTp_Internal.h"
#include "PduR_CanTp.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PDUR_AR_RELEASE_MAJOR_VERSION) || !defined(PDUR_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> PduR_CanTp.h: missing AUTOSAR release"
#elif (PDUR_AR_RELEASE_MAJOR_VERSION != 4U) || (PDUR_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> PduR_CanTp.h: incompatible AUTOSAR release"
#endif
#include "LSduR_CanTp.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(LSDUR_AR_RELEASE_MAJOR_VERSION) || !defined(LSDUR_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> LSduR_CanTp.h: missing AUTOSAR release"
#elif (LSDUR_AR_RELEASE_MAJOR_VERSION != 4U) || (LSDUR_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> LSduR_CanTp.h: incompatible AUTOSAR release"
#endif
#include "SchM_CanTp.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> SchM_CanTp.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> SchM_CanTp.h: incompatible AUTOSAR release"
#endif
#include "Det.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(DET_AR_RELEASE_MAJOR_VERSION) || !defined(DET_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> Det.h: missing AUTOSAR release"
#elif (DET_AR_RELEASE_MAJOR_VERSION != 4U) || (DET_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> Det.h: incompatible AUTOSAR release"
#endif
#include "Can_GeneralTypes.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CAN_AR_RELEASE_MAJOR_VERSION) || !defined(CAN_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> Can_GeneralTypes.h: missing AUTOSAR release"
#elif (CAN_AR_RELEASE_MAJOR_VERSION != 4U) || (CAN_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> Can_GeneralTypes.h: incompatible AUTOSAR release"
#endif
#include "Ecu_ProfileConfig.h"
/* Runtime error values (SWS_CanTp_00352); zero means none. */
#define CANTP_E_PADDING       0x70u
#define CANTP_E_OPER_NOT_SUPP 0xa0u
#define CANTP_E_COM           0xb0u
#define CANTP_E_RX_COM        0xc0u
#define CANTP_E_TX_COM        0xd0u
#define CANTP_FRAME_BYTES     CAN_FD_MAX_LENGTH /**< Largest N-PDU buffer in the FD profile. */
#define CANTP_MIN_PADDED_DL   8u                /**< Padded small frames and FC use eight bytes. */
#define CANTP_SHORT_SF_MAX    7u                /**< Data bytes after one SF PCI byte. */
#define CANTP_SHORT_FF_MAX    4095u             /**< Largest 12-bit FF_DL before escape. */
#define CANTP_PCI_SF          0u
#define CANTP_PCI_FF          1u
#define CANTP_PCI_CF          2u
#define CANTP_PCI_FC          3u
#define CANTP_FS_CTS          0u
#define CANTP_FS_WAIT         1u
#define CANTP_FS_OVFLW        2u
#define CANTP_STMIN_MAX_MS    0x7fu
/* SF, FF and CF waiting while another context makes calls; full means dropped. */
#define CANTP_RX_QUEUE  4u
#define CANTP_TIMER_MIN 2u

#define CANTP_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"

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
    uint32 connection;
    CanTp_WorkKindType kind;
    CanTp_LowerType direction;
    uint32 generation;
    PduIdType id;
    Std_ReturnType result;
    PduLengthType offset;
    PduLengthType length;
    uint8 frame[CANTP_FRAME_BYTES];
} CanTp_WorkType;
/* The owner identifies receptions before a pending cycle; only timers actually reloaded
 * by that reception exclude the pending cycle from their elapsed time. */
typedef enum { CANTP_RECEIVE_AFTER_MAIN, CANTP_RECEIVE_BEFORE_MAIN } CanTp_ReceiveOrderType;
typedef struct {
    uint32 remaining;             /**< Complete main-function periods still owed. */
    boolean reloaded_before_main; /**< The pending cycle has not elapsed since reload. */
} CanTp_TimerType;
/* The transmitting side of the channel. CanTp_Transmit sets every member but result and
 * frame; CanTp_Init and CanTp_Shutdown only reset state and advance generation. A call made
 * outside the area for an earlier generation has a stale result. */
typedef struct {
    CanTp_TxStateType state;        /**< Step of the transmission. */
    uint32 generation;              /**< Advanced at each end or reset. */
    CanTp_TimerType timer;          /**< N_As, N_Bs or N_Cs. */
    uint32 delay;                   /**< Remaining periods of STmin before the next CF. */
    uint32 separation;              /**< STmin of the last FC, in periods. */
    boolean due;                    /**< The next SF, FF or CF may be copied now. */
    PduLengthType total;            /**< N-SDU length in bytes. */
    PduLengthType position;         /**< Bytes already sent. */
    PduLengthType payload;          /**< Data bytes in the frame being sent. */
    uint8 sequence;                 /**< SN of the next CF, 0 to 15. */
    uint8 block_size;               /**< BS of the last FC; 0 means no further FC. */
    uint8 block_sent;               /**< CFs sent in the current block. */
    Std_ReturnType result;          /**< Result owed to PduR in CANTP_TX_DONE. */
    uint8 frame_length;             /**< Padded CAN_DL of the frame being sent. */
    uint8 frame[CANTP_FRAME_BYTES]; /**< Frame being built or sent. */
} CanTp_TxType;
/* The receiving side of the channel. Admitting an SF or FF sets every member but result
 * and frame; CanTp_Init and CanTp_Shutdown only reset state and advance generation. A call
 * made outside the area for an earlier generation has a stale result. */
typedef struct {
    CanTp_RxStateType state;          /**< Step of the reception. */
    uint32 generation;                /**< Advanced at each start, end or reset. */
    CanTp_TimerType timer;            /**< N_Ar, N_Br or N_Cr. */
    boolean due;                      /**< BUFFER state: ask PduR for buffer again. */
    uint8 flow;                       /**< FS of the FC to send next: CTS, WAIT or OVFLW. */
    uint8 waits;                      /**< FC(WAIT) frames sent, at most wait_frame_max. */
    PduLengthType total;              /**< N-SDU length in bytes from the SF or FF. */
    PduLengthType position;           /**< Bytes already copied to PduR. */
    uint8 sequence;                   /**< SN expected in the next CF, 0 to 15. */
    Std_ReturnType result;            /**< Result owed to PduR in CANTP_RX_DONE. */
    uint8 rx_dl;                      /**< RX_DL set by the first frame. */
    uint8 frame[CANTP_MIN_PADDED_DL]; /**< FC frame being built or sent. */
} CanTp_RxType;

/* All connection state is protected by CANTP_STATE. A Tx N-PDU claim is kept
 * across reset until the lower layer returns its physical-route confirmation. */
typedef struct {
    const CanTp_ConnectionConfigType *config;
    CanTp_TxType tx;
    CanTp_RxType rx;
    CanTp_LowerType lower_owner;
    uint32 lower_generation;
    PduIdType pending_lower_id;
    uint8 queued[CANTP_RX_QUEUE][CANTP_FRAME_BYTES];
    uint8 queued_length[CANTP_RX_QUEUE];
    uint8 queue_head;
    uint8 queue_count;
} CanTp_ConnectionType;
static CanTp_ConnectionType
    connections[CANTP_CONNECTION_CAPACITY] CANTP_VAR_CLEARED; /**< Connection state. */
static uint32 connection_count CANTP_VAR_CLEARED;             /**< Inactive when zero. */
/* A context is making CanTp's calls to other modules; never reset, so a reinitialization
 * cannot start a second runner beside it. */
static boolean cantp_running CANTP_VAR_CLEARED;                /**< A runner is active. */
static CanTp_ReceiveOrderType receive_order CANTP_VAR_CLEARED; /**< Owner's receive ordering. */
#define CANTP_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanTp_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "CanTp.c -> CanTp_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp.c -> CanTp_MemMap.h: incompatible AUTOSAR release"
#endif

#define CANTP_START_SEC_CODE
#include "CanTp_MemMap.h"
static CANTP_CODE void CanTp_Report(uint8 ApiId, uint8 ErrorId)
{
    if (ErrorId != 0u) {
        (void)Det_ReportRuntimeError(CANTP_MODULE_ID, 0u, ApiId, ErrorId);
    }
}
/* The helpers below up to CanTp_Run run inside CANTP_STATE. */
static CANTP_CODE void CanTp_StartTimer(CanTp_TimerType *timer, uint32 periods)
{
    timer->remaining = periods;
    timer->reloaded_before_main = (receive_order == CANTP_RECEIVE_BEFORE_MAIN);
}
static CANTP_CODE boolean CanTp_TimerElapsed(CanTp_TimerType *timer)
{
    if (timer->reloaded_before_main) {
        timer->reloaded_before_main = FALSE;
    } else if (timer->remaining > 0u) {
        --timer->remaining;
    } else {
        /* An elapsed timer stays elapsed until its state is replaced. */
    }
    return (timer->remaining == 0u) ? TRUE : FALSE;
}

static CANTP_CODE void CanTp_TxFinish(CanTp_ConnectionType *connection, Std_ReturnType result)
{
    connection->tx.state = CANTP_TX_DONE;
    connection->tx.result = result;
    ++connection->tx.generation;
}
static CANTP_CODE void CanTp_RxFinish(CanTp_ConnectionType *connection, Std_ReturnType result)
{
    connection->rx.state = CANTP_RX_DONE;
    connection->rx.result = result;
    ++connection->rx.generation;
}
/* Ends a reception the upper layer never accepted: no indication (SWS_CanTp_00081). */
static CANTP_CODE void CanTp_RxDrop(CanTp_ConnectionType *connection)
{
    connection->rx.state = CANTP_RX_IDLE;
    ++connection->rx.generation;
}
static CANTP_CODE void CanTp_Reset(CanTp_ConnectionType *connection)
{
    connection->tx.state = CANTP_TX_IDLE;
    ++connection->tx.generation;
    CanTp_RxDrop(connection);
    connection->queue_count = 0u;
}
static CANTP_CODE boolean CanTp_Receiving(const CanTp_ConnectionType *connection)
{
    return ((connection->rx.state == CANTP_RX_BUFFER) ||
            (connection->rx.state == CANTP_RX_FLOW_SEND) ||
            (connection->rx.state == CANTP_RX_FLOW_CONFIRM) ||
            (connection->rx.state == CANTP_RX_DATA))
               ? TRUE
               : FALSE;
}
static CANTP_CODE uint8 CanTp_WireLength(uint8 length)
{
    return (length <= CANTP_MIN_PADDED_DL) ? CANTP_MIN_PADDED_DL
                                           : Can_FdDlcToLength(Can_FdLengthToDlc(length));
}
static CANTP_CODE uint8 CanTp_SingleCapacity(uint8 frame_length)
{
    return (frame_length <= CANTP_MIN_PADDED_DL) ? CANTP_SHORT_SF_MAX : (frame_length - 2u);
}
typedef struct {
    uint32 total;
    uint8 pci;
} CanTp_StartType;
static CANTP_CODE CanTp_StartType CanTp_DecodeStart(const uint8 *data, uint8 length)
{
    CanTp_StartType start = {0u, 0u};
    const uint8 kind = data[0] / 16u;
    const uint8 low = data[0] & 0x0fu;
    if (length >= CANTP_MIN_PADDED_DL) {
        if (kind == CANTP_PCI_SF) {
            if (length == CANTP_MIN_PADDED_DL) {
                if ((low > 0u) && (low <= CANTP_SHORT_SF_MAX)) {
                    start.total = low;
                    start.pci = 1u;
                }
            } else if ((low == 0u) && (data[1] > CANTP_SHORT_SF_MAX) &&
                       (data[1] <= CanTp_SingleCapacity(length)) &&
                       (CanTp_WireLength(2u + data[1]) == length)) {
                start.total = data[1];
                start.pci = 2u;
            } else {
                /* Invalid SF escape. */
            }
        } else if (kind == CANTP_PCI_FF) {
            const uint32 total = ((uint32)low * 256u) + (uint32)data[1];
            if (total == 0u) {
                const uint32 extended = ((uint32)data[2] << 24u) | ((uint32)data[3] << 16u) |
                                        ((uint32)data[4] << 8u) | (uint32)data[5];
                if (extended > CANTP_SHORT_FF_MAX) {
                    start.total = extended;
                    start.pci = 6u;
                }
            } else if (total > CanTp_SingleCapacity(length)) {
                start.total = total;
                start.pci = 2u;
            } else {
                /* The data fits in an SF. */
            }
        } else {
            /* Only SF and FF start a reception. */
        }
    }
    return start;
}
static CANTP_CODE void CanTp_Pad(uint8 *frame, uint8 used, uint8 length, uint8 padding)
{
    for (uint8 i = used; i < length; ++i) {
        frame[i] = padding; /* SWS_CanTp_00351 */
    }
}
/* Ticks before the next CF: one more than STmin needs, because the first tick may follow
 * the confirmation at once and STmin is a minimum. Reserved values mean 127 ms and
 * 100-900 us one tick (ISO 15765-2). */
static CANTP_CODE uint32 CanTp_Separation(const CanTp_ConnectionType *connection, uint8 stmin)
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
    ticks = (milliseconds + connection->config->main_period_ms - 1u) /
            connection->config->main_period_ms;
    return (ticks == 0u) ? 0u : (ticks + 1u);
}
/* An FC for the transmission (SWS_CanTp_00315, 00309, 00317, 00349); returns the error. */
static CANTP_CODE uint8 CanTp_ReceiveFlow(CanTp_ConnectionType *connection,
                                          const uint8 *data,
                                          uint8 length)
{
    uint8 error = 0u;
    const uint8 status = data[0] & 0x0fu;
    if (connection->tx.state != CANTP_TX_FLOW) {
        /* An FC that is not awaited is ignored (SWS_CanTp_00057). */
    } else if (length < CANTP_MIN_PADDED_DL) {
        CanTp_TxFinish(connection, E_NOT_OK);
        error = CANTP_E_PADDING;
    } else if (status == CANTP_FS_CTS) {
        connection->tx.block_size = data[1];
        connection->tx.block_sent = 0u;
        connection->tx.separation = CanTp_Separation(connection, data[2]);
        connection->tx.delay = 0u; /* STmin separates CFs; the first follows the FC at once. */
        connection->tx.due = TRUE;
        connection->tx.state = CANTP_TX_COPY;
        CanTp_StartTimer(&connection->tx.timer, connection->config->n_cs);
    } else if (status == CANTP_FS_WAIT) {
        CanTp_StartTimer(&connection->tx.timer, connection->config->n_bs);
    } else {
        CanTp_TxFinish(connection, E_NOT_OK);
        error = CANTP_E_COM;
    }
    return error;
}
static CANTP_CODE void CanTp_TxConfirmed(CanTp_ConnectionType *connection, Std_ReturnType result)
{
    if (result != E_OK) {
        CanTp_TxFinish(connection, E_NOT_OK); /* SWS_CanTp_00355 */
    } else {
        const boolean first = (connection->tx.position == 0u);
        connection->tx.position += connection->tx.payload;
        if (connection->tx.position == connection->tx.total) {
            CanTp_TxFinish(connection, E_OK);
        } else if ((first == TRUE) ||
                   ((connection->tx.block_size > 0u) &&
                    ((connection->tx.block_sent + 1u) >= connection->tx.block_size))) {
            connection->tx.state = CANTP_TX_FLOW;
            CanTp_StartTimer(&connection->tx.timer, connection->config->n_bs);
            if (first == FALSE) {
                connection->tx.sequence = (uint8)((connection->tx.sequence + 1u) % 16u);
            }
        } else {
            ++connection->tx.block_sent;
            connection->tx.sequence = (uint8)((connection->tx.sequence + 1u) % 16u);
            connection->tx.state = CANTP_TX_COPY;
            CanTp_StartTimer(&connection->tx.timer, connection->config->n_cs);
            connection->tx.delay = connection->tx.separation;
            connection->tx.due = (connection->tx.delay == 0u) ? TRUE : FALSE;
        }
    }
}
static CANTP_CODE void CanTp_FlowConfirmed(CanTp_ConnectionType *connection, Std_ReturnType result)
{
    if (connection->rx.flow == CANTP_FS_OVFLW) {
        CanTp_RxDrop(connection); /* The reception was refused at admission (SWS_CanTp_00318). */
    } else if (result != E_OK) {
        CanTp_RxFinish(connection, E_NOT_OK);
    } else if (connection->rx.flow == CANTP_FS_WAIT) {
        connection->rx.state = CANTP_RX_BUFFER;
        CanTp_StartTimer(&connection->rx.timer, connection->config->n_br);
    } else {
        connection->rx.state = CANTP_RX_DATA;
        CanTp_StartTimer(&connection->rx.timer, connection->config->n_cr); /* SWS_CanTp_00312 */
    }
}
static CANTP_CODE uint8 CanTp_TxTick(CanTp_ConnectionType *connection)
{
    uint8 error = 0u;
    if ((connection->tx.state == CANTP_TX_COPY) && (connection->tx.delay > 0u)) {
        --connection->tx.delay;
        connection->tx.due = (connection->tx.delay == 0u) ? TRUE : FALSE;
    } else if ((connection->tx.state != CANTP_TX_IDLE) && (connection->tx.state != CANTP_TX_DONE)) {
        if (CanTp_TimerElapsed(&connection->tx.timer) == TRUE) {
            /* N_As, N_Bs or N_Cs (SWS_CanTp_00229, 00280, 00310, 00316). */
            CanTp_TxFinish(connection, E_NOT_OK);
            error = CANTP_E_TX_COM;
        } else if (connection->tx.state == CANTP_TX_COPY) {
            connection->tx.due = TRUE; /* Retry a busy upper copy within N_Cs (SWS_CanTp_00184). */
        } else {
            /* Waiting for the N-PDU, its confirmation or an FC. */
        }
    } else {
        /* Nothing is timed. */
    }
    return error;
}
static CANTP_CODE uint8 CanTp_RxTick(CanTp_ConnectionType *connection)
{
    uint8 error = 0u;
    if (CanTp_Receiving(connection) == TRUE) {
        const boolean elapsed = CanTp_TimerElapsed(&connection->rx.timer);
        if (connection->rx.state == CANTP_RX_BUFFER) {
            /* Ask the upper layer for buffer again; at N_Br expiry this last answer decides
             * (SWS_CanTp_00222, 00341). */
            connection->rx.due = TRUE;
        } else if (elapsed) {
            /* N_Ar or N_Cr (SWS_CanTp_00223, 00311, 00313). */
            if ((connection->rx.state != CANTP_RX_DATA) &&
                (connection->rx.flow == CANTP_FS_OVFLW)) {
                CanTp_RxDrop(connection);
            } else {
                CanTp_RxFinish(connection, E_NOT_OK);
            }
            error = CANTP_E_RX_COM;
        } else {
            /* Waiting for the N-PDU, its confirmation or a CF. */
        }
    }
    return error;
}
/* Claims the Tx N-PDU for the frame of `direction`; the caller sends it after the area. */
static CANTP_CODE void CanTp_Claim(CanTp_ConnectionType *connection,
                                   CanTp_WorkType *work,
                                   CanTp_LowerType direction)
{
    const uint8 *frame =
        (direction == CANTP_LOWER_TX) ? connection->tx.frame : connection->rx.frame;
    const uint8 length =
        (direction == CANTP_LOWER_TX) ? connection->tx.frame_length : CANTP_MIN_PADDED_DL;
    connection->lower_owner = direction;
    connection->lower_generation =
        (direction == CANTP_LOWER_TX) ? connection->tx.generation : connection->rx.generation;
    connection->pending_lower_id = connection->config->lower_transmit;
    for (uint8 i = 0u; i < length; ++i) {
        work->frame[i] = frame[i];
    }
    work->length = length;
    work->kind = CANTP_WORK_LOWER;
    work->direction = direction;
    work->generation = connection->lower_generation;
    work->id = connection->pending_lower_id;
}
/* Builds the next SF, FF or CF header and asks for its data (SWS_CanTp_00086, 00231/00232). */
static CANTP_CODE void CanTp_PrepareCopy(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    const uint8 tx_dl = connection->config->tx_dl;
    const uint32 total = (uint32)connection->tx.total;
    uint8 *const frame = connection->tx.frame;
    uint8 pci = 1u;
    if (total <= CANTP_SHORT_SF_MAX) {
        frame[0] = (uint8)total;
    } else if (total <= CanTp_SingleCapacity(tx_dl)) {
        frame[0] = 0u;
        frame[1] = (uint8)total;
        pci = 2u;
    } else if (connection->tx.position != 0u) {
        frame[0] = (uint8)(0x20u | connection->tx.sequence);
    } else if (total <= CANTP_SHORT_FF_MAX) {
        frame[0] = (uint8)(0x10u | (uint8)(total / 256u));
        frame[1] = (uint8)(total % 256u);
        pci = 2u;
    } else {
        frame[0] = 0x10u;
        frame[1] = 0u;
        frame[2] = (uint8)(total >> 24u);
        frame[3] = (uint8)(total >> 16u);
        frame[4] = (uint8)(total >> 8u);
        frame[5] = (uint8)total;
        pci = 6u;
    }
    const uint8 capacity = tx_dl - pci;
    work->offset = pci;
    work->length = connection->tx.total - connection->tx.position;
    if (work->length > capacity) {
        work->length = capacity;
    }
    const uint8 used = pci + (uint8)work->length;
    connection->tx.frame_length = CanTp_WireLength(used);
    CanTp_Pad(frame, used, connection->tx.frame_length, connection->config->padding);
    connection->tx.due = FALSE;
    work->kind = CANTP_WORK_COPY_TX;
    work->generation = connection->tx.generation;
    work->id = connection->config->upper_transmit;
}
static CANTP_CODE void CanTp_PrepareFlow(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    CanTp_Pad(connection->rx.frame, 3u, CANTP_MIN_PADDED_DL, connection->config->padding);
    /* The whole N-SDU is requested at once: BS 0 and STmin 0 for every FC (SWS_CanTp_00067). */
    connection->rx.frame[0] = (uint8)(0x30u | connection->rx.flow);
    connection->rx.frame[1] = 0u;
    connection->rx.frame[2] = 0u;
    connection->rx.state = CANTP_RX_FLOW_CONFIRM;
    CanTp_Claim(connection, work, CANTP_LOWER_RX);
}
/* Takes the oldest connection->queued frame. */
static CANTP_CODE void CanTp_TakeFrame(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    for (uint8 i = 0u; i < CANTP_FRAME_BYTES; ++i) {
        work->frame[i] = connection->queued[connection->queue_head][i];
    }
    work->length = connection->queued_length[connection->queue_head];
    work->kind = CANTP_WORK_FRAME;
    work->generation = connection->rx.generation;
    connection->queue_head = (uint8)((connection->queue_head + 1u) % CANTP_RX_QUEUE);
    --connection->queue_count;
}
static CANTP_CODE CanTp_ConnectionType *CanTp_Pending(PduIdType id)
{
    CanTp_ConnectionType *owner = NULL_PTR;
    for (uint32 i = 0u; (i < CANTP_CONNECTION_CAPACITY) && (owner == NULL_PTR); ++i) {
        if ((connections[i].lower_owner != CANTP_LOWER_FREE) &&
            (connections[i].pending_lower_id == id)) {
            owner = &connections[i];
        }
    }
    return owner;
}
/* Chooses the next call: owed notifications first, then received frames in arrival order;
 * a Tx copy or a frame to send on the Tx N-PDU only when `send` allows it. */
static CANTP_CODE void CanTp_NextConnection(CanTp_ConnectionType *connection,
                                            CanTp_WorkType *work,
                                            boolean send)
{
    work->kind = CANTP_WORK_NONE;
    if ((connection->queue_count > 0u) && (CanTp_Receiving(connection) == TRUE) &&
        (CanTp_DecodeStart(connection->queued[connection->queue_head],
                           connection->queued_length[connection->queue_head])
             .total != 0u)) {
        /* A valid SF or FF ends the reception in progress first (SWS_CanTp_00057 table 1). */
        if (connection->rx.flow == CANTP_FS_OVFLW) {
            CanTp_RxDrop(connection);
        } else {
            CanTp_RxFinish(connection, E_NOT_OK);
        }
    }
    if (connection->tx.state == CANTP_TX_DONE) {
        /* Free before the confirmation, which may already request the next N-SDU. */
        connection->tx.state = CANTP_TX_IDLE;
        work->kind = CANTP_WORK_TX_DONE;
        work->id = connection->config->upper_transmit;
        work->result = connection->tx.result;
    } else if (connection->rx.state == CANTP_RX_DONE) {
        connection->rx.state = CANTP_RX_IDLE;
        work->kind = CANTP_WORK_RX_DONE;
        work->id = connection->config->upper_receive;
        work->result = connection->rx.result;
    } else if (connection->queue_count > 0u) {
        CanTp_TakeFrame(connection, work);
    } else if (send && (connection->tx.state == CANTP_TX_COPY) && connection->tx.due) {
        CanTp_PrepareCopy(connection, work);
    } else if (send && (connection->rx.state == CANTP_RX_FLOW_SEND) &&
               (connection->lower_owner == CANTP_LOWER_FREE) &&
               (CanTp_Pending(connection->config->lower_transmit) == NULL_PTR)) {
        CanTp_PrepareFlow(connection, work);
    } else if (send && (connection->tx.state == CANTP_TX_SEND) &&
               (connection->lower_owner == CANTP_LOWER_FREE) &&
               (CanTp_Pending(connection->config->lower_transmit) == NULL_PTR)) {
        connection->tx.state = CANTP_TX_CONFIRM; /* Published before a synchronous confirmation. */
        CanTp_Claim(connection, work, CANTP_LOWER_TX);
    } else if ((connection->rx.state == CANTP_RX_BUFFER) && connection->rx.due) {
        connection->rx.due = FALSE;
        work->kind = CANTP_WORK_POLL;
        work->generation = connection->rx.generation;
        work->id = connection->config->upper_receive;
    } else {
        /* Nothing to call now. */
    }
}
static CANTP_CODE void CanTp_Copied(CanTp_ConnectionType *connection,
                                    const CanTp_WorkType *work,
                                    BufReq_ReturnType result)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == connection->tx.generation) &&
        (connection->tx.state == CANTP_TX_COPY)) {
        if (result == BUFREQ_OK) {
            connection->tx.payload = work->length;
            connection->tx.state = CANTP_TX_SEND;
            CanTp_StartTimer(&connection->tx.timer, connection->config->n_as);
        } else if (result != BUFREQ_E_BUSY) {
            CanTp_TxFinish(connection, E_NOT_OK); /* SWS_CanTp_00087 */
        } else {
            /* Retried at the next main function within N_Cs (SWS_CanTp_00184, 00280). */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
/* A refused N-PDU ends its connection (SWS_CanTp_00342, 00343). */
static CANTP_CODE void CanTp_Refused(CanTp_ConnectionType *connection, const CanTp_WorkType *work)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((connection->lower_owner == work->direction) &&
        (connection->lower_generation == work->generation)) {
        connection->lower_owner = CANTP_LOWER_FREE;
        if ((work->direction == CANTP_LOWER_TX) &&
            (connection->tx.generation == work->generation)) {
            CanTp_TxFinish(connection, E_NOT_OK);
        } else if ((work->direction == CANTP_LOWER_RX) &&
                   (connection->rx.generation == work->generation)) {
            if (connection->rx.flow == CANTP_FS_OVFLW) {
                CanTp_RxDrop(connection);
            } else {
                CanTp_RxFinish(connection, E_NOT_OK);
            }
        } else {
            /* The connection already ended. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
/* Returns the error of an N_Br expiry after WFTmax FC(WAIT) (SWS_CanTp_00341). */
static CANTP_CODE uint8 CanTp_Polled(CanTp_ConnectionType *connection,
                                     const CanTp_WorkType *work,
                                     BufReq_ReturnType result,
                                     PduLengthType available)
{
    uint8 error = 0u;
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == connection->rx.generation) &&
        (connection->rx.state == CANTP_RX_BUFFER)) {
        if (result != BUFREQ_OK) {
            CanTp_RxFinish(connection, E_NOT_OK);
        } else if (available >= (connection->rx.total - connection->rx.position)) {
            connection->rx.flow = CANTP_FS_CTS; /* SWS_CanTp_00224 */
            connection->rx.state = CANTP_RX_FLOW_SEND;
            CanTp_StartTimer(&connection->rx.timer, connection->config->n_ar);
        } else if (connection->rx.timer.remaining > 0u) {
            /* Keep asking until N_Br expires. */
        } else if (connection->rx.waits < connection->config->wait_frame_max) {
            ++connection->rx.waits; /* Suspend with FC(WAIT). */
            connection->rx.flow = CANTP_FS_WAIT;
            connection->rx.state = CANTP_RX_FLOW_SEND;
            CanTp_StartTimer(&connection->rx.timer, connection->config->n_ar);
        } else {
            CanTp_RxFinish(connection, E_NOT_OK);
            error = CANTP_E_RX_COM;
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    return error;
}
/* Copies the rest of an admitted SF or FF; returns the error. */
static CANTP_CODE uint8 CanTp_AdmitData(CanTp_ConnectionType *connection,
                                        const CanTp_WorkType *work,
                                        const PduInfoType *payload)
{
    uint8 error = 0u;
    PduLengthType available = 0u;
    const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, payload, &available);
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == connection->rx.generation) &&
        (connection->rx.state == CANTP_RX_START)) {
        if (result != BUFREQ_OK) {
            CanTp_RxFinish(connection, E_NOT_OK);
            error = CANTP_E_COM;
        } else if ((work->frame[0] / 16u) == CANTP_PCI_SF) {
            CanTp_RxFinish(connection, E_OK);
        } else {
            connection->rx.position = payload->SduLength;
            if (available >= (connection->rx.total - connection->rx.position)) {
                connection->rx.flow = CANTP_FS_CTS; /* SWS_CanTp_00224 */
                connection->rx.state = CANTP_RX_FLOW_SEND;
                CanTp_StartTimer(&connection->rx.timer, connection->config->n_ar);
            } else {
                connection->rx.state = CANTP_RX_BUFFER; /* N_Br keeps running (SWS_CanTp_00082). */
            }
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    return error;
}
/* Starts a reception from a valid SF or FF (SWS_CanTp_00079, 00081, 00318, 00339, 00353). */
static CANTP_CODE uint8 CanTp_Admit(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    uint8 error = 0u;
    boolean copy = FALSE;
    const boolean first = ((work->frame[0] / 16u) == CANTP_PCI_FF);
    const CanTp_StartType start = CanTp_DecodeStart(work->frame, (uint8)work->length);
    const PduLengthType length = (PduLengthType)start.total;
    const PduLengthType bytes = first ? (work->length - start.pci) : length;
    const PduInfoType payload = {&work->frame[start.pci], NULL_PTR, bytes};
    PduLengthType available = 0u;
    const BufReq_ReturnType result =
        PduR_CanTpStartOfReception(work->id, &payload, length, &available);
    SchM_Enter_CanTp_CANTP_STATE();
    if ((work->generation == connection->rx.generation) &&
        (connection->rx.state == CANTP_RX_START)) {
        if ((result == BUFREQ_OK) && (available < payload.SduLength)) {
            CanTp_RxFinish(connection, E_NOT_OK);
            error = CANTP_E_COM;
        } else if (result == BUFREQ_OK) {
            copy = (boolean)TRUE;
        } else if ((result == BUFREQ_E_OVFL) && (first == TRUE)) {
            connection->rx.flow = CANTP_FS_OVFLW;
            connection->rx.state = CANTP_RX_FLOW_SEND;
            CanTp_StartTimer(&connection->rx.timer, connection->config->n_ar);
        } else {
            CanTp_RxDrop(connection);
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    if (copy == TRUE) {
        error = CanTp_AdmitData(connection, work, &payload);
    }
    return error;
}
/* A received CF of the reception (SWS_CanTp_00269, 00271, 00314, 00346). */
static CANTP_CODE uint8 CanTp_Consecutive(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    uint8 error = 0u;
    boolean copy = FALSE;
    PduInfoType payload = {&work->frame[1], NULL_PTR, 0u};
    SchM_Enter_CanTp_CANTP_STATE();
    if ((connection->rx.state == CANTP_RX_DATA) &&
        (work->generation == connection->rx.generation)) {
        const uint8 capacity = connection->rx.rx_dl - 1u;
        const PduLengthType remaining = connection->rx.total - connection->rx.position;
        payload.SduLength = (remaining > capacity) ? capacity : remaining;
        if (work->length < CanTp_WireLength(1u + (uint8)payload.SduLength)) {
            CanTp_RxFinish(connection, E_NOT_OK);
            error = CANTP_E_PADDING;
        } else if ((work->length > connection->rx.rx_dl) ||
                   ((work->frame[0] & 0x0fu) != connection->rx.sequence)) {
            CanTp_RxFinish(connection, E_NOT_OK);
            error = CANTP_E_COM;
        } else {
            work->id = connection->config->upper_receive;
            copy = (boolean)TRUE;
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    if (copy == TRUE) {
        PduLengthType available = 0u;
        const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, &payload, &available);
        SchM_Enter_CanTp_CANTP_STATE();
        if ((work->generation == connection->rx.generation) &&
            (connection->rx.state == CANTP_RX_DATA)) {
            if (result != BUFREQ_OK) {
                CanTp_RxFinish(connection, E_NOT_OK);
                error = CANTP_E_COM;
            } else {
                connection->rx.position += payload.SduLength;
                connection->rx.sequence = (uint8)((connection->rx.sequence + 1u) % 16u);
                CanTp_StartTimer(&connection->rx.timer, connection->config->n_cr);
                if (connection->rx.position == connection->rx.total) {
                    CanTp_RxFinish(connection, E_OK);
                }
            }
        }
        SchM_Exit_CanTp_CANTP_STATE();
    }
    return error;
}
/* A received SF, FF or CF (SWS_CanTp_00057 table 1, 00124, 00345). */
static CANTP_CODE uint8 CanTp_Frame(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    uint8 error = 0u;
    const uint8 kind = work->frame[0] / 16u;
    if (kind == CANTP_PCI_CF) {
        error = CanTp_Consecutive(connection, work);
    } else {
        const CanTp_StartType start = CanTp_DecodeStart(work->frame, (uint8)work->length);
        boolean admit = FALSE;
        SchM_Enter_CanTp_CANTP_STATE();
        if (work->generation != connection->rx.generation) {
            /* Received before Init or Shutdown lost the connections. */
        } else if ((kind == CANTP_PCI_SF) && (work->length < CANTP_MIN_PADDED_DL)) {
            error = CANTP_E_PADDING; /* Rejected; a reception in progress continues. */
        } else if ((start.total != 0u) && (connection->rx.state == CANTP_RX_IDLE)) {
            const uint32 total = start.total;
            const PduLengthType sdu_length = (PduLengthType)total;
            if ((uint32)sdu_length != total) {
                /* The upper API cannot represent this wire length (SWS_CanTp_00352).
                 * Do not admit a truncated N-SDU or fabricate an upper overflow result. */
                error = CANTP_E_COM;
            } else {
                connection->rx.total = sdu_length;
                connection->rx.rx_dl = (work->length <= CANTP_MIN_PADDED_DL)
                                           ? CANTP_MIN_PADDED_DL
                                           : (uint8)work->length; /* SWS_CanTp_00350 */
                connection->rx.position = 0u;
                connection->rx.sequence = 1u;
                connection->rx.waits = 0u;
                connection->rx.due = FALSE;
                connection->rx.flow = CANTP_FS_CTS;
                CanTp_StartTimer(
                    &connection->rx.timer,
                    connection->config->n_br); /* Before the upper call (SWS_CanTp_00166). */
                connection->rx.state = CANTP_RX_START;
                ++connection->rx.generation;
                work->generation = connection->rx.generation;
                work->id = connection->config->upper_receive;
                admit = (boolean)TRUE;
            }
        } else {
            /* Malformed SF or FF: ignored (ISO 15765-2). */
        }
        SchM_Exit_CanTp_CANTP_STATE();
        if (admit == TRUE) {
            error = CanTp_Admit(connection, work);
        }
    }
    return error;
}
static CANTP_CODE void CanTp_Perform(CanTp_ConnectionType *connection, CanTp_WorkType *work)
{
    if (work->kind == CANTP_WORK_TX_DONE) {
        PduR_CanTpTxConfirmation(work->id, work->result);
    } else if (work->kind == CANTP_WORK_RX_DONE) {
        PduR_CanTpRxIndication(work->id, work->result);
    } else if (work->kind == CANTP_WORK_FRAME) {
        CanTp_Report(CANTP_SID_RX_INDICATION, CanTp_Frame(connection, work));
    } else if (work->kind == CANTP_WORK_COPY_TX) {
        /* No retry information: ISO 15765-2 has no recovery (SWS_CanTp_00272). */
        PduLengthType available = 0u;
        const PduInfoType data = {&connection->tx.frame[work->offset], NULL_PTR, work->length};
        CanTp_Copied(connection, work, PduR_CanTpCopyTxData(work->id, &data, NULL_PTR, &available));
    } else if (work->kind == CANTP_WORK_LOWER) {
        const PduInfoType frame = {work->frame, NULL_PTR, work->length};
        if (LSduR_CanTpTransmit(work->id, &frame) != E_OK) {
            CanTp_Refused(connection, work);
        }
    } else {
        PduLengthType available = 0u;
        const PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
        const BufReq_ReturnType result = PduR_CanTpCopyRxData(work->id, &query, &available);
        CanTp_Report(CANTP_SID_MAIN_FUNCTION, CanTp_Polled(connection, work, result, available));
    }
}
/* Selects work under CANTP_STATE and releases the runner when nothing is owed. */
static CANTP_CODE void CanTp_Next(CanTp_WorkType *work, boolean send)
{
    work->kind = CANTP_WORK_NONE;
    for (uint32 i = 0u; (i < CANTP_CONNECTION_CAPACITY) && (i < connection_count) &&
                        (work->kind == CANTP_WORK_NONE);
         ++i) {
        work->connection = i;
        CanTp_NextConnection(&connections[i], work, send);
    }
    if (work->kind == CANTP_WORK_NONE) {
        cantp_running = FALSE;
    }
}
/* Makes the calls owed so far unless another context is making them; that
 * context continues until nothing is left, so no recorded event is lost. Without `send`,
 * frames wait for the next run that allows them, at the latest the main function. */
static CANTP_CODE void CanTp_Run(boolean send)
{
    boolean more = FALSE;
    SchM_Enter_CanTp_CANTP_STATE();
    if (cantp_running == FALSE) {
        cantp_running = TRUE;
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
            CanTp_Perform(&connections[work.connection], &work);
        }
    }
}
static CANTP_CODE boolean CanTp_ValidConnection(const CanTp_ConnectionConfigType *config)
{
    return (config->maximum_length >= CANTP_MIN_PADDED_DL) &&
           ((PduLengthType)config->maximum_length == config->maximum_length) &&
           (config->n_as >= CANTP_TIMER_MIN) && (config->n_ar >= CANTP_TIMER_MIN) &&
           (config->n_bs >= CANTP_TIMER_MIN) && (config->n_br >= CANTP_TIMER_MIN) &&
           (config->n_cr >= CANTP_TIMER_MIN) && (config->n_cs >= CANTP_TIMER_MIN) &&
           (config->main_period_ms > 0u) && (config->tx_dl >= 8u) && (config->rx_max_dl >= 8u) &&
           Can_IsWireLength(CAN_ID_FD_FLAG, config->tx_dl) &&
           Can_IsWireLength(CAN_ID_FD_FLAG, config->rx_max_dl);
}
static CANTP_CODE void CanTp_ResetAll(void)
{
    for (uint32 i = 0u; i < CANTP_CONNECTION_CAPACITY; ++i) {
        CanTp_Reset(&connections[i]);
    }
    receive_order = CANTP_RECEIVE_AFTER_MAIN;
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE void CanTp_Init(const CanTp_ConfigType *CfgPtr)
{
    SchM_Enter_CanTp_CANTP_STATE();
    if ((CfgPtr != NULL_PTR) && (CfgPtr->connections != NULL_PTR) &&
        (CfgPtr->connection_count <= CANTP_CONNECTION_CAPACITY)) {
        boolean valid = (boolean)TRUE;
        for (uint32 i = 0u;
             (i < CANTP_CONNECTION_CAPACITY) && (i < CfgPtr->connection_count) && valid;
             ++i) {
            valid = CanTp_ValidConnection(&CfgPtr->connections[i]);
        }
        if (valid) {
            CanTp_ResetAll(); /* SWS_CanTp_00273: silently drop every connection. */
            connection_count = CfgPtr->connection_count;
            for (uint32 i = 0u; (i < CANTP_CONNECTION_CAPACITY) && (i < connection_count); ++i) {
                connections[i].config = &CfgPtr->connections[i];
            }
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE void CanTp_Shutdown(void)
{
    SchM_Enter_CanTp_CANTP_STATE();
    connection_count = 0u;
    CanTp_ResetAll(); /* No notification (SWS_CanTp_00200). */
    SchM_Exit_CanTp_CANTP_STATE();
}
typedef enum { CANTP_TX_SDU, CANTP_RX_SDU, CANTP_RX_PDU } CanTp_IdDomainType;
static CANTP_CODE CanTp_ConnectionType *CanTp_Find(PduIdType id, CanTp_IdDomainType domain)
{
    CanTp_ConnectionType *connection = NULL_PTR;
    for (uint32 i = 0u;
         (i < CANTP_CONNECTION_CAPACITY) && (i < connection_count) && (connection == NULL_PTR);
         ++i) {
        const CanTp_ConnectionConfigType *config = connections[i].config;
        PduIdType mapped = config->lower_receive;
        if (domain == CANTP_TX_SDU) {
            mapped = config->transmit;
        } else if (domain == CANTP_RX_SDU) {
            mapped = config->receive;
        } else {
            /* Lower Rx N-PDU. */
        }
        if (mapped == id) {
            connection = &connections[i];
        }
    }
    return connection;
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE Std_ReturnType CanTp_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanTp_CANTP_STATE();
    /* Only the length is used (SWS_CanTp_00225, 00299); a busy channel refuses (00123). */
    CanTp_ConnectionType *const connection = CanTp_Find(TxPduId, CANTP_TX_SDU);
    if ((connection != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->MetaDataPtr == NULL_PTR) && (PduInfoPtr->SduLength > 0u) &&
        (PduInfoPtr->SduLength <= connection->config->maximum_length) &&
        (connection->tx.state == CANTP_TX_IDLE)) {
        connection->tx.total = PduInfoPtr->SduLength;
        connection->tx.position = 0u;
        connection->tx.sequence = 1u;
        connection->tx.block_size = 0u;
        connection->tx.block_sent = 0u;
        connection->tx.separation = 0u;
        connection->tx.delay = 0u;
        connection->tx.due = TRUE;
        CanTp_StartTimer(&connection->tx.timer,
                         connection->config->n_cs); /* Before the first copy (SWS_CanTp_00167). */
        connection->tx.state = CANTP_TX_COPY;
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
    known = (CanTp_Find(TxPduId, CANTP_TX_SDU) != NULL_PTR);
    SchM_Exit_CanTp_CANTP_STATE();
    /* CanTpTc is false: no transmission of this N-SDU can be cancelled (SWS_CanTp_00242). */
    if (known == TRUE) {
        CanTp_Report(CANTP_SID_CANCEL_TRANSMIT, CANTP_E_OPER_NOT_SUPP);
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
    CanTp_ConnectionType *const connection = CanTp_Find(RxPduId, CANTP_RX_SDU);
    if (connection != NULL_PTR) {
        if ((CanTp_Receiving(connection) == TRUE) && (connection->rx.flow != CANTP_FS_OVFLW)) {
            /* Refused while the last CF is awaited (SWS_CanTp_00262). The FC in flight
             * keeps the N-PDU until its own confirmation (SWS_CanTp_00075). */
            if ((connection->rx.state != CANTP_RX_DATA) ||
                (((uint32)connection->rx.total - (uint32)connection->rx.position) >
                 ((uint32)connection->rx.rx_dl - 1u))) {
                CanTp_RxFinish(connection, E_NOT_OK);
                result = E_OK;
            }
        } else if (connection->rx.state != CANTP_RX_START) {
            error = CANTP_E_OPER_NOT_SUPP;
        } else {
            /* An SF or FF is being admitted: not cancellable yet. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    CanTp_Report(CANTP_SID_CANCEL_RECEIVE, error);
    if (result == E_OK) {
        CanTp_Run(TRUE);
    }
    return result;
}
CANTP_CODE void Ecu_CanTpReceptionBeforeMain(boolean pending)
{
    SchM_Enter_CanTp_CANTP_STATE();
    receive_order = pending ? CANTP_RECEIVE_BEFORE_MAIN : CANTP_RECEIVE_AFTER_MAIN;
    SchM_Exit_CanTp_CANTP_STATE();
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE void CanTp_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    uint8 error = 0u;
    SchM_Enter_CanTp_CANTP_STATE();
    CanTp_ConnectionType *const connection = CanTp_Find(RxPduId, CANTP_RX_PDU);
    if ((connection != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) && (PduInfoPtr->MetaDataPtr == NULL_PTR) &&
        (PduInfoPtr->SduLength > 0u) && (PduInfoPtr->SduLength <= connection->config->rx_max_dl) &&
        Can_IsWireLength(CAN_ID_FD_FLAG, PduInfoPtr->SduLength)) {
        const uint8 length = (uint8)PduInfoPtr->SduLength;
        const uint8 kind = PduInfoPtr->SduDataPtr[0] / 16u;
        if (kind == CANTP_PCI_FC) {
            error = CanTp_ReceiveFlow(connection, PduInfoPtr->SduDataPtr, length);
        } else if ((kind <= CANTP_PCI_CF) && (connection->queue_count < CANTP_RX_QUEUE)) {
            const uint8 slot =
                (uint8)((connection->queue_head + connection->queue_count) % CANTP_RX_QUEUE);
            for (uint8 i = 0u; i < length; ++i) {
                connection->queued[slot][i] = PduInfoPtr->SduDataPtr[i];
            }
            connection->queued_length[slot] = length;
            ++connection->queue_count;
        } else if (kind <= CANTP_PCI_CF) {
            error = CANTP_E_COM; /* Arrived faster than the calls it needs could be made. */
        } else {
            /* Unknown N-PDU type: ignored (SWS_CanTp_00057). */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    CanTp_Report(CANTP_SID_RX_INDICATION, error);
    CanTp_Run(TRUE);
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE void CanTp_TxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    SchM_Enter_CanTp_CANTP_STATE();
    CanTp_ConnectionType *const connection = CanTp_Pending(TxPduId);
    if (connection != NULL_PTR) {
        const CanTp_LowerType owner = connection->lower_owner;
        connection->lower_owner = CANTP_LOWER_FREE;
        if ((owner == CANTP_LOWER_TX) &&
            (connection->lower_generation == connection->tx.generation) &&
            (connection->tx.state == CANTP_TX_CONFIRM)) {
            CanTp_TxConfirmed(connection, result);
        } else if ((owner == CANTP_LOWER_RX) &&
                   (connection->lower_generation == connection->rx.generation) &&
                   (connection->rx.state == CANTP_RX_FLOW_CONFIRM)) {
            CanTp_FlowConfirmed(connection, result);
        } else {
            /* Late confirmation after a timeout, Init or Shutdown: only frees the N-PDU. */
        }
    }
    SchM_Exit_CanTp_CANTP_STATE();
    /* The driver may hold its slot until this confirmation returns, so the next frame
     * waits for the main function, as STmin 0 allows. */
    CanTp_Run(FALSE);
}
/* MR12 RULE 8.7 VIOLATION: the no-diagnostic package retains this supported library service. */
CANTP_CODE void CanTp_MainFunction(void)
{
    for (uint32 i = 0u; i < CANTP_CONNECTION_CAPACITY; ++i) {
        uint8 tx_error = 0u;
        uint8 rx_error = 0u;
        SchM_Enter_CanTp_CANTP_STATE();
        if (i < connection_count) {
            tx_error = CanTp_TxTick(&connections[i]);
            rx_error = CanTp_RxTick(&connections[i]);
        }
        SchM_Exit_CanTp_CANTP_STATE();
        CanTp_Report(CANTP_SID_MAIN_FUNCTION, tx_error);
        CanTp_Report(CANTP_SID_MAIN_FUNCTION, rx_error);
    }
    CanTp_Run(TRUE);
}
#define CANTP_STOP_SEC_CODE
#include "CanTp_MemMap.h"
