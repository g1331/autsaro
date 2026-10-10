/** Fixed controller polling/zero-DLC oracle with the shared actual driver. */
#include "Can.h"
#include "CanIf.h"
#include "SchM_Can.h"
#include "PduR.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_CanIf.h"
#include "LSduR_PduR.h"
#include "Ecu_Config.h"
#include <assert.h>
static unsigned emitted;
static unsigned received;
static unsigned positive;
static unsigned negative;
static unsigned modes;
static unsigned busoffs;
static uint8 expected_length;
const EcuPolicyConfig Ecu_Policy = {ECU_TX_SYNCHRONOUS,
                                    ECU_RX_BEFORE_DEADLINE,
                                    0u,
                                    0u,
                                    ECU_WAIT_ABORT,
                                    0u,
                                    0u,
                                    NULL_PTR,
                                    0u,
                                    50u,
                                    5000u};
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    assert(id == 0x321u && length == expected_length);
    if (length == 0u) {
        assert(data[0] == 0u);
    } else {
        assert(data[0] == 42u);
    }
    ++emitted;
    return ECU_OK;
}
static void indication(PduIdType id, const PduInfoType *info) {
    assert(id == 17u && info != NULL_PTR && info->SduLength == expected_length);
    if (info->SduLength > 0u) {
        assert(info->SduDataPtr[0] == 42u);
    }
    ++received;
}
static void confirmation(PduIdType id, Std_ReturnType result) {
    assert(id == 18u);
    if (result == E_OK) {
        ++positive;
    } else {
        assert(result == E_NOT_OK);
        ++negative;
    }
}
static void mode(uint8 controller, Can_ControllerStateType value) {
    assert(controller == 0u && (value == CAN_CS_STARTED || value == CAN_CS_STOPPED));
    ++modes;
}
static void busoff(uint8 controller) {
    Can_ControllerStateType actual = CAN_CS_STARTED;
    assert(controller == 0u);
    assert(Can_GetControllerMode(0u, &actual) == E_OK && actual == CAN_CS_STOPPED);
    ++busoffs;
}
static const Can_ConfigType can = {sink};
static const CanIf_RxPduConfigType rx[] = {{0x320u, 21u, 0u}};
static const CanIf_TxPduConfigType tx[] = {{0x321u, 51u, 4u}};
static const CanIf_ConfigType canif = {rx, 1u, tx, 1u, mode, busoff, 0u, 0u, 0u, 1u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, NULL_PTR}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 1u};
static const PduR_RxRouteType pdur_rx[] = {{31u, 17u, indication}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 18u, 41u, LSduR_PduRTransmit, confirmation, NULL_PTR}};
static const PduR_PBConfigType pdur = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
int main(void) {
    void (*poll_off)(void) = Can_MainFunction_BusOff;
    void (*poll_mode)(void) = Can_MainFunction_Mode;
    uint8 bytes[8] = {42u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    const PduInfoType zero = {bytes, NULL_PTR, 0u};
    const PduInfoType four = {bytes, NULL_PTR, 4u};
    CanIf_PduModeType pdu_mode = CANIF_OFFLINE;
    Can_Init(&can);
    CanIf_Init(&canif);
    LSduR_Init(&ls);
    PduR_Init(&pdur);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    assert(modes == 0u);
    poll_mode();
    assert(modes == 1u);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    {
        const Can_PduType wrong_hth = {51u, 0u, 0x321u, bytes};
        assert(Can_Write(0u, &wrong_hth) == E_NOT_OK);
    }
    expected_length = 0u;
    assert(CanIf_Transmit(51u, &zero) == E_OK);
    assert(CanIf_Transmit(51u, &zero) == E_NOT_OK); /* Actual driver BUSY. */
    assert(Can_HostFlush() == ECU_OK && emitted == 1u && positive == 0u);
    Can_MainFunction_Write();
    assert(positive == 1u);
    Can_MainFunction_Write();
    assert(positive == 1u);
    assert(Can_Inject(0x320u, 0u, bytes, 0u) == ECU_OK && received == 0u);
    assert(Can_Inject(0x320u, 0u, bytes, 0u) == ECU_ERR_CAN_BUSY);
    Can_MainFunction_Read();
    assert(received == 1u);
    Can_MainFunction_Read();
    assert(received == 1u);
    expected_length = 4u;
    assert(CanIf_Transmit(51u, &four) == E_OK);
    bytes[0] = 7u;
    assert(Can_HostFlush() == ECU_OK && emitted == 2u && positive == 1u);
    Can_MainFunction_Write();
    assert(positive == 2u);
    bytes[0] = 42u;
    assert(CanIf_Transmit(51u, &four) == E_OK);
    Can_SetMode(CAN_BUS_OFF);
    assert(busoffs == 0u && negative == 0u && Can_GetMode() == CAN_BUS_OFF);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_NOT_OK);
    assert(Can_SetControllerMode(0u, CAN_CS_STARTED) == E_NOT_OK);
    Can_DeInit(); /* Pending polling event must survive lifecycle requests. */
    poll_off();
    assert(busoffs == 1u && negative == 1u);
    assert(CanIf_GetPduMode(0u, &pdu_mode) == E_OK && pdu_mode == CANIF_TX_OFFLINE);
    poll_off();
    Can_SetMode(CAN_BUS_OFF);
    poll_off();
    assert(busoffs == 1u && negative == 1u);
    assert(Can_HostFlush() == ECU_OK && emitted == 2u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    poll_mode();
    assert(modes == 2u);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    expected_length = 0u;
    assert(CanIf_Transmit(51u, &zero) == E_OK);
    assert(Can_HostFlush() == ECU_OK && positive == 2u);
    Can_MainFunction_Write();
    assert(positive == 3u);
    expected_length = 4u;
    assert(Can_Inject(0x320u, 4u, bytes, 0u) == ECU_OK);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    poll_mode();
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    poll_mode();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    Can_MainFunction_Read();
    assert(received == 1u);
    assert(Can_Inject(0x320u, 4u, bytes, 0u) == ECU_OK);
    Can_MainFunction_Read();
    assert(received == 2u);
    assert(Can_Inject(0x320u, 4u, bytes, 0u) == ECU_OK);
    Can_SetMode(CAN_BUS_OFF);
    poll_off();
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    poll_mode();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    Can_MainFunction_Read();
    assert(received == 2u);
    assert(Can_Inject(0x320u, 4u, bytes, 0u) == ECU_OK);
    Can_MainFunction_Read();
    assert(received == 3u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    poll_mode();
    Can_DeInit();
    poll_off();
    assert(busoffs == 2u);
    return 0;
}
