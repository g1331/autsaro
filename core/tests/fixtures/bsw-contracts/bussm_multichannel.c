/** @file Independent CanIf controller and ComM consumers of the CDD BusSM. */
#include "Ecu_HostBusSM.h"
#include "ComM_BswM.h"
#include <assert.h>

static Can_ControllerStateType controller_modes[256];
static CanIf_PduModeType pdu_modes[256];
static Can_ErrorStateType errors[256];
static unsigned requests[256];
static unsigned published[256];
static ComM_ModeType observed[256];

Std_ReturnType CanIf_GetPduMode(uint8 controller, CanIf_PduModeType *mode)
{
    assert(controller == 3u || controller == 9u);
    *mode = pdu_modes[controller];
    return E_OK;
}
Std_ReturnType CanIf_SetPduMode(uint8 controller, CanIf_PduModeType mode)
{
    assert(controller == 3u || controller == 9u);
    pdu_modes[controller] = mode;
    return E_OK;
}
Std_ReturnType CanIf_GetControllerMode(uint8 controller, Can_ControllerStateType *mode)
{
    assert(controller == 3u || controller == 9u);
    *mode = controller_modes[controller];
    return E_OK;
}
Std_ReturnType CanIf_GetControllerErrorState(uint8 controller, Can_ErrorStateType *error)
{
    assert(controller == 3u || controller == 9u);
    *error = errors[controller];
    return E_OK;
}
Std_ReturnType CanIf_SetControllerMode(uint8 controller, Can_ControllerStateType mode)
{
    assert(controller == 3u || controller == 9u);
    ++requests[controller];
    controller_modes[controller] = mode;
    return E_OK;
}
void ComM_BusSM_ModeIndication(NetworkHandleType channel, ComM_ModeType mode)
{
    assert(channel == 252u || channel == 89u);
    ++published[channel];
    observed[channel] = mode;
}
static void expect_mode(NetworkHandleType channel, ComM_ModeType expected)
{
    ComM_ModeType mode = 99u;
    assert(Ecu_HostBusSM_GetCurrentComMode(channel, &mode) == E_OK);
    assert(mode == expected);
}

int main(void)
{
    static const Ecu_HostBusSM_ChannelConfigType channels[] = {{252u, 9u}, {89u, 3u}};
    static const Ecu_HostBusSM_ConfigType config = {channels, 2u};
    controller_modes[3u] = CAN_CS_STOPPED;
    controller_modes[9u] = CAN_CS_STOPPED;
    pdu_modes[3u] = CANIF_OFFLINE;
    pdu_modes[9u] = CANIF_OFFLINE;
    Ecu_HostBusSM_Init(&config);

    assert(Ecu_HostBusSM_RequestComMode(89u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(requests[3u] == 1u && requests[9u] == 0u);
    expect_mode(89u, COMM_NO_COMMUNICATION);
    Ecu_HostBusSM_ControllerModeIndication(3u, CAN_CS_STARTED);
    expect_mode(89u, COMM_FULL_COMMUNICATION);
    expect_mode(252u, COMM_NO_COMMUNICATION);
    assert(published[89u] == 1u && observed[89u] == COMM_FULL_COMMUNICATION);
    assert(published[252u] == 0u);

    assert(Ecu_HostBusSM_RequestComMode(252u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(requests[3u] == 1u && requests[9u] == 1u);
    expect_mode(89u, COMM_FULL_COMMUNICATION);
    expect_mode(252u, COMM_NO_COMMUNICATION);
    Ecu_HostBusSM_ControllerModeIndication(9u, CAN_CS_STARTED);
    expect_mode(252u, COMM_FULL_COMMUNICATION);

    errors[9u] = CAN_ERRORSTATE_BUSOFF;
    controller_modes[9u] = CAN_CS_STOPPED;
    pdu_modes[9u] = CANIF_TX_OFFLINE;
    Ecu_HostBusSM_ControllerBusOff(9u);
    assert(published[252u] == 2u && observed[252u] == COMM_NO_COMMUNICATION);
    expect_mode(89u, COMM_FULL_COMMUNICATION);
    assert(published[89u] == 1u);
    assert(Ecu_HostBusSM_RequestComMode(252u, COMM_FULL_COMMUNICATION) == E_NOT_OK);

    errors[9u] = CAN_ERRORSTATE_ACTIVE;
    assert(Ecu_HostBusSM_RequestComMode(252u, COMM_FULL_COMMUNICATION) == E_OK);
    Ecu_HostBusSM_ControllerModeIndication(9u, CAN_CS_STARTED);
    expect_mode(252u, COMM_FULL_COMMUNICATION);
    assert(published[252u] == 3u && published[89u] == 1u);
    return 0;
}
