/** Independently authored R24-11 COM/PduR/LSduR/CanIf/Can consumer.
 * Actual delivery modules and driver; only host output/error/mode observers.
 */
#include "Com.h"
#include "Com_Internal.h"
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "LSduR_CanTp.h"
#include "LSduR_CanIf.h"
#include "CanIf.h"
#include "Can.h"
#include "SchM_Can.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>

static unsigned frames;
static unsigned successes;
static unsigned failures;
static uint32 emitted[8];
static unsigned acknowledgments;
static unsigned timeouts;
static unsigned errors;
static uint8 last_api;
static uint8 last_error;
static uint8 previous_error;
static unsigned modes;
static unsigned bus_offs;
/* Actual queued profile policy: flushing does not synthesize confirmation.
 * This verifies PERIODIC COM continues while driver acceptance is possible.
 */
const EcuPolicyConfig Ecu_Policy = {
    ECU_TX_QUEUED, ECU_RX_BEFORE_DEADLINE, 0u, 0u, ECU_WAIT_ABORT, 0u, 0u, NULL_PTR, 0u, 50u,
    5000u};
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    assert(id == 0x456u && length == 4u && frames < 8u);
    emitted[frames++] = (uint32)data[0] | ((uint32)data[1] << 8u) | ((uint32)data[2] << 16u) |
                        ((uint32)data[3] << 24u);
    return ECU_OK;
}
static void receive(CbkHandleIdType handle) {
    assert(handle == 17u);
    ++acknowledgments;
}
static void timeout(CbkHandleIdType handle) {
    assert(handle == 17u);
    ++timeouts;
}
static void mode(uint8 controller, Can_ControllerStateType state) {
    assert(controller == 0u);
    assert(state == CAN_CS_STARTED || state == CAN_CS_STOPPED || state == CAN_CS_SLEEP);
    ++modes;
}
static void bus_off(uint8 controller) {
    assert(controller == 0u);
    ++bus_offs;
}
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code) {
    assert(module == 60u && instance == 0u);
    last_api = api;
    previous_error = last_error;
    last_error = code;
    ++errors;
    return E_NOT_OK; /* DET runtime return remains E_OK and does not halt. */
}
/* Observe the actual upper callback without replacing its implementation. */
static void confirmed(PduIdType id, Std_ReturnType result) {
    assert(id == 2u);
    Com_TxConfirmation(id, result);
    if (result == E_OK) {
        ++successes;
    } else {
        assert(result == E_NOT_OK);
        ++failures;
    }
}
static const Det_ErrorHookType error_hooks[] = {error};
static const Det_ConfigType det_config = {NULL_PTR, 0u, error_hooks, 1u};
static const Com_PduConfigType pdus[] = {{1u, 10u, TRUE, 7u, 30u, 17u},
                                         {2u, 11u, FALSE, 9u, 0u, 0u}};
static const Com_ConfigType com_config = {pdus, 2u, 0u, receive, timeout, PduR_ComTransmit};
static const PduR_RxRouteType pdur_rx[] = {{31u, 1u, Com_RxIndication}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 2u, 41u, LSduR_PduRTransmit, confirmed, Com_TriggerTransmit}};
static const PduR_PBConfigType pdur_config = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}};
static const LSduR_PBConfigType ls_config = {29u, ls_rx, 1u, ls_tx, 1u};
static const CanIf_RxPduConfigType canif_rx[] = {{0x123u, 21u, 4u}};
static const CanIf_TxPduConfigType canif_tx[] = {{0x456u, 51u, 4u}};
static const CanIf_ConfigType canif_config = {canif_rx, 1u, canif_tx, 1u, mode,
                                              bus_off,  0u, 0u,       0u, 0u};
static const Can_ConfigType can_config = {sink};
int main(void) {
    /* Fixed signatures are independent of the implementation/inventory. */
    void (*pdur_init)(const PduR_PBConfigType *) = PduR_Init;
    void (*ls_init)(const LSduR_PBConfigType *) = LSduR_Init;
    Std_ReturnType (*ls_tp)(PduIdType, const PduInfoType *) = LSduR_CanTpTransmit;
    void (*ls_rx)(PduIdType, const PduInfoType *) = LSduR_CanIfRxIndication;
    void (*ls_confirm)(PduIdType, Std_ReturnType) = LSduR_CanIfTxConfirmation;
    Std_ReturnType (*can_write)(Can_HwHandleType, const Can_PduType *) = Can_Write;
    Std_ReturnType (*can_mode)(uint8, Can_ControllerStateType) = Can_SetControllerMode;
    uint8 bytes[8] = {42u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    PduInfoType info = {bytes, NULL_PTR, 4u};
    uint32 value = 99u;
    CanIf_PduModeType channel = CANIF_ONLINE;
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_PduType direct = {51u, 4u, 0x456u, bytes};
    assert(ls_tp(41u, &info) == E_NOT_OK);
    ls_rx(21u, &info);
    ls_confirm(51u, E_OK);
    assert(PduR_ComTransmit(2u, &info) == E_NOT_OK);
    assert(PduR_GetConfigurationId() == 0u && LSduR_GetConfigurationId() == 0u);
    Det_Init(&det_config);
    pdur_init(&pdur_config);
    ls_init(&ls_config);
    CanIf_Init(&canif_config);
    Com_Init(&com_config);
    Can_Init(&can_config);
    assert(PduR_GetConfigurationId() == 23u && LSduR_GetConfigurationId() == 29u);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_OFFLINE);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_NOT_OK);
    assert(CanIf_Transmit(51u, &info) == E_NOT_OK && errors == 1u);
    assert(last_api == 0x49u && last_error == 70u);
    assert(can_mode(0u, CAN_CS_STARTED) == E_OK);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STARTED);
    Can_MainFunction_Wakeup();
    assert(modes == 1u);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STARTED);
    assert(CanIf_SetPduMode(0u, CANIF_TX_OFFLINE_ACTIVE) == E_NOT_OK);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    assert(CanIf_GetPduMode(1u, &channel) == E_NOT_OK && channel == CANIF_OFFLINE);
    Com_IpduGroupStart(0u, TRUE);
    assert(Can_Inject(0x123u, 3u, bytes, 0u) == ECU_ERR_FRAME_DLC);
    assert(errors == 2u && last_api == 20u && last_error == 61u && acknowledgments == 0u);
    assert(Can_Inject(0x777u, 4u, bytes, 0u) == ECU_ERR_FRAME_ID);
    assert(Can_Inject(0x123u, 4u, bytes, 0u) == ECU_OK);
    assert(acknowledgments == 1u);
    assert(Com_ReceiveSignal(10u, &value) == E_OK && value == 42u);
    {
        unsigned period;
        for (period = 0u; period < 29u; ++period) {
            Ecu_ComMainFunctionRx();
        }
        assert(timeouts == 0u);
        Ecu_ComMainFunctionRx();
        assert(timeouts == 1u);
    }
    value = 0x12345678u;
    assert(Com_SendSignal(11u, &value) == E_OK);
    Ecu_ComMainFunctionTx();
    assert(can_write(0u, &direct) == CAN_BUSY);
    value = 43u;
    assert(Com_SendSignal(11u, &value) == E_OK);
    Ecu_ComMainFunctionTx(); /* Actual BUSY must preserve the first copied request. */
    assert(Can_HostFlush() == ECU_OK && frames == 1u && emitted[0] == 0x12345678u);
    Ecu_ComMainFunctionTx(); /* Queued confirmation is still outstanding. */
    assert(Can_HostFlush() == ECU_OK && frames == 2u && emitted[1] == 43u);
    CanIf_TxConfirmation(51u);
    CanIf_TxConfirmation(51u);
    assert(successes == 2u && failures == 0u);
    CanIf_TxConfirmation(51u); /* Late duplicate ignored. */
    assert(successes == 2u);
    info.SduLength = 5u;
    assert(PduR_ComTransmit(2u, &info) == E_NOT_OK);
    assert(errors == 3u && last_api == 0x49u && last_error == 90u);
    info.SduLength = 9u;
    assert(PduR_ComTransmit(2u, &info) == E_NOT_OK);
    assert(errors == 5u && previous_error == 62u && last_error == 90u);
    info.SduLength = 4u;
    assert(PduR_ComTransmit(99u, &info) == E_NOT_OK);
    assert(ls_tp(41u, &info) == E_NOT_OK); /* Different configured upper module. */
    {
        uint8 pull_data[4] = {9u, 9u, 9u, 9u};
        PduInfoType pull = {pull_data, NULL_PTR, 3u};
        assert(LSduR_CanIfTriggerTransmit(51u, &pull) == E_NOT_OK);
        assert(pull.SduLength == 3u && pull_data[0] == 9u && pull_data[3] == 9u);
        pull.SduLength = 4u;
        assert(LSduR_CanIfTriggerTransmit(51u, &pull) == E_OK && pull_data[0] == 43u);
    }
    Ecu_ComMainFunctionTx();
    assert(Can_HostFlush() == ECU_OK && frames == 3u);
    Ecu_ComMainFunctionTx();
    assert(Can_HostFlush() == ECU_OK && frames == 4u);
    assert(CanIf_SetPduMode(0u, CANIF_TX_OFFLINE) == E_OK);
    CanIf_TxConfirmation(51u); /* Actual queued driver confirmation arrives offline. */
    assert(successes == 2u && failures == 0u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STOPPED);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_NOT_OK);
    assert(PduR_ComTransmit(2u, &info) == E_NOT_OK);
    assert(successes == 2u && failures == 0u && modes == 1u);
    Can_MainFunction_Wakeup();
    assert(modes == 2u && successes == 2u && failures == 2u);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(Can_HostFlush() == ECU_OK && frames == 4u);
    CanIf_TxConfirmation(51u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    Ecu_ComMainFunctionTx();
    assert(Can_HostFlush() == ECU_OK && frames == 5u && emitted[4] == 43u);
    Can_SetMode(CAN_BUS_OFF);
    assert(bus_offs == 1u && failures == 3u);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STOPPED);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_NOT_OK);
    assert(failures == 3u);
    Can_SetMode(CAN_BUS_OFF);
    CanIf_TxConfirmation(51u);
    assert(bus_offs == 2u && failures == 3u && successes == 2u);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(CanIf_SetControllerMode(0u, CAN_CS_SLEEP) == E_OK);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_OFFLINE);
    Can_MainFunction_Wakeup();
    Com_DeInit();
    CanIf_DeInit();
    Can_DeInit();
    assert(CanIf_GetPduMode(0u, &channel) == E_NOT_OK);
    return 0;
}
