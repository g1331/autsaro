/** libFuzzer driver of the delivered CanTp through its public interface (R24-11).
 * The input is a configuration prefix and a sequence of operations: received N-PDUs of 1 to 8
 * bytes, Tx confirmations, main functions, transmit and cancel requests, CanTp_Init and
 * CanTp_Shutdown, and the answers PduR and LSduR give next. The stubs check the contract
 * a PduR or LSduR may rely on, and afterwards every connection must converge: with
 * cooperative layers and bounded main functions each session ends through its timeout, and
 * a new reception and transmission then succeed. A failed check prints its line and aborts.
 */
#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) ((condition) ? (void)0 : CanTpFuzz_Fail(__LINE__, #condition))

enum { RX_NSDU = 4u, TX_NSDU = 6u, RX_NPDU = 2u, TX_NPDU = 3u, UPPER_RX = 21u, UPPER_TX = 23u };
/* More main functions than any session can last: a transmission of 256 bytes sends 37 CFs,
 * each up to 129 periods apart for the largest STmin (127 ms) at a 1 ms period; timers last at
 * most 9 periods and a reception sends at most three FC(WAIT). */
enum { SETTLE = 6000u, FREE_LIMIT = 4096u };

/* Restored before each input, so that an input runs alone as it does when replayed. */
static const CanTp_ConfigType defaults = {RX_NSDU,
                                          TX_NSDU,
                                          RX_NPDU,
                                          TX_NPDU,
                                          256u,
                                          2u,
                                          2u,
                                          2u,
                                          2u,
                                          2u,
                                          2u,
                                          1u,
                                          0u,
                                          0xccu,
                                          UPPER_RX,
                                          UPPER_TX};
static CanTp_ConfigType config;
static const uint8 *input;
static size_t input_size;
static size_t input_at;
/* What the stubs answer next. */
static BufReq_ReturnType start_result;
static BufReq_ReturnType copy_rx_result;
static BufReq_ReturnType copy_tx_result;
static Std_ReturnType lower_result;
static boolean confirm_inside;
static Std_ReturnType confirm_inside_result;
/* Free bytes of the upper Rx buffer; a copy uses them, the upper layer frees more. */
static PduLengthType free_space;
/* Between CanTp_Init and CanTp_Shutdown; when off, CanTp calls nobody (SWS_CanTp_00200). */
static boolean running;
/* The reception PduR accepted and the transmission CanTp accepted, until their result. */
static boolean rx_open;
static boolean rx_cancelled;
static PduLengthType rx_length;
static PduLengthType rx_copied;
static uint8 *rx_buffer;
static boolean tx_open;
static PduLengthType tx_length;
static PduLengthType tx_copied;
/* A frame LSduR accepted and has not confirmed. */
static boolean frame_pending;
/* Data bytes of the last Tx copy, which fill the next SF or CF. */
static PduLengthType tx_copy_length;
/* An FF refused with BUFREQ_E_OVFL owes FC(OVFLW) (SWS_CanTp_00318) until it is sent, a new
 * SF or FF replaces it (00057 table 1), the module restarts, or N_Ar expires (00311): N_Ar
 * periods have passed with the Tx N-PDU busy in each before the last, as an idle period could
 * carry the FC. */
static boolean overflow_owed;
static unsigned overflow_starts;
static unsigned overflow_periods;
static boolean overflow_idle;
static unsigned frames_sent;
static unsigned rx_results;
static Std_ReturnType rx_last;
static unsigned tx_results;
static Std_ReturnType tx_last;

static void CanTpFuzz_Fail(int line, const char *condition)
{
    (void)fprintf(stderr, "cantp_receive.c:%d: check failed: %s\n", line, condition);
    abort();
}
static boolean take(uint8 *value)
{
    boolean available = FALSE;
    if (input_at < input_size) {
        *value = input[input_at];
        ++input_at;
        available = TRUE;
    }
    return available;
}
/* CanTp calls PduR, LSduR and Det outside its area (ADR 0018); a held area aborts here. */
static void outside(void)
{
    SchM_Enter_CanTp_CANTP_STATE();
    SchM_Exit_CanTp_CANTP_STATE();
}
static void close_reception(void)
{
    free(rx_buffer);
    rx_buffer = NULL_PTR;
    rx_open = FALSE;
    rx_cancelled = FALSE;
}

/* An SF or FF opens a reception (SWS_CanTp_00079); a reception in progress has ended with
 * its indication first (SWS_CanTp_00057 table 1, 00084). */
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType length,
                                             PduLengthType *available)
{
    outside();
    CHECK(running && id == UPPER_RX && !rx_open && length > 0u);
    CHECK(info != NULL_PTR && info->MetaDataPtr == NULL_PTR && info->SduLength <= 8u);
    CHECK(info->SduLength <= length && (info->SduLength == 0u || info->SduDataPtr != NULL_PTR));
    CHECK(available != NULL_PTR);
    overflow_owed = (start_result == BUFREQ_E_OVFL) && (length > 7u);
    overflow_periods = 0u;
    overflow_idle = FALSE;
    ++overflow_starts;
    if (start_result == BUFREQ_OK) {
        rx_open = TRUE;
        rx_length = length;
        rx_copied = 0u;
        rx_buffer = malloc(length);
        CHECK(rx_buffer != NULL_PTR);
        *available = free_space;
    }
    return start_result;
}
/* Only an accepted reception copies, never beyond its length or the free buffer last
 * reported: CTS waits until the next block fits (SWS_CanTp_00080, 00222, 00224). */
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available)
{
    outside();
    CHECK(running && id == UPPER_RX && rx_open && info != NULL_PTR && available != NULL_PTR);
    if (info->SduLength == 0u) {
        CHECK(info->SduDataPtr == NULL_PTR);
    } else {
        CHECK(info->SduDataPtr != NULL_PTR && info->SduLength <= free_space);
        CHECK(info->SduLength <= rx_length - rx_copied);
        if (copy_rx_result == BUFREQ_OK) {
            memcpy(&rx_buffer[rx_copied], info->SduDataPtr, info->SduLength);
            rx_copied = (PduLengthType)(rx_copied + info->SduLength);
            free_space = (PduLengthType)(free_space - info->SduLength);
        }
    }
    *available = free_space;
    return copy_rx_result;
}
/* Exactly one indication ends each accepted reception; E_OK only for the whole N-SDU
 * (SWS_CanTp_00084, 00081, 00339), E_NOT_OK after a successful cancellation (00263). */
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    outside();
    CHECK(running && id == UPPER_RX && rx_open);
    CHECK(result != E_OK || (rx_copied == rx_length && !rx_cancelled));
    rx_last = result;
    ++rx_results;
    close_reception();
}
/* Each segment's data, within the accepted N-SDU (SWS_CanTp_00086, 00299). */
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available)
{
    (void)retry;
    outside();
    CHECK(running && id == UPPER_TX && tx_open && info != NULL_PTR && available != NULL_PTR);
    CHECK(info->SduDataPtr != NULL_PTR && info->SduLength > 0u && info->SduLength <= 7u);
    CHECK(info->SduLength <= tx_length - tx_copied);
    if (copy_tx_result == BUFREQ_OK) {
        /* Writes the whole span CanTp offers, so a short frame buffer is reported. */
        memset(info->SduDataPtr, 0x5a, info->SduLength);
        tx_copied = (PduLengthType)(tx_copied + info->SduLength);
        tx_copy_length = info->SduLength;
    }
    *available = (PduLengthType)(tx_length - tx_copied);
    return copy_tx_result;
}
/* Exactly one confirmation ends each accepted transmission, E_OK only after all of its data
 * (SWS_CanTp_00090, 00204, 00205). */
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    outside();
    CHECK(running && id == UPPER_TX && tx_open);
    CHECK(result != E_OK || tx_copied == tx_length);
    tx_open = FALSE;
    tx_last = result;
    ++tx_results;
}
/* One frame at a time on the Tx N-PDU until its confirmation (SWS_CanTp_00075, 00248),
 * always eight bytes with the unused bytes of an SF, CF or FC set to the padding byte
 * (SWS_CanTp_00347, 00348). */
Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    const uint8 *frame;
    uint8 used = 8u;
    uint8 i;
    outside();
    CHECK(running && TxPduId == TX_NPDU && !frame_pending);
    CHECK(PduInfoPtr != NULL_PTR && PduInfoPtr->SduDataPtr != NULL_PTR);
    CHECK(PduInfoPtr->MetaDataPtr == NULL_PTR && PduInfoPtr->SduLength == 8u);
    ++frames_sent;
    frame = PduInfoPtr->SduDataPtr;
    if ((frame[0] >> 4u) == 0u) {
        used = (uint8)(1u + frame[0]);
        CHECK(used >= 2u && used <= 8u);
    } else if ((frame[0] >> 4u) == 2u) {
        used = (uint8)(1u + tx_copy_length);
    } else if ((frame[0] >> 4u) == 3u) {
        used = 3u;
        if (frame[0] == 0x32u) {
            overflow_owed = FALSE;
        }
    } else {
        CHECK((frame[0] >> 4u) == 1u);
    }
    for (i = used; i < 8u; ++i) {
        CHECK(frame[i] == config.padding);
    }
    if (lower_result == E_OK) {
        frame_pending = TRUE;
        if (confirm_inside) {
            frame_pending = FALSE;
            CanTp_TxConfirmation(TX_NPDU, confirm_inside_result);
        }
    }
    return lower_result;
}
Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId, uint8 InstanceId, uint8 ApiId, uint8 ErrorId)
{
    outside();
    CHECK(ModuleId == 35u && InstanceId == 0u && ErrorId != 0u);
    (void)ApiId;
    return E_OK;
}

/* Timers of 2 to 9 periods, any legal N-SDU limit, WFTmax, padding byte and period. */
static void configure(void)
{
    uint8 value = 0u;
    if (take(&value)) {
        config.maximum_length = (uint16)(8u + (value % 249u));
    }
    if (take(&value)) {
        config.n_as = 2u + (value & 7u);
        config.n_ar = 2u + (((uint32)value >> 4u) & 7u);
    }
    if (take(&value)) {
        config.n_br = 2u + (value & 7u);
        config.n_bs = 2u + (((uint32)value >> 4u) & 7u);
    }
    if (take(&value)) {
        config.n_cr = 2u + (value & 7u);
        config.n_cs = 2u + (((uint32)value >> 4u) & 7u);
    }
    if (take(&value)) {
        config.wait_frame_max = value & 3u;
        config.main_period_ms = (uint16)(1u + (((uint32)value >> 2u) & 3u));
    }
    if (take(&value)) {
        config.padding = value;
    }
}
static void restart(void)
{
    CanTp_Init(&config);
    running = TRUE;
    overflow_owed = FALSE;
    /* All connections are lost without notification; a frame in flight stays until its
     * confirmation (SWS_CanTp_00273, 00075). */
    close_reception();
    tx_open = FALSE;
}
static void cooperate(void)
{
    start_result = BUFREQ_OK;
    copy_rx_result = BUFREQ_OK;
    copy_tx_result = BUFREQ_OK;
    lower_result = E_OK;
    confirm_inside = FALSE;
    free_space = FREE_LIMIT;
}
static void receive(PduIdType id, const uint8 *bytes, PduLengthType length, boolean metadata)
{
    /* An exactly sized copy, so that reading past the frame is reported. */
    uint8 *frame = malloc(length);
    uint8 tag = 0u;
    const PduInfoType info = {frame, metadata ? &tag : NULL_PTR, length};
    CHECK(frame != NULL_PTR);
    memcpy(frame, bytes, length);
    CanTp_RxIndication(id, &info);
    free(frame);
}
/* A main function, counting the periods of an owed FC(OVFLW) from the one after its FF. */
static void tick(void)
{
    const boolean busy = frame_pending;
    const unsigned starts = overflow_starts;
    const unsigned sent = frames_sent;
    CanTp_MainFunction();
    if (overflow_owed && (starts == overflow_starts)) {
        ++overflow_periods;
        /* The period in which N_Ar elapses may end the reception before the FC goes. */
        overflow_idle = overflow_idle || ((overflow_periods < config.n_ar) &&
                                          !(busy || frame_pending || (sent != frames_sent)));
        if (!overflow_idle && (overflow_periods >= config.n_ar)) {
            overflow_owed = FALSE;
        }
    }
}
static void confirm(Std_ReturnType result)
{
    frame_pending = FALSE;
    CanTp_TxConfirmation(TX_NPDU, result);
}
/* A transmission is accepted exactly when the module runs, the length is legal and the
 * channel is free (SWS_CanTp_00123, 00212). */
static void transmit(PduLengthType length)
{
    const PduInfoType info = {NULL_PTR, NULL_PTR, length};
    const boolean legal = (running && length > 0u && length <= config.maximum_length);
    const boolean busy = tx_open;
    Std_ReturnType result;
    if (legal && !busy) {
        tx_open = TRUE;
        tx_length = length;
        tx_copied = 0u;
    }
    result = CanTp_Transmit(TX_NSDU, &info);
    CHECK((result == E_OK) == (legal && !busy));
    if (result != E_OK && legal && !busy) {
        tx_open = FALSE;
    }
}
/* Only a reception in progress is cancelled, and its indication reports E_NOT_OK, possibly
 * before the call returns (SWS_CanTp_00257, 00263). */
static void cancel(PduIdType id)
{
    const boolean open = rx_open;
    const unsigned before = rx_results;
    if (CanTp_CancelReceive(id) == E_OK) {
        CHECK(open && id == RX_NSDU);
        if (rx_open) {
            rx_cancelled = TRUE;
        } else {
            CHECK(rx_results == before + 1u && rx_last == E_NOT_OK);
        }
    }
}
static void behave(uint8 value)
{
    static const BufReq_ReturnType starts[4] = {
        BUFREQ_OK, BUFREQ_OK, BUFREQ_E_NOT_OK, BUFREQ_E_OVFL};
    static const BufReq_ReturnType copies[4] = {
        BUFREQ_OK, BUFREQ_OK, BUFREQ_E_BUSY, BUFREQ_E_NOT_OK};
    start_result = starts[value & 3u];
    copy_rx_result = ((value & 4u) != 0u) ? BUFREQ_E_NOT_OK : BUFREQ_OK;
    copy_tx_result = copies[(value >> 3u) & 3u];
    lower_result = ((value & 0x20u) != 0u) ? E_NOT_OK : E_OK;
    confirm_inside = ((value & 0x40u) != 0u) ? TRUE : FALSE;
    confirm_inside_result = ((value & 0x80u) != 0u) ? E_NOT_OK : E_OK;
}
static void operate(void)
{
    uint8 op = 0u;
    while (take(&op)) {
        uint8 value = 0u;
        uint8 frame[8];
        if (!take(&value)) {
            break;
        }
        switch (op % 11u) {
            case 0u:
            case 1u:
            case 2u: {
                const PduLengthType length = (PduLengthType)(1u + (value & 7u));
                PduLengthType i;
                for (i = 0u; i < length; ++i) {
                    if (!take(&frame[i])) {
                        return;
                    }
                }
                receive((value & 0x80u) != 0u ? (PduIdType)(RX_NPDU + 1u) : RX_NPDU,
                        frame,
                        length,
                        (value & 0x40u) != 0u);
                break;
            }
            case 3u: {
                uint8 i;
                for (i = 0u; i <= (value & 3u); ++i) {
                    tick();
                }
                break;
            }
            case 4u:
                if ((value & 2u) != 0u) {
                    CanTp_TxConfirmation(TX_NPDU + 1u, E_OK);
                } else {
                    confirm(((value & 1u) != 0u) ? E_NOT_OK : E_OK);
                }
                break;
            case 5u: {
                uint8 low = 0u;
                if (!take(&low)) {
                    return;
                }
                transmit((PduLengthType)((((uint32)value << 8u) | low) % 264u));
                break;
            }
            case 6u: {
                uint8 space = 0u;
                if (!take(&space)) {
                    return;
                }
                behave(value);
                /* A buffer already offered to a reception never shrinks. */
                if (!rx_open) {
                    free_space = space;
                }
                break;
            }
            case 7u:
                free_space = (PduLengthType)(free_space + value);
                if (free_space > FREE_LIMIT) {
                    free_space = FREE_LIMIT;
                }
                break;
            case 8u:
                if ((value & 1u) != 0u) {
                    (void)CanTp_CancelTransmit((value & 2u) != 0u ? TX_NSDU : RX_NSDU);
                } else {
                    cancel((value & 2u) != 0u ? RX_NSDU : TX_NSDU);
                }
                break;
            case 9u:
                restart();
                break;
            default:
                /* No notification about pending sessions (SWS_CanTp_00200). */
                CanTp_Shutdown();
                running = FALSE;
                overflow_owed = FALSE;
                close_reception();
                tx_open = FALSE;
                break;
        }
    }
}
/* Every session ends through its own completion or timeout (SWS_CanTp_00310, 00311,
 * 00313, 00316, 00280); then a new SF reception and an SF transmission succeed. */
static void converge(void)
{
    static const uint8 single[8] = {0x02u, 0x5au, 0xa5u, 0xccu, 0xccu, 0xccu, 0xccu, 0xccu};
    unsigned i;
    unsigned before;
    cooperate();
    if (!running) {
        restart();
    }
    for (i = 0u; (i < SETTLE) && (rx_open || tx_open || frame_pending || overflow_owed); ++i) {
        if (frame_pending) {
            confirm(E_OK);
        }
        tick();
    }
    CHECK(!rx_open && !tx_open && !overflow_owed);
    before = rx_results;
    receive(RX_NPDU, single, 8u, FALSE);
    CHECK(rx_results == before + 1u && rx_last == E_OK);
    before = tx_results;
    transmit(3u);
    for (i = 0u; (i < SETTLE) && (tx_results == before); ++i) {
        if (frame_pending) {
            confirm(E_OK);
        }
        tick();
    }
    CHECK(tx_results == before + 1u && tx_last == E_OK && !frame_pending);
}

int CanTpFuzz_TestOneInput(const uint8 *data, size_t size)
{
    input = data;
    input_size = size;
    input_at = 0u;
    config = defaults;
    configure();
    frame_pending = FALSE;
    cooperate();
    restart();
    operate();
    converge();
    return 0;
}
