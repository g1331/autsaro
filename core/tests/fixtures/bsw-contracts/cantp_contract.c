/** Independent R24-11 consumer of the delivered CanTp, built with the host's strict areas.
 * Every PduR, LSduR and Det stub re-enters CANTP_STATE, so a call made inside the area
 * aborts (ADR 0018). Expected frames follow ISO 15765-2 with padded 8-byte N-PDUs.
 * Verifies: SWS_CanTp_00027、00168、00170、00238、00030、00111、00273、00010、00202、00200
 * Verifies: SWS_CanTp_00177、00242、00274、00075、00076、00355、00078、00079、00329、00350
 * Verifies: SWS_CanTp_00166、00080、00081、00318、00353、00339、00082、00222、00341、00223
 * Verifies: SWS_CanTp_00311、00224、00269、00312、00313、00271、00314、00084、00277、00064
 * Verifies: SWS_CanTp_00067、00342、00225、00167、00086、00272、00087、00184、00280、00089
 * Verifies: SWS_CanTp_00310、00309、00317、00315、00316、00090、00343、00096、00122、00123
 * Verifies: SWS_CanTp_00124、00248、00116、00059、00344、00345、00346、00347、00348、00349
 * Verifies: SWS_CanTp_00351、00057、00229、00352、00231、00232、00354、00204、00205、00206
 * Verifies: SWS_CanTp_00298、00299、00256、00255、00261、00262、00263、00164、00235、00236
 * Verifies: SWS_CanTp_00008、00199、00213、00208、00211、00212、00246、00257、00214、00215
 * Verifies: SWS_CanTp_00216、00209
 */
#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
#include <assert.h>
#include <string.h>

enum { RX_NSDU = 4u, TX_NSDU = 6u, RX_NPDU = 2u, TX_NPDU = 3u, UPPER_RX = 21u, UPPER_TX = 23u };
enum { N_AS = 5u, N_AR = 5u, N_BR = 4u, N_BS = 10u, N_CR = 10u, N_CS = 10u, WFT_MAX = 2u };
enum { PAD = 0xccu, RX_INDICATION = 0x42u, MAIN = 0x06u, CANCEL_TX = 0x4au, CANCEL_RX = 0x4cu };
enum { E_PADDING = 0x70u, E_OPER_NOT_SUPPORTED = 0xa0u, E_COM = 0xb0u, E_RX_COM = 0xc0u };
enum { E_TX_COM = 0xd0u };

static const CanTp_ConfigType config = {.receive = RX_NSDU,
                                        .transmit = TX_NSDU,
                                        .lower_receive = RX_NPDU,
                                        .lower_transmit = TX_NPDU,
                                        .maximum_length = 256u,
                                        .n_as = N_AS,
                                        .n_ar = N_AR,
                                        .n_br = N_BR,
                                        .n_bs = N_BS,
                                        .n_cr = N_CR,
                                        .n_cs = N_CS,
                                        .main_period_ms = 1u,
                                        .wait_frame_max = WFT_MAX,
                                        .padding = PAD,
                                        .upper_receive = UPPER_RX,
                                        .upper_transmit = UPPER_TX};

static uint8 sent[32][8];
static unsigned sent_count;
static Std_ReturnType lower_result;
static uint8 message[64];
static PduLengthType tx_offset;
static BufReq_ReturnType copy_tx_result;
static unsigned copy_tx_calls;
static unsigned tx_done;
static Std_ReturnType tx_result;
static void (*after_tx_done)(void);
static BufReq_ReturnType start_result;
static unsigned starts;
static PduLengthType started_length;
static uint8 start_data[8];
static PduLengthType start_data_length;
/* Space the upper layer offers at admission, and what is left of it. */
static PduLengthType capacity;
static PduLengthType buffer_size;
static BufReq_ReturnType copy_rx_result;
static uint8 received[64];
static PduLengthType received_length;
static unsigned polls;
static unsigned rx_done;
static Std_ReturnType rx_result;
static unsigned det_count;
static uint8 det_api;
static uint8 det_error;

/* A held CANTP_STATE makes this abort on the host. */
static void outside(void)
{
    SchM_Enter_CanTp_CANTP_STATE();
    SchM_Exit_CanTp_CANTP_STATE();
}
Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    outside();
    assert(TxPduId == TX_NPDU && PduInfoPtr->SduLength == 8u);
    assert(PduInfoPtr->MetaDataPtr == NULL_PTR && sent_count < 32u);
    memcpy(sent[sent_count++], PduInfoPtr->SduDataPtr, 8u);
    return lower_result;
}
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available)
{
    outside();
    assert(id == UPPER_TX && retry == NULL_PTR && info->SduDataPtr != NULL_PTR);
    ++copy_tx_calls;
    if (copy_tx_result == BUFREQ_OK) {
        memcpy(info->SduDataPtr, &message[tx_offset], info->SduLength);
        tx_offset = (PduLengthType)(tx_offset + info->SduLength);
    }
    *available = 0u;
    return copy_tx_result;
}
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    outside();
    assert(id == UPPER_TX);
    tx_result = result;
    ++tx_done;
    if (after_tx_done != NULL_PTR) {
        after_tx_done();
    }
}
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType length,
                                             PduLengthType *available)
{
    outside();
    assert(id == UPPER_RX && info != NULL_PTR && info->MetaDataPtr == NULL_PTR);
    assert(info->SduLength <= 8u);
    ++starts;
    started_length = length;
    start_data_length = info->SduLength;
    memcpy(start_data, info->SduDataPtr, info->SduLength);
    received_length = 0u;
    buffer_size = capacity;
    *available = buffer_size;
    return start_result;
}
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available)
{
    outside();
    assert(id == UPPER_RX);
    if (info->SduLength == 0u) {
        assert(info->SduDataPtr == NULL_PTR);
        ++polls;
    } else if (copy_rx_result == BUFREQ_OK) {
        assert(info->SduLength <= buffer_size);
        memcpy(&received[received_length], info->SduDataPtr, info->SduLength);
        received_length = (PduLengthType)(received_length + info->SduLength);
        buffer_size = (PduLengthType)(buffer_size - info->SduLength);
    } else {
        /* Refused. */
    }
    *available = buffer_size;
    return copy_rx_result;
}
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    outside();
    assert(id == UPPER_RX);
    rx_result = result;
    ++rx_done;
}
Std_ReturnType Det_ReportRuntimeError(uint16_t ModuleId,
                                      uint8_t InstanceId,
                                      uint8_t ApiId,
                                      uint8_t ErrorId)
{
    outside();
    assert(ModuleId == 35u && InstanceId == 0u);
    det_api = ApiId;
    det_error = ErrorId;
    ++det_count;
    return E_OK;
}

static void start(void)
{
    CanTp_Init(&config);
    sent_count = 0u;
    lower_result = E_OK;
    copy_tx_result = BUFREQ_OK;
    copy_tx_calls = 0u;
    tx_done = 0u;
    after_tx_done = NULL_PTR;
    start_result = BUFREQ_OK;
    starts = 0u;
    capacity = 64u;
    copy_rx_result = BUFREQ_OK;
    polls = 0u;
    rx_done = 0u;
    det_count = 0u;
}
static void ticks(unsigned count)
{
    unsigned i;
    for (i = 0u; i < count; ++i) {
        CanTp_MainFunction();
    }
}
static void confirm(Std_ReturnType result) { CanTp_TxConfirmation(TX_NPDU, result); }
static Std_ReturnType transmit(PduLengthType length)
{
    /* Only the length is used (SWS_CanTp_00299). */
    const PduInfoType info = {NULL_PTR, NULL_PTR, length};
    tx_offset = 0u;
    return CanTp_Transmit(TX_NSDU, &info);
}
static void receive8(uint8 b0, uint8 b1, uint8 b2, uint8 b3)
{
    uint8 frame[8] = {0u, 0u, 0u, 0u, PAD, PAD, PAD, PAD};
    const PduInfoType info = {frame, NULL_PTR, 8u};
    frame[0] = b0;
    frame[1] = b1;
    frame[2] = b2;
    frame[3] = b3;
    CanTp_RxIndication(RX_NPDU, &info);
}
/* A CF or FF carrying message bytes from `first` (1-based values). */
static void receive_data(uint8 pci, uint8 dl, uint8 first, PduLengthType length)
{
    uint8 frame[8];
    const uint8 header = ((pci & 0xf0u) == 0x10u) ? 2u : 1u;
    const PduInfoType info = {frame, NULL_PTR, length};
    uint8 i;
    frame[0] = pci;
    frame[1] = dl;
    for (i = header; i < 8u; ++i) {
        frame[i] = (uint8)(first + i - header);
    }
    CanTp_RxIndication(RX_NPDU, &info);
}
static void expect_frame(unsigned index, uint8 pci, uint8 first, uint8 data, uint8 header)
{
    uint8 i;
    assert(index < sent_count && sent[index][0] == pci);
    for (i = header; i < 8u; ++i) {
        const uint8 expected = ((uint8)(i - header) < data) ? (uint8)(first + i - header) : PAD;
        assert(sent[index][i] == expected);
    }
}
static void expect_flow(unsigned index, uint8 status)
{
    static const uint8 padding[5] = {PAD, PAD, PAD, PAD, PAD};
    /* BS 0 and STmin 0 on every FC (SWS_CanTp_00067), padded (SWS_CanTp_00347). */
    assert(index < sent_count && sent[index][0] == (uint8)(0x30u | status));
    assert(sent[index][1] == 0u && sent[index][2] == 0u);
    assert(memcmp(&sent[index][3], padding, 5u) == 0);
}
static void expect_det(uint8 api, uint8 error)
{
    assert(det_count == 1u && det_api == api && det_error == error);
    det_count = 0u;
}

static void off_state(void)
{
    static const CanTp_ConfigType no_nbr = {.receive = RX_NSDU,
                                            .transmit = TX_NSDU,
                                            .lower_receive = RX_NPDU,
                                            .lower_transmit = TX_NPDU,
                                            .maximum_length = 256u,
                                            .n_as = N_AS,
                                            .n_ar = N_AR,
                                            .n_br = 0u,
                                            .n_bs = N_BS,
                                            .n_cr = N_CR,
                                            .n_cs = N_CS,
                                            .main_period_ms = 1u};
    assert(transmit(5u) == E_NOT_OK);
    receive8(0x03u, 1u, 2u, 3u);
    ticks(1u);
    assert(CanTp_CancelReceive(RX_NSDU) == E_NOT_OK);
    assert(CanTp_CancelTransmit(TX_NSDU) == E_NOT_OK);
    CanTp_Init(NULL_PTR);
    CanTp_Init(&no_nbr);
    assert(transmit(5u) == E_NOT_OK);
    assert(sent_count == 0u && starts == 0u && det_count == 0u);
}
static void single_frames(void)
{
    start();
    assert(transmit(5u) == E_OK && copy_tx_calls == 1u);
    expect_frame(0u, 0x05u, 1u, 5u, 1u);
    assert(transmit(5u) == E_NOT_OK);
    confirm(E_OK);
    assert(tx_done == 1u && tx_result == E_OK);

    receive8(0x03u, 0xa1u, 0xa2u, 0xa3u);
    assert(starts == 1u && started_length == 3u && start_data_length == 3u);
    assert(start_data[0] == 0xa1u && start_data[2] == 0xa3u);
    assert(received_length == 3u && received[2] == 0xa3u && rx_done == 1u && rx_result == E_OK);
    /* A short SF is rejected; an SF_DL of 0 or past 7 is not an SF on Classical CAN. */
    receive_data(0x03u, 0xa1u, 0xa2u, 4u);
    expect_det(RX_INDICATION, E_PADDING);
    receive8(0x00u, 1u, 2u, 3u);
    receive8(0x08u, 1u, 2u, 3u);
    assert(starts == 1u && rx_done == 1u);
    start_result = BUFREQ_E_NOT_OK;
    receive8(0x01u, 1u, 2u, 3u);
    start_result = BUFREQ_E_OVFL;
    receive8(0x01u, 1u, 2u, 3u);
    assert(starts == 3u && rx_done == 1u && sent_count == 1u);
    start_result = BUFREQ_OK;
    capacity = 2u;
    receive8(0x03u, 1u, 2u, 3u);
    assert(rx_done == 2u && rx_result == E_NOT_OK && received_length == 0u);
    expect_det(RX_INDICATION, E_COM);
}
static void segmented_transmission(void)
{
    start();
    assert(transmit(27u) == E_OK);
    expect_frame(0u, 0x10u, 1u, 6u, 2u);
    assert(sent[0][1] == 27u);
    confirm(E_OK);
    ticks(N_BS - 1u);
    receive8(0x31u, 0u, 0u, 0u);
    ticks(N_BS - 1u);
    assert(sent_count == 1u && tx_done == 0u);
    receive8(0x30u, 2u, 3u, 0u);
    expect_frame(1u, 0x21u, 7u, 7u, 1u);
    confirm(E_OK);
    /* STmin 3 ms at a 1 ms period: the fourth period after the confirmation sends. */
    ticks(3u);
    assert(sent_count == 2u);
    ticks(1u);
    expect_frame(2u, 0x22u, 14u, 7u, 1u);
    confirm(E_OK);
    ticks(N_BS - 1u);
    assert(sent_count == 3u && tx_done == 0u);
    receive8(0x30u, 0u, 0xf3u, 0u);
    expect_frame(3u, 0x23u, 21u, 7u, 1u);
    confirm(E_OK);
    assert(tx_done == 1u && tx_result == E_OK && copy_tx_calls == 4u && det_count == 0u);

    /* A reserved STmin (0xF0) separates CFs by 127 ms. */
    assert(transmit(20u) == E_OK);
    confirm(E_OK);
    receive8(0x30u, 0u, 0xf0u, 0u);
    expect_frame(5u, 0x21u, 7u, 7u, 1u);
    confirm(E_OK);
    ticks(127u);
    assert(sent_count == 6u);
    ticks(1u);
    expect_frame(6u, 0x22u, 14u, 7u, 1u);
    confirm(E_OK);
    assert(tx_done == 2u && tx_result == E_OK && det_count == 0u);
}
static void flow_control_failures(void)
{
    static const struct {
        uint8 pci;
        PduLengthType length;
        uint8 error;
    } cases[] = {{0x32u, 8u, E_COM}, {0x35u, 8u, E_COM}, {0x30u, 3u, E_PADDING}};
    unsigned i;
    for (i = 0u; i < 3u; ++i) {
        uint8 frame[8] = {0u, 0u, 0u, PAD, PAD, PAD, PAD, PAD};
        const PduInfoType info = {frame, NULL_PTR, cases[i].length};
        start();
        frame[0] = cases[i].pci;
        assert(transmit(20u) == E_OK);
        confirm(E_OK);
        CanTp_RxIndication(RX_NPDU, &info);
        assert(tx_done == 1u && tx_result == E_NOT_OK && sent_count == 1u);
        expect_det(RX_INDICATION, cases[i].error);
    }
    start();
    assert(transmit(20u) == E_OK);
    confirm(E_OK);
    ticks(N_BS - 1u);
    assert(tx_done == 0u);
    ticks(1u);
    assert(tx_done == 1u && tx_result == E_NOT_OK);
    expect_det(MAIN, E_TX_COM);
    receive8(0x30u, 0u, 0u, 0u);
    assert(sent_count == 1u);
}
static void copy_failures(void)
{
    start();
    copy_tx_result = BUFREQ_E_NOT_OK;
    assert(transmit(5u) == E_OK);
    assert(tx_done == 1u && tx_result == E_NOT_OK && sent_count == 0u);
    copy_tx_result = BUFREQ_E_BUSY;
    assert(transmit(5u) == E_OK);
    ticks(2u);
    assert(copy_tx_calls == 4u && sent_count == 0u);
    copy_tx_result = BUFREQ_OK;
    ticks(1u);
    expect_frame(0u, 0x05u, 1u, 5u, 1u);
    confirm(E_OK);
    assert(tx_done == 2u && tx_result == E_OK);
    /* N_Cs runs from the request (SWS_CanTp_00167): still busy at its end aborts. */
    copy_tx_result = BUFREQ_E_BUSY;
    assert(transmit(5u) == E_OK);
    ticks(N_CS - 1u);
    assert(tx_done == 2u);
    ticks(1u);
    assert(tx_done == 3u && tx_result == E_NOT_OK && copy_tx_calls == 5u + N_CS);
    expect_det(MAIN, E_TX_COM);
}
static void lower_failures(void)
{
    start();
    lower_result = E_NOT_OK;
    assert(transmit(5u) == E_OK);
    assert(tx_done == 1u && tx_result == E_NOT_OK);
    lower_result = E_OK;
    assert(transmit(5u) == E_OK && sent_count == 2u);
    confirm(E_NOT_OK);
    assert(tx_done == 2u && tx_result == E_NOT_OK);

    /* An unconfirmed N-PDU ends the N-SDU at N_As, once, and stays occupied. */
    assert(transmit(5u) == E_OK && sent_count == 3u);
    ticks(N_AS - 1u);
    assert(tx_done == 2u);
    ticks(1u);
    assert(tx_done == 3u && tx_result == E_NOT_OK);
    expect_det(MAIN, E_TX_COM);
    ticks(N_AS);
    assert(tx_done == 3u && det_count == 0u);
    assert(transmit(5u) == E_OK && sent_count == 3u);
    /* Its late confirmation only frees the N-PDU; the waiting frame follows at the next
     * main function. */
    confirm(E_OK);
    assert(sent_count == 3u);
    ticks(1u);
    assert(tx_done == 3u && sent_count == 4u);
    CanTp_TxConfirmation(TX_NPDU + 1u, E_OK);
    assert(tx_done == 3u);
    confirm(E_OK);
    assert(tx_done == 4u && tx_result == E_OK);
}
/* Starts a 20-byte reception whose FC(CTS) was confirmed. */
static void receive_first(void)
{
    receive_data(0x10u, 20u, 1u, 8u);
    assert(started_length == 20u && start_data_length == 6u && start_data[5] == 6u);
    expect_flow(sent_count - 1u, 0u);
    confirm(E_OK);
}
static void segmented_reception(void)
{
    start();
    receive_data(0x10u, 20u, 1u, 8u);
    assert(starts == 1u && received_length == 6u && sent_count == 1u);
    expect_flow(0u, 0u);
    /* A CF before the FC is confirmed is unexpected and ignored. */
    receive_data(0x21u, 0u, 7u, 8u);
    assert(received_length == 6u);
    confirm(E_OK);
    receive_data(0x21u, 0u, 7u, 8u);
    ticks(N_CR - 1u);
    receive_data(0x22u, 0u, 14u, 8u);
    assert(rx_done == 1u && rx_result == E_OK && received_length == 20u);
    assert(received[0] == 1u && received[19] == 20u && det_count == 0u);

    receive_first();
    receive_data(0x22u, 0u, 7u, 8u);
    assert(rx_done == 2u && rx_result == E_NOT_OK);
    expect_det(RX_INDICATION, E_COM);
    receive_first();
    ticks(N_CR - 1u);
    assert(rx_done == 2u);
    ticks(1u);
    assert(rx_done == 3u && rx_result == E_NOT_OK);
    expect_det(MAIN, E_RX_COM);
    receive_first();
    receive_data(0x21u, 0u, 7u, 8u);
    receive_data(0x22u, 0u, 14u, 7u);
    assert(rx_done == 4u && rx_result == E_NOT_OK);
    expect_det(RX_INDICATION, E_PADDING);
    receive_first();
    copy_rx_result = BUFREQ_E_NOT_OK;
    receive_data(0x21u, 0u, 7u, 8u);
    assert(rx_done == 5u && rx_result == E_NOT_OK);
    expect_det(RX_INDICATION, E_COM);
    copy_rx_result = BUFREQ_OK;
}
static void reception_admission(void)
{
    start();
    start_result = BUFREQ_E_OVFL;
    receive_data(0x10u, 20u, 1u, 8u);
    expect_flow(0u, 2u);
    confirm(E_OK);
    start_result = BUFREQ_E_NOT_OK;
    receive_data(0x10u, 20u, 1u, 8u);
    assert(rx_done == 0u && sent_count == 1u);
    start_result = BUFREQ_OK;
    capacity = 4u;
    receive_data(0x10u, 20u, 1u, 8u);
    assert(rx_done == 1u && rx_result == E_NOT_OK && sent_count == 1u);
    expect_det(RX_INDICATION, E_COM);
    /* An FF_DL that fits an SF is not an FF. */
    capacity = 64u;
    receive_data(0x10u, 7u, 1u, 8u);
    assert(starts == 3u);

    /* N_Ar: an FC left unconfirmed ends the reception; its confirmation comes late. */
    receive_data(0x10u, 20u, 1u, 8u);
    ticks(N_AR);
    assert(rx_done == 2u && rx_result == E_NOT_OK);
    expect_det(MAIN, E_RX_COM);
    confirm(E_OK);
    receive_data(0x21u, 0u, 7u, 8u);
    assert(rx_done == 2u);
    lower_result = E_NOT_OK;
    receive_data(0x10u, 20u, 1u, 8u);
    assert(rx_done == 3u && rx_result == E_NOT_OK);
    lower_result = E_OK;

    /* A new SF or FF ends the reception in progress and starts again. */
    receive_first();
    receive_data(0x21u, 0u, 7u, 8u);
    receive_data(0x10u, 20u, 1u, 8u);
    assert(rx_done == 4u && rx_result == E_NOT_OK && received_length == 6u);
    confirm(E_OK);
    receive8(0x02u, 0x51u, 0x52u, 0u);
    assert(rx_done == 6u && rx_result == E_OK && received_length == 2u);
    assert(rx_done == 6u && det_count == 0u);
}
static void buffer_waits(void)
{
    start();
    capacity = 10u;
    receive_data(0x10u, 20u, 1u, 8u);
    assert(received_length == 6u && sent_count == 0u);
    ticks(1u);
    assert(polls == 1u && sent_count == 0u);
    buffer_size = 14u;
    ticks(1u);
    assert(polls == 2u);
    expect_flow(0u, 0u);
    confirm(E_OK);
    receive_data(0x21u, 0u, 7u, 8u);
    receive_data(0x22u, 0u, 14u, 8u);
    assert(rx_done == 1u && rx_result == E_OK && received_length == 20u);

    /* N_Br expires: FC(WAIT) until WFTmax, then the reception ends. */
    capacity = 6u;
    receive_data(0x10u, 20u, 1u, 8u);
    ticks(N_BR - 1u);
    assert(polls == 2u + N_BR - 1u && sent_count == 1u);
    ticks(1u);
    expect_flow(1u, 1u);
    confirm(E_OK);
    ticks(N_BR);
    expect_flow(2u, 1u);
    confirm(E_OK);
    ticks(N_BR - 1u);
    assert(rx_done == 1u);
    ticks(1u);
    assert(rx_done == 2u && rx_result == E_NOT_OK && sent_count == 3u);
    expect_det(MAIN, E_RX_COM);
}
static void full_duplex(void)
{
    start();
    assert(transmit(20u) == E_OK);
    confirm(E_OK);
    receive8(0x01u, 0x61u, 0u, 0u);
    assert(rx_done == 1u && rx_result == E_OK && tx_done == 0u);
    receive_data(0x10u, 20u, 1u, 8u);
    expect_flow(1u, 0u);
    /* The FC holds the shared N-PDU: the CF waits for its confirmation. */
    receive8(0x30u, 0u, 0u, 0u);
    assert(sent_count == 2u && copy_tx_calls == 2u);
    confirm(E_OK);
    ticks(1u);
    expect_frame(2u, 0x21u, 7u, 7u, 1u);
    receive_data(0x21u, 0u, 7u, 8u);
    confirm(E_OK);
    assert(sent_count == 3u);
    ticks(1u);
    expect_frame(3u, 0x22u, 14u, 7u, 1u);
    receive_data(0x22u, 0u, 14u, 8u);
    confirm(E_OK);
    assert(rx_done == 2u && rx_result == E_OK && tx_done == 1u && tx_result == E_OK);
    assert(det_count == 0u);
}
static void cancellation(void)
{
    start();
    assert(CanTp_CancelTransmit(TX_NSDU + 1u) == E_NOT_OK && det_count == 0u);
    assert(transmit(5u) == E_OK);
    assert(CanTp_CancelTransmit(TX_NSDU) == E_NOT_OK);
    expect_det(CANCEL_TX, E_OPER_NOT_SUPPORTED);
    confirm(E_OK);
    assert(tx_done == 1u && tx_result == E_OK);

    assert(CanTp_CancelReceive(RX_NSDU + 1u) == E_NOT_OK && det_count == 0u);
    assert(CanTp_CancelReceive(RX_NSDU) == E_NOT_OK);
    expect_det(CANCEL_RX, E_OPER_NOT_SUPPORTED);
    receive_data(0x10u, 20u, 1u, 8u);
    assert(CanTp_CancelReceive(RX_NSDU) == E_OK);
    assert(rx_done == 1u && rx_result == E_NOT_OK);
    confirm(E_OK);
    receive_first();
    assert(CanTp_CancelReceive(RX_NSDU) == E_OK && rx_done == 2u);
    /* Only the last CF is awaited: cancellation is refused. */
    receive_data(0x10u, 13u, 1u, 8u);
    confirm(E_OK);
    assert(CanTp_CancelReceive(RX_NSDU) == E_NOT_OK && det_count == 0u);
    receive_data(0x21u, 0u, 7u, 8u);
    assert(rx_done == 3u && rx_result == E_OK && received_length == 13u);
}
static void stop_and_restart(void)
{
    start();
    assert(transmit(20u) == E_OK);
    receive_data(0x10u, 20u, 1u, 8u);
    /* Reinitialization loses both connections without notification. */
    CanTp_Init(&config);
    ticks(N_CR + N_AS);
    assert(tx_done == 0u && rx_done == 0u && det_count == 0u);
    /* The FF still holds the N-PDU until its own confirmation. */
    assert(transmit(5u) == E_OK && sent_count == 1u);
    confirm(E_OK);
    ticks(1u);
    assert(sent_count == 2u && tx_done == 0u);
    confirm(E_OK);
    assert(tx_done == 1u && tx_result == E_OK);
    receive_first();
    CanTp_Shutdown();
    assert(transmit(5u) == E_NOT_OK && CanTp_CancelReceive(RX_NSDU) == E_NOT_OK);
    receive_data(0x21u, 0u, 7u, 8u);
    ticks(N_CR);
    assert(rx_done == 0u && det_count == 0u);
}
static void request_next(void) { assert(transmit(3u) == E_OK); }
static void reentrant_requests(void)
{
    start();
    after_tx_done = request_next;
    assert(transmit(5u) == E_OK);
    confirm(E_OK);
    after_tx_done = NULL_PTR;
    assert(tx_done == 1u && sent_count == 1u);
    ticks(1u);
    assert(sent_count == 2u);
    expect_frame(1u, 0x03u, 1u, 3u, 1u);
    confirm(E_OK);
    assert(tx_done == 2u && tx_result == E_OK);
}

int main(void)
{
    void (*initialize)(const CanTp_ConfigType *) = CanTp_Init;
    void (*shutdown)(void) = CanTp_Shutdown;
    Std_ReturnType (*request)(PduIdType, const PduInfoType *) = CanTp_Transmit;
    Std_ReturnType (*cancel_tx)(PduIdType) = CanTp_CancelTransmit;
    Std_ReturnType (*cancel_rx)(PduIdType) = CanTp_CancelReceive;
    void (*indicate)(PduIdType, const PduInfoType *) = CanTp_RxIndication;
    void (*confirmation)(PduIdType, Std_ReturnType) = CanTp_TxConfirmation;
    void (*main_function)(void) = CanTp_MainFunction;
    unsigned i;
    /* The R24-11 signatures (SWS_CanTp_00208, 00211, 00212, 00246, 00257, 00214, 00215, 00213). */
    (void)initialize;
    (void)shutdown;
    (void)request;
    (void)cancel_tx;
    (void)cancel_rx;
    (void)indicate;
    (void)confirmation;
    (void)main_function;
    for (i = 0u; i < sizeof(message); ++i) {
        message[i] = (uint8)(i + 1u);
    }
    off_state();
    single_frames();
    segmented_transmission();
    flow_control_failures();
    copy_failures();
    lower_failures();
    segmented_reception();
    reception_admission();
    buffer_waits();
    full_duplex();
    cancellation();
    stop_and_restart();
    reentrant_requests();
    return 0;
}
