/** Independent R24-11 Rx group/DM consumer; no lower-module stubs. */
#include "Com.h"
#include "Com_Internal.h"
#include <assert.h>
#include <stddef.h>

static unsigned acknowledgments[3];
static unsigned timeouts[3];
static unsigned handle_index(CbkHandleIdType handle) {
    unsigned index = 3u;
    if (handle == 17u) {
        index = 0u;
    } else if (handle == 43u) {
        index = 1u;
    } else if (handle == 59u) {
        index = 2u;
    }
    assert(index < 3u);
    return index;
}
static void receive(CbkHandleIdType handle) { ++acknowledgments[handle_index(handle)]; }
static void timeout(CbkHandleIdType handle) { ++timeouts[handle_index(handle)]; }
static const Com_PduConfigType pdus[] = {
    {0u, 10u, TRUE, 7u, 30u, 17u}, {1u, 11u, TRUE, 9u, 50u, 43u}, {2u, 12u, TRUE, 1u, 0u, 59u}};
static const Com_ConfigType configuration = {pdus, 3u, 0u, receive, timeout, NULL_PTR};
static void periods(unsigned count) {
    unsigned index;
    for (index = 0u; index < count; ++index) {
        Ecu_ComMainFunctionRx();
    }
}
int main(void) {
    void (*initialize)(const Com_ConfigType *) = Com_Init;
    void (*deinitialize)(void) = Com_DeInit;
    Com_StatusType (*status)(void) = Com_GetStatus;
    uint8 (*read)(Com_SignalIdType, void *) = Com_ReceiveSignal;
    uint8 (*write)(Com_SignalIdType, const void *) = Com_SendSignal;
    void (*start)(Com_IpduGroupIdType, boolean) = Com_IpduGroupStart;
    void (*stop)(Com_IpduGroupIdType) = Com_IpduGroupStop;
    void (*enable)(Com_IpduGroupIdType) = Com_EnableReceptionDM;
    void (*disable)(Com_IpduGroupIdType) = Com_DisableReceptionDM;
    void (*rx)(PduIdType, const PduInfoType *) = Com_RxIndication;
    Std_ReturnType (*pull)(PduIdType, PduInfoType *) = Com_TriggerTransmit;
    void (*confirm)(PduIdType, Std_ReturnType) = Com_TxConfirmation;
    uint32 value = 99u;
    uint8 payload[] = {0x78u, 0x56u, 0x34u, 0x12u};
    PduInfoType info = {payload, NULL_PTR, 4u};
    PduInfoType short_info = {payload, NULL_PTR, 3u};
    assert(COM_UNINIT == 0 && COM_SERVICE_NOT_AVAILABLE == 0x80u);
    assert(sizeof(Com_SignalIdType) == 2u && sizeof(Com_IpduGroupIdType) == 2u);
    assert(sizeof(PduIdType) == 1u && sizeof(PduLengthType) == 2u);
    assert(sizeof(CbkHandleIdType) == 2u);
    assert(BUFREQ_OK == 0 && BUFREQ_E_NOT_OK == 1 && BUFREQ_E_BUSY == 2 && BUFREQ_E_OVFL == 3);
    assert(TP_DATACONF == 0 && TP_DATARETRY == 1 && TP_CONFPENDING == 2);
    assert(status() == COM_UNINIT);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE && value == 99u);
    assert(write(10u, &value) == COM_SERVICE_NOT_AVAILABLE);
    periods(5u);
    initialize(NULL_PTR);
    assert(status() == COM_UNINIT);
    initialize(&configuration);
    assert(status() == COM_INIT);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE && value == 7u);
    rx(0u, &info);
    assert(acknowledgments[0] == 0u);
    start(1u, TRUE);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE);
    start(0u, TRUE);
    assert(read(10u, &value) == E_OK && value == 7u);
    assert(read(11u, &value) == E_OK && value == 9u);
    periods(100u);
    assert(timeouts[0] == 0u && timeouts[1] == 0u);
    rx(0u, NULL_PTR);
    rx(0u, &short_info);
    rx(9u, &info);
    assert(acknowledgments[0] == 0u);
    Ecu_ComReceptionBeforeMain(FALSE); /* Epoch0 has no pending periodic execution. */
    rx(0u, &info);
    rx(2u, &info);
    assert(acknowledgments[0] == 1u && acknowledgments[2] == 1u);
    assert(read(10u, &value) == E_OK && value == 0x12345678u);
    assert(read(99u, &value) == E_NOT_OK && value == 0x12345678u);
    assert(read(10u, NULL_PTR) == E_NOT_OK);
    assert(write(10u, &value) == E_NOT_OK);
    assert(pull(0u, &info) == E_NOT_OK);
    confirm(0u, E_OK);
    periods(29u);
    assert(timeouts[0] == 0u);
    enable(0u); /* Already enabled: do not reset the running timer. */
    periods(1u);
    assert(timeouts[0] == 1u && timeouts[1] == 0u && timeouts[2] == 0u);
    assert(read(10u, &value) == E_OK && value == 0x12345678u); /* NONE retains value. */
    periods(30u);
    assert(timeouts[0] == 2u);
    Ecu_ComReceptionBeforeMain(TRUE); /* Receive before this period's check. */
    rx(0u, &info);
    rx(1u, &info);
    periods(1u);
    periods(29u);
    assert(timeouts[0] == 2u && timeouts[1] == 0u);
    periods(1u);
    assert(timeouts[0] == 3u && timeouts[1] == 0u);
    periods(20u);
    assert(timeouts[1] == 1u && timeouts[2] == 0u);
    disable(0u);
    periods(100u);
    assert(timeouts[0] == 3u && timeouts[1] == 1u);
    payload[0] = 42u;
    payload[1] = 0u;
    payload[2] = 0u;
    payload[3] = 0u;
    rx(0u, &info);
    assert(read(10u, &value) == E_OK && value == 42u);
    enable(0u);
    periods(100u); /* first timeout zero: enabling alone does not restart regular DM. */
    assert(timeouts[0] == 3u && timeouts[1] == 1u);
    rx(0u, &info);
    periods(30u);
    assert(timeouts[0] == 4u);
    stop(0u);
    payload[0] = 43u;
    rx(0u, &info);
    periods(100u);
    assert(timeouts[0] == 4u);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE && value == 42u);
    start(0u, FALSE);
    assert(read(10u, &value) == E_OK && value == 42u);
    start(0u, TRUE); /* Already started: initialization must not erase buffered data. */
    assert(read(10u, &value) == E_OK && value == 42u);
    stop(0u);
    start(0u, TRUE);
    assert(read(10u, &value) == E_OK && value == 7u);
    rx(0u, &info);
    assert(read(10u, &value) == E_OK && value == 43u);
    deinitialize();
    value = 55u;
    assert(status() == COM_UNINIT);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE && value == 55u);
    rx(0u, &info);
    periods(100u);
    initialize(&configuration);
    assert(read(10u, &value) == COM_SERVICE_NOT_AVAILABLE && value == 7u);
    return 0;
}
