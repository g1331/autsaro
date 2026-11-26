/** @file Independent byte vectors for COM network integer packing. */
#include "Com.h"
#include "Com_Internal.h"
#include <assert.h>
#include <string.h>

static const Com_PduConfigType pdus[] = {{1u, FALSE, 4u, 0u, 0u, 1u},
                                         {2u, FALSE, 2u, 0u, 1u, 1u},
                                         {3u, FALSE, 2u, 0u, 2u, 1u},
                                         {4u, FALSE, 2u, 0xabu, 3u, 2u},
                                         {2u, TRUE, 2u, 0u, 5u, 1u},
                                         {3u, TRUE, 2u, 0u, 6u, 1u}};
static const Com_SignalConfigType signals[] = {{10u, COM_UINT32, 0u, 32u, FALSE, 0u, 0u, 0u},
                                               {20u, COM_SINT16, 5u, 10u, FALSE, 0u, 0u, 0u},
                                               {30u, COM_SINT16, 5u, 10u, TRUE, 0u, 0u, 0u},
                                               {40u, COM_SINT16, 5u, 10u, FALSE, 0u, 0u, 0u},
                                               {41u, COM_UINT8, 0u, 3u, FALSE, 4u, 0u, 0u},
                                               {50u, COM_SINT16, 5u, 10u, FALSE, 7u, 1u, 11u},
                                               {60u, COM_SINT16, 5u, 10u, TRUE, 0x3f9u, 3u, 22u}};
static unsigned received_le;
static unsigned received_be;
static unsigned timed_out_le;
static unsigned timed_out_be;

static void received(CbkHandleIdType handle)
{
    if (handle == 11u) {
        ++received_le;
    } else {
        assert(handle == 22u);
        ++received_be;
    }
}
static void timed_out(CbkHandleIdType handle)
{
    if (handle == 11u) {
        ++timed_out_le;
    } else {
        assert(handle == 22u);
        ++timed_out_be;
    }
}
static Std_ReturnType transmit(PduIdType id, const PduInfoType *info)
{
    (void)id;
    (void)info;
    assert(0);
    return E_NOT_OK;
}
static void rejected(PduIdType id, Std_ReturnType result)
{
    (void)id;
    (void)result;
    assert(0);
}
static void pack(PduIdType pdu, PduInfoType *info, const uint8 *expected, PduLengthType length)
{
    info->SduLength = 4u;
    assert(Com_TriggerTransmit(pdu, info) == E_OK);
    assert(info->SduLength == length);
    assert(memcmp(info->SduDataPtr, expected, length) == 0);
}
static void expect(Com_SignalIdType signal, uint8 status, sint16 expected)
{
    sint16 value = 1000;
    assert(Com_ReceiveSignal(signal, &value) == status);
    assert(value == expected);
}
int main(void)
{
    static const Com_ConfigType config = {
        pdus, 6u, signals, 7u, 0u, received, timed_out, transmit, rejected};
    static const uint8 u32_le[] = {0x78u, 0x56u, 0x34u, 0x12u};
    static const uint8 minus_three_le[] = {0xa0u, 0x7fu};
    static const uint8 minus_three_be[] = {0x3fu, 0xd0u};
    static const uint8 isolated[] = {0xacu, 0xffu};
    static const sint16 limits[] = {-512, -1, 0, 511};
    const uint32 baseline = 0x12345678u;
    const sint16 minus_three = -3;
    const sint16 too_low = -513;
    const sint16 too_high = 512;
    const uint8 too_wide = 8u;
    uint8 buffer[4];
    PduInfoType info = {buffer, NULL_PTR, 0u};

    Com_Init(&config);
    expect(50u, COM_SERVICE_NOT_AVAILABLE, 7);
    expect(60u, COM_SERVICE_NOT_AVAILABLE, -7);
    Com_IpduGroupStart(0u, TRUE);
    assert(Com_SendSignal(10u, &baseline) == E_OK);
    pack(1u, &info, u32_le, 4u);
    assert(Com_SendSignal(20u, &minus_three) == E_OK);
    assert(Com_SendSignal(30u, &minus_three) == E_OK);
    assert(Com_SendSignal(40u, &minus_three) == E_OK);
    assert(Com_SendSignal(41u, &too_wide) == E_NOT_OK);
    pack(4u, &info, isolated, 2u);

    assert(Com_SendSignal(20u, &too_low) == E_NOT_OK);
    assert(Com_SendSignal(30u, &too_high) == E_NOT_OK);
    pack(2u, &info, minus_three_le, 2u);
    Com_RxIndication(2u, &info);
    expect(50u, E_OK, -3);
    pack(3u, &info, minus_three_be, 2u);
    Com_RxIndication(3u, &info);
    expect(60u, E_OK, -3);

    Com_EnableReceptionDM(0u);
    for (unsigned index = 0u; index < sizeof(limits) / sizeof(limits[0]); ++index) {
        const sint16 sent = limits[index];
        assert(Com_SendSignal(20u, &sent) == E_OK);
        assert(Com_SendSignal(30u, &sent) == E_OK);
        info.SduLength = 4u;
        assert(Com_TriggerTransmit(2u, &info) == E_OK);
        Com_RxIndication(2u, &info);
        expect(50u, E_OK, sent);
        info.SduLength = 4u;
        assert(Com_TriggerTransmit(3u, &info) == E_OK);
        Com_RxIndication(3u, &info);
        expect(60u, E_OK, sent);
    }
    assert(received_le == 5u && received_be == 5u);
    Ecu_ComMainFunctionRx();
    assert(timed_out_le == 1u && timed_out_be == 0u);
    pack(2u, &info, (const uint8[]){0xe0u, 0x3fu}, 2u);
    Com_RxIndication(2u, &info);
    assert(received_le == 6u && received_be == 5u);
    expect(50u, E_OK, 511);
    expect(60u, E_OK, 511);
    return 0;
}
