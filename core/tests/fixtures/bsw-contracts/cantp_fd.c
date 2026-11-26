/** Byte-level FD and connection contracts through the public CanTp interfaces.
 * Verifies: SWS_CanTp_00248, SWS_CanTp_00314, SWS_CanTp_00349, SWS_CanTp_00350
 * Verifies: SWS_CanTp_00351, SWS_CanTp_00354
 */
#include "CanTp.h"
#include "Det.h"
#include "LSduR_CanTp.h"
#include "PduR_CanTp.h"
#include "SchM_CanTp.h"
#include <assert.h>
#include <string.h>

enum { PAD = 0xccu, A = 0u, B = 1u, SHARED = 2u };
static CanTp_ConnectionConfigType connections[] = {
    {10u, 20u, 30u, 40u, 4096u, 5u, 5u, 5u, 5u, 3u, 5u, 1u, 0u, PAD, 50u, 60u, 64u, 64u},
    {11u, 21u, 31u, 41u, 4096u, 5u, 5u, 5u, 5u, 3u, 5u, 1u, 0u, PAD, 51u, 61u, 64u, 64u},
    {12u, 22u, 32u, 40u, 4096u, 5u, 5u, 5u, 5u, 3u, 5u, 1u, 0u, PAD, 52u, 62u, 64u, 64u},
};
static const CanTp_ConfigType config = {connections, 3u};
static struct {
    PduLengthType tx_copied;
    PduLengthType rx_copied;
    PduLengthType rx_total;
    unsigned tx_done;
    unsigned rx_done;
    Std_ReturnType tx_result;
    Std_ReturnType rx_result;
} sessions[3];

static unsigned sends;
static PduIdType sent_id;
static PduLengthType sent_length;
static uint8 sent[64];
static uint8 error;

Std_ReturnType LSduR_CanTpTransmit(PduIdType id, const PduInfoType *info)
{
    assert((id == 40u || id == 41u) && info->SduLength <= sizeof(sent));
    sent_id = id;
    sent_length = info->SduLength;
    memcpy(sent, info->SduDataPtr, sent_length);
    ++sends;
    return E_OK;
}

BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available)
{
    const unsigned channel = id - 60u;
    assert(channel < 3u && retry == NULL_PTR);
    for (unsigned i = 0u; i < info->SduLength; ++i) {
        info->SduDataPtr[i] = (uint8)sessions[channel].tx_copied;
        ++sessions[channel].tx_copied;
    }
    *available = (PduLengthType)(4096u - sessions[channel].tx_copied);
    return BUFREQ_OK;
}

void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    const unsigned channel = id - 60u;
    assert(channel < 3u);
    ++sessions[channel].tx_done;
    sessions[channel].tx_result = result;
}

BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType total,
                                             PduLengthType *available)
{
    const unsigned channel = id - 50u;
    assert(channel < 3u && info->SduLength <= total);
    sessions[channel].rx_total = total;
    sessions[channel].rx_copied = 0u;
    *available = total;
    return BUFREQ_OK;
}

BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available)
{
    const unsigned channel = id - 50u;
    assert(channel < 3u);
    for (unsigned i = 0u; i < info->SduLength; ++i) {
        assert(info->SduDataPtr[i] == (uint8)sessions[channel].rx_copied);
        ++sessions[channel].rx_copied;
    }
    *available = sessions[channel].rx_total - sessions[channel].rx_copied;
    return BUFREQ_OK;
}

void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    const unsigned channel = id - 50u;
    assert(channel < 3u);
    ++sessions[channel].rx_done;
    sessions[channel].rx_result = result;
}

Std_ReturnType Det_ReportRuntimeError(uint16 module, uint8 instance, uint8 api, uint8 id)
{
    assert(module == CANTP_MODULE_ID && instance == 0u);
    (void)api;
    error = id;
    return E_OK;
}

static void start(unsigned channel, PduLengthType length)
{
    const PduInfoType info = {NULL_PTR, NULL_PTR, length};
    sessions[channel].tx_copied = 0u;
    assert(CanTp_Transmit(connections[channel].transmit, &info) == E_OK);
}

static void receive(unsigned channel, uint8 *frame, PduLengthType length)
{
    const PduInfoType info = {frame, NULL_PTR, length};
    CanTp_RxIndication(connections[channel].lower_receive, &info);
}

static void payload(uint8 *frame, unsigned pci, unsigned count, unsigned pos, unsigned length)
{
    memset(&frame[pci], PAD, length - pci);
    for (unsigned i = 0u; i < count; ++i) {
        frame[pci + i] = (uint8)(pos + i);
    }
}

static void expect(uint8 *bytes, unsigned length, PduIdType id)
{
    assert(sent_id == id && sent_length == length);
    assert(memcmp(sent, bytes, length) == 0);
}

static void single_frames(void)
{
    /* Include the two escape PCI bytes when mapping the logical N-PDU to the wire DLC. */
    const unsigned logical[] = {13u, 33u, 49u};
    const unsigned wire[] = {16u, 48u, 64u};
    for (unsigned i = 0u; i < 3u; ++i) {
        uint8 frame[64] = {0u, (uint8)(logical[i] - 2u)};
        payload(frame, 2u, frame[1], 0u, wire[i]);
        start(A, frame[1]);
        expect(frame, wire[i], 40u);
        CanTp_TxConfirmation(40u, E_OK);
        receive(A, frame, wire[i]);
        assert(sessions[A].rx_result == E_OK && sessions[A].rx_copied == frame[1]);
    }
    uint8 frame[65] = {0u, 8u};
    payload(frame, 2u, 8u, 0u, 12u);
    start(A, 8u);
    expect(frame, 12u, 40u);
    assert(sessions[A].tx_done == 3u);
    CanTp_TxConfirmation(41u, E_OK);
    assert(sessions[A].tx_done == 3u);
    CanTp_TxConfirmation(40u, E_OK);
    receive(A, frame, sizeof(frame));
    assert(sessions[A].rx_done == 3u);
    receive(A, frame, 12u);
    assert(sessions[A].rx_done == 4u && sessions[A].rx_copied == 8u);
}

static void segmented_tx(void)
{
    uint8 flow[8] = {0x30u, 0u, 0u, PAD, PAD, PAD, PAD, PAD};
    uint8 first[64] = {0x10u, 70u};
    uint8 last[12] = {0x21u}; /* 1 PCI + 8 data bytes occupy a 12-byte CAN_DL. */
    payload(first, 2u, 62u, 0u, 64u);
    payload(last, 1u, 8u, 62u, 12u);
    start(A, 70u);
    expect(first, 64u, 40u);
    CanTp_TxConfirmation(41u, E_OK);
    const unsigned before = sends;
    receive(A, flow, 8u);
    assert(sends == before); /* Wrong channel confirmation did not open N_Bs. */
    CanTp_TxConfirmation(40u, E_OK);
    receive(B, flow, 8u);
    assert(sends == before); /* Wrong channel CTS cannot start a CF. */
    receive(A, flow, 8u);
    expect(last, 12u, 40u);
    CanTp_TxConfirmation(40u, E_OK);
    assert(sessions[A].tx_copied == 70u && sessions[A].tx_result == E_OK);

    start(A, 4096u);
    uint8 escape[64] = {0x10u, 0u, 0u, 0u, 0x10u, 0u};
    payload(escape, 6u, 58u, 0u, 64u);
    expect(escape, 64u, 40u);
    CanTp_TxConfirmation(40u, E_OK);
    uint8 cf[64] = {0x21u};
    payload(cf, 1u, 63u, 58u, 64u);
    receive(A, flow, 8u);
    expect(cf, 64u, 40u);
    CanTp_TxConfirmation(40u, E_NOT_OK);

    start(A, 70u);
    CanTp_TxConfirmation(40u, E_OK);
    receive(A, flow, 3u);
    assert(error == 0x70u && sessions[A].tx_result == E_NOT_OK);
    start(A, 70u);
    CanTp_TxConfirmation(40u, E_OK);
    flow[0] = 0x33u;
    receive(A, flow, 8u);
    assert(error == 0xb0u && sessions[A].tx_result == E_NOT_OK);
}

static void segmented_rx(void)
{
    uint8 first[16] = {0x10u, 32u};
    uint8 consecutive[16] = {0x21u};
    uint8 last[8] = {0x22u};
    uint8 flow[8] = {0x30u, 0u, 0u, PAD, PAD, PAD, PAD, PAD};
    payload(first, 2u, 14u, 0u, 16u);
    payload(consecutive, 1u, 15u, 14u, 16u);
    payload(last, 1u, 3u, 29u, 8u);
    receive(A, first, 16u);
    expect(flow, 8u, 40u);
    receive(B, first, 16u);
    expect(flow, 8u, 41u);
    CanTp_TxConfirmation(41u, E_OK);
    receive(A, consecutive, 16u);
    assert(sessions[A].rx_copied == 14u);
    CanTp_TxConfirmation(40u, E_OK);
    receive(B, consecutive, 16u);
    assert(sessions[A].rx_copied == 14u && sessions[B].rx_copied == 29u);
    receive(A, consecutive, 16u);
    receive(A, last, 8u);
    receive(B, last, 8u);
    assert(sessions[A].rx_result == E_OK && sessions[A].rx_copied == 32u);
    assert(sessions[B].rx_result == E_OK && sessions[B].rx_copied == 32u);

    receive(A, first, 16u);
    CanTp_TxConfirmation(40u, E_OK);
    receive(A, last, 8u); /* Nonfinal CF shorter than RX_DL. */
    assert(error == 0x70u && sessions[A].rx_result == E_NOT_OK);
    receive(A, first, 16u);
    CanTp_TxConfirmation(40u, E_OK);
    consecutive[0] = 0x22u;
    receive(A, consecutive, 16u);
    assert(error == 0xb0u && sessions[A].rx_result == E_NOT_OK);

    uint8 escape[64] = {0x10u, 0u, 0u, 0u, 0x10u, 0u};
    payload(escape, 6u, 58u, 0u, 64u);
    receive(A, escape, 64u);
    assert(sessions[A].rx_total == 4096u && sessions[A].rx_copied == 58u);
    CanTp_TxConfirmation(40u, E_OK);
    const unsigned done = sessions[A].rx_done;
    for (unsigned i = 0u; i < 3u; ++i) {
        CanTp_MainFunction();
    }
    assert(error == 0xc0u && sessions[A].rx_done == done + 1u);
    first[0] = 0x03u;
    payload(first, 1u, 3u, 0u, 8u);
    receive(A, first, 7u);
    assert(error == 0x70u && sessions[A].rx_done == done + 1u);
    receive(A, first, 8u);
    assert(sessions[A].rx_done == done + 2u && sessions[A].rx_result == E_OK);
}

static void shared_tx(void)
{
    const unsigned before = sends;
    const unsigned done = sessions[A].tx_done;
    start(A, 3u);
    start(SHARED, 4u);
    start(B, 5u);
    assert(sends == before + 2u && sent_id == 41u);
    CanTp_TxConfirmation(41u, E_OK);
    CanTp_MainFunction();
    assert(sends == before + 2u && sessions[A].tx_done == done && sessions[SHARED].tx_done == 0u);
    CanTp_TxConfirmation(40u, E_OK);
    assert(sessions[A].tx_done == done + 1u && sessions[SHARED].tx_done == 0u);
    CanTp_MainFunction();
    assert(sends == before + 3u && sent_id == 40u && sent[0] == 4u);
    CanTp_TxConfirmation(40u, E_OK);
    assert(sessions[SHARED].tx_done == 1u && sessions[SHARED].tx_result == E_OK);
}

int main(void)
{
    CanTp_Init(&config);
    single_frames();
    segmented_tx();
    segmented_rx();
    shared_tx();
    return 0;
}
