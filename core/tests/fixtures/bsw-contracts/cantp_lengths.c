/** Literal wire lengths and transfer bytes, through the public CanTp interfaces.
 * Verifies: SWS_CanTp_00057, SWS_CanTp_00079, SWS_CanTp_00318, SWS_CanTp_00269
 * Verifies: SWS_CanTp_00352, SWS_CanTp_00008, SWS_COMTYPE_00008
 */
#include "CanTp.h"
#include "SchM_CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "Det.h"
#include <assert.h>
#include <string.h>
#include <stdio.h>

static CanTp_ConfigType config = {
    1u, 2u, 3u, 4u, 64u, 5u, 5u, 10u, 10u, 10u, 10u, 1u, 2u, 0xccu, 5u, 6u};
static uint16 capacity;
static uint16 received;
static uint16 transmitted;
static uint16 wire_bytes;
static unsigned starts;
static unsigned rx_done;
static unsigned tx_done;
static unsigned errors;
static unsigned frames;
static PduLengthType started;
static Std_ReturnType rx_result;
static uint8 last_frame[8];
static uint8 received_bytes[256];
static uint8 wire[256];

Std_ReturnType LSduR_CanTpTransmit(PduIdType id, const PduInfoType *info)
{
    uint8 offset;
    uint16 count;
    assert(id == 4u && info->SduLength == 8u);
    memcpy(last_frame, info->SduDataPtr, 8u);
    ++frames;
    if ((last_frame[0] / 16u) != 3u) {
        offset = ((last_frame[0] / 16u) == 1u) ? 2u : 1u;
        if (offset == 2u) {
            assert((((uint16)(last_frame[0] & 15u) * 256u) + last_frame[1]) == capacity);
        }
        count = (uint16)(capacity - wire_bytes);
        if (count > (8u - offset)) {
            count = (uint16)(8u - offset);
        }
        memcpy(&wire[wire_bytes], &last_frame[offset], count);
        wire_bytes = (uint16)(wire_bytes + count);
    }
    return E_OK;
}
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType length,
                                             PduLengthType *available)
{
    assert(id == 5u && info->SduLength <= 7u);
    ++starts;
    started = length;
    received = 0u;
    if (length > capacity) {
        return BUFREQ_E_OVFL;
    }
    *available = (PduLengthType)capacity;
    return BUFREQ_OK;
}
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available)
{
    assert(id == 5u && info->SduLength <= (uint16)(capacity - received));
    if (info->SduLength > 0u) {
        memcpy(&received_bytes[received], info->SduDataPtr, info->SduLength);
        received = (uint16)(received + info->SduLength);
    }
    *available = (PduLengthType)(capacity - received);
    return BUFREQ_OK;
}
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    assert(id == 5u);
    ++rx_done;
    rx_result = result;
}
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available)
{
    PduLengthType i;
    assert(id == 6u && retry == NULL_PTR && info->SduLength <= (uint16)(capacity - transmitted));
    for (i = 0u; i < info->SduLength; ++i) {
        info->SduDataPtr[i] = (uint8)(transmitted + i);
    }
    transmitted = (uint16)(transmitted + info->SduLength);
    *available = (PduLengthType)(capacity - transmitted);
    return BUFREQ_OK;
}
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    assert(id == 6u && result == E_OK);
    ++tx_done;
}
Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId, uint8 InstanceId, uint8 ApiId, uint8 ErrorId)
{
    if (ModuleId != 35u || InstanceId != 0u || ApiId != 0x42u || ErrorId != 0xb0u) {
        (void)fprintf(stderr,
                      "Det module=%u instance=%u api=%u error=%u capacity=%u rx=%u tx=%u\n",
                      ModuleId,
                      InstanceId,
                      ApiId,
                      ErrorId,
                      capacity,
                      received,
                      transmitted);
    }
    assert(ModuleId == 35u && InstanceId == 0u && ApiId == 0x42u && ErrorId == 0xb0u);
    ++errors;
    return E_OK;
}
static void receive(uint8 *frame)
{
    const PduInfoType info = {frame, NULL_PTR, 8u};
    CanTp_RxIndication(3u, &info);
}
static void first(uint16 length)
{
    uint8 frame[8] = {0x10u, 0u, 0u, 1u, 2u, 3u, 4u, 5u};
    frame[0] = (uint8)(0x10u + (length / 256u));
    frame[1] = (uint8)(length % 256u);
    receive(frame);
}
static void transfer(uint16 length)
{
    uint16 position;
    uint8 sequence = 1u;
    unsigned before;
    uint8 flow[8] = {0x30u, 0u, 0u, 0xccu, 0xccu, 0xccu, 0xccu, 0xccu};
    const PduInfoType tx = {NULL_PTR, NULL_PTR, (PduLengthType)length};
    config.maximum_length = length;
    capacity = length;
    CanTp_Init(&config);
    before = rx_done;
    first(length);
    assert(started == length && received == 6u && last_frame[0] == 0x30u);
    /* Rejected Init must not reset an admitted receive. */
    {
        CanTp_ConfigType invalid = config;
        invalid.maximum_length = 257u;
        CanTp_Init(&invalid);
        invalid.maximum_length = 7u;
        CanTp_Init(&invalid);
        if (sizeof(PduLengthType) == 1u) {
            invalid.maximum_length = 256u;
            CanTp_Init(&invalid);
        }
    }
    CanTp_TxConfirmation(4u, E_OK);
    for (position = 6u; position < length;) {
        uint8 frame[8];
        uint8 i;
        frame[0] = (uint8)(0x20u + sequence);
        for (i = 1u; i < 8u; ++i) {
            frame[i] = (uint8)(position + i - 1u);
        }
        receive(frame);
        {
            uint16 remaining = (uint16)(length - position);
            position = (uint16)(position + ((remaining < 7u) ? remaining : 7u));
        }
        sequence = (uint8)((sequence + 1u) % 16u);
    }
    assert(received == length && rx_done == before + 1u && rx_result == E_OK);
    for (position = 0u; position < length; ++position) {
        assert(received_bytes[position] == (uint8)position);
    }
    transmitted = 0u;
    wire_bytes = 0u;
    before = tx_done;
    assert(CanTp_Transmit(2u, &tx) == E_OK);
    assert(CanTp_Transmit(2u, &tx) == E_NOT_OK);
    CanTp_TxConfirmation(4u, E_OK);
    {
        unsigned sent = frames;
        receive(flow);
        if (frames != sent) {
            CanTp_TxConfirmation(4u, E_OK);
        }
    }
    /* Deterministic main periods, no timeouts or sleeps: at most 37 data frames. */
    for (position = 0u; position < 40u && tx_done == before; ++position) {
        unsigned sent = frames;
        CanTp_MainFunction();
        if (frames != sent) {
            CanTp_TxConfirmation(4u, E_OK);
        }
    }
    assert(tx_done == before + 1u && transmitted == length && wire_bytes == length);
    for (position = 0u; position < length; ++position) {
        assert(wire[position] == (uint8)position);
    }
}
int main(void)
{
    uint16 lengths[] = {256u, 263u, 511u};
    unsigned i;
    transfer(8u);
    transfer(64u);
    transfer(255u);
    if (sizeof(PduLengthType) > 1u) {
        transfer(256u);
    }
    capacity = 64u;
    config.maximum_length = 64u;
    CanTp_Init(&config);
    first(65u);
    assert(started == 65u && last_frame[0] == 0x32u);
    CanTp_TxConfirmation(4u, E_OK);
    for (i = 0u; i < 3u; ++i) {
        unsigned before_starts;
        unsigned before_errors;
        unsigned before_done;
        unsigned before_frames;
        CanTp_Init(&config);
        first(64u);
        CanTp_TxConfirmation(4u, E_OK);
        before_starts = starts;
        before_errors = errors;
        before_done = rx_done;
        before_frames = frames;
        first(lengths[i]);
        assert(rx_done == before_done + 1u && rx_result == E_NOT_OK);
        if (sizeof(PduLengthType) == 1u) {
            assert(starts == before_starts && errors == before_errors + 1u);
            assert(frames == before_frames);
        } else {
            assert(starts == before_starts + 1u && started == lengths[i]);
            assert(errors == before_errors && last_frame[0] == 0x32u);
            CanTp_TxConfirmation(4u, E_OK);
        }
        /* Recovery after the refusal, with unchanged initialized configuration. */
        {
            uint8 single[8] = {1u, 0x99u, 0xccu, 0xccu, 0xccu, 0xccu, 0xccu, 0xccu};
            receive(single);
            assert(started == 1u && received == 1u && received_bytes[0] == 0x99u);
            assert(rx_done == before_done + 2u && rx_result == E_OK);
        }
    }
    CanTp_Shutdown();
    return 0;
}
