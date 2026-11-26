/** @file Exercise the actual Can and CanIf chain on each configured driver controller. */
#include "CanIf.h"
#include "Det.h"
#include "Ecu_Execution.h"
#include "LSduR_CanIf.h"
#include "SchM_Can.h"
#include <assert.h>

#define CHANNELS CAN_TEST_CHANNELS
#define ID       0x123u

const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_QUEUED};

static uint64 tokens[CHANNELS];
static unsigned emitted[CHANNELS];
static unsigned confirmations[2u * CHANNELS];
static unsigned cancellations[2u * CHANNELS];
static unsigned receptions[2u * CHANNELS];
static unsigned faults[CHANNELS];
static unsigned offline_errors;

static uint8 abstract_for(uint8 channel) { return CHANNELS - 1u - channel; }

EcuStatus Ecu_ExecutionTransmit(Can_TxSinkType sink,
                                uint8 controller,
                                PduIdType handle,
                                uint32 id,
                                uint8 dlc,
                                const uint8 *data,
                                boolean brs,
                                uint32 data_phase_bitrate)
{
    assert(sink == NULL_PTR && controller < CHANNELS);
    const uint8 abstract = abstract_for(controller);
    assert((handle % CHANNELS) == abstract);
    assert(dlc == 1u && data[0] == handle);
    if (handle < CHANNELS) {
        assert(id == ID && brs == FALSE && data_phase_bitrate == 0u);
    } else {
        assert(id == (ID | CAN_ID_EXTENDED_FLAG | CAN_ID_FD_FLAG));
        assert(brs == TRUE && data_phase_bitrate == 2000000u);
    }
    ++emitted[controller];
    tokens[controller] = Can_HostTransmitTokenFor(controller);
    assert(tokens[controller] != 0u);
    return ECU_OK;
}

Std_ReturnType Det_ReportRuntimeError(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == CANIF_MODULE_ID && instance == 0u && api == CANIF_SID_TRANSMIT);
    assert(error == CANIF_E_STOPPED);
    ++offline_errors;
    return E_OK;
}
void LSduR_CanIfTxConfirmation(PduIdType handle, Std_ReturnType result)
{
    assert(handle < (2u * CHANNELS));
    if (result == E_OK) {
        ++confirmations[handle];
    } else {
        assert(result == E_NOT_OK);
        ++cancellations[handle];
    }
}
void LSduR_CanIfRxIndication(PduIdType handle, const PduInfoType *pdu)
{
    assert(handle < (2u * CHANNELS));
    assert(pdu->SduLength == 1u && pdu->SduDataPtr[0] == handle);
    ++receptions[handle];
}
static void mode(uint8 abstract, Can_ControllerStateType requested)
{
    Can_ControllerStateType actual = CAN_CS_UNINIT;
    assert(Can_GetControllerMode(abstract_for(abstract), &actual) == E_OK);
    assert(actual == requested);
    if (requested == CAN_CS_STARTED) {
        assert(CanIf_SetPduMode(abstract, CANIF_ONLINE) == E_OK);
    }
}
static void bus_off(uint8 abstract)
{
    assert(abstract < CHANNELS);
    ++faults[abstract];
}
static void request(PduIdType handle, Std_ReturnType expected)
{
    uint8 data = handle;
    const PduInfoType info = {&data, NULL_PTR, 1u};
    assert(CanIf_Transmit(handle, &info) == expected);
}
static void input(uint8 channel, boolean fd, EcuStatus expected)
{
    const uint8 data = abstract_for(channel) + (fd ? CHANNELS : 0u);
    const uint32 id = fd ? (ID | CAN_ID_EXTENDED_FLAG | CAN_ID_FD_FLAG) : ID;
    assert(Can_HostInjectChannel(channel, id, 1u, &data) == expected);
    Can_MainFunction_Read();
}
static void configure(void)
{
    static Can_ControllerConfigType can_controllers[CHANNELS];
    static Can_HohConfigType hoh[2u * CHANNELS];
    static CanIf_ControllerConfigType canif_controllers[CHANNELS];
    static CanIf_RxPduConfigType rx[2u * CHANNELS];
    static CanIf_TxPduConfigType tx[2u * CHANNELS];
    static const Can_ConfigType can = {
        .controllers = can_controllers,
        .controller_count = CHANNELS,
        .hardware = hoh,
        .hardware_count = 2u * CHANNELS,
    };
    static const CanIf_ConfigType canif = {
        rx, 2u * CHANNELS, tx, 2u * CHANNELS, mode, bus_off, canif_controllers, CHANNELS};
    for (uint8 channel = 0u; channel < CHANNELS; ++channel) {
        const uint8 abstract = abstract_for(channel);
        const Can_HwHandleType hrh = 2u * channel;
        const Can_HwHandleType hth = hrh + 1u;
        can_controllers[channel] =
            (Can_ControllerConfigType){channel, abstract, 500000u, 2000000u, TRUE};
        hoh[hrh] = (Can_HohConfigType){hrh, channel, CAN_HOH_ID_MIXED, TRUE, 0u};
        hoh[hth] = (Can_HohConfigType){hth, channel, CAN_HOH_ID_MIXED, FALSE, 0u};
        canif_controllers[abstract] = (CanIf_ControllerConfigType){abstract, channel};
        for (uint8 fd = 0u; fd < 2u; ++fd) {
            const PduIdType handle = abstract + (fd * CHANNELS);
            const uint32 id = (fd != 0u) ? (ID | CAN_ID_EXTENDED_FLAG | CAN_ID_FD_FLAG) : ID;
            rx[handle] = (CanIf_RxPduConfigType){id, handle, 1u, FALSE, abstract, hrh};
            tx[handle] = (CanIf_TxPduConfigType){id, handle, 1u, abstract, hth};
        }
    }
    Can_Init(&can);
    CanIf_Init(&canif);
    for (uint8 channel = 0u; channel < CHANNELS; ++channel) {
        assert(CanIf_SetControllerMode(channel, CAN_CS_STARTED) == E_OK);
    }
    Can_MainFunction_Mode();
}

int main(void)
{
    configure();
    uint8 byte = 0u;
    const PduInfoType pdu = {&byte, NULL_PTR, 1u};
    const Can_HwType wrong_hrh = {ID, 0u, 0u};
    CanIf_RxIndication(&wrong_hrh, &pdu);
    assert(receptions[0] == 0u);
    for (uint8 format = 0u; format < 2u; ++format) {
        for (uint8 channel = 0u; channel < CHANNELS; ++channel) {
            const PduIdType handle = abstract_for(channel) + (format * CHANNELS);
            input(channel, format != 0u, ECU_OK);
            assert(receptions[handle] == 1u);
            request(handle, E_OK);
        }
        assert(Can_HostFlush() == ECU_OK);
        for (uint8 channel = 0u; channel < CHANNELS; ++channel) {
            assert(emitted[channel] == (unsigned)format + 1u);
            const PduIdType handle = abstract_for(channel) + (format * CHANNELS);
            assert(Can_HostCompleteTransmitFor(channel, handle, tokens[channel]) == E_OK);
        }
        Can_MainFunction_Write();
        for (PduIdType handle = format * CHANNELS; handle < ((format + 1u) * CHANNELS); ++handle) {
            assert(confirmations[handle] == 1u && receptions[handle] == 1u);
        }
    }

    const uint8 stopped = CHANNELS - 1u;
    const uint8 other = CHANNELS - 2u;
    request(0u, E_OK);
    request(1u, E_OK);
    assert(Can_HostFlush() == ECU_OK);
    const uint64 late = tokens[stopped];
    Can_HostSetControllerMode(stopped, CAN_STOPPED);
    assert(cancellations[0] == 1u && cancellations[1] == 0u);
    assert(Can_HostCompleteTransmitFor(stopped, 0u, late) == E_NOT_OK);
    assert(Can_HostCompleteTransmitFor(other, 1u, tokens[other]) == E_OK);
    Can_MainFunction_Write();
    assert(confirmations[0] == 1u && confirmations[1] == 2u);
    input(stopped, FALSE, ECU_ERR_CONTROLLER);
    input(other, FALSE, ECU_OK);
    Can_HostSetControllerMode(stopped, CAN_STARTED);
    request(0u, E_OK);
    assert(Can_HostFlush() == ECU_OK);
    assert(tokens[stopped] > late);
    assert(Can_HostCompleteTransmitFor(stopped, 0u, late) == E_NOT_OK);
    assert(Can_HostCompleteTransmitFor(stopped, 0u, tokens[stopped]) == E_OK);
    Can_MainFunction_Write();
    assert(confirmations[0] == 2u);

    assert(CanIf_SetPduMode(0u, CANIF_OFFLINE) == E_OK);
    request(0u, E_NOT_OK);
    assert(offline_errors == 1u);
    input(stopped, FALSE, ECU_OK);
    assert(receptions[0] == 1u);
    input(other, FALSE, ECU_OK);
    assert(receptions[1] == 3u);
    Can_HostSetControllerMode(stopped, CAN_STARTED);
    input(stopped, FALSE, ECU_OK);
    assert(receptions[0] == 2u);

    Can_HostSetControllerMode(stopped, CAN_BUS_OFF);
    Can_MainFunction_BusOff();
    assert(faults[0] == 1u && faults[1] == 0u);
    input(stopped, FALSE, ECU_ERR_CONTROLLER);
    input(other, FALSE, ECU_OK);
    Can_HostSetControllerMode(stopped, CAN_STARTED);
    input(stopped, FALSE, ECU_OK);
    assert(receptions[0] == 3u && receptions[1] == 4u);
    return 0;
}
