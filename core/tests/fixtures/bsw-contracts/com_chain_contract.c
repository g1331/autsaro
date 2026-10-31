/** Independently authored R24-11 COM/PduR/LSduR/CanIf/Can consumer.
 * Actual delivery modules and driver; only host output/error/mode observers.
 * Verifies: SWS_Com_00138、00669、00759、00624、00840、00861、00348、00647、00869、00478、00032
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
#include "Can_Host.h"
#include "SchM_Can.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>

static unsigned frames;
static unsigned successes;
static unsigned failures;
static uint32 emitted[12];
static uint64_t emitted_tokens[12];
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
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_QUEUED};
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8])
{
    assert(id == 0x456u && length == 4u && frames < 12u);
    emitted_tokens[frames] = Can_HostTransmitToken();
    assert(emitted_tokens[frames] != UINT64_C(0));
    emitted[frames++] = (uint32)data[0] | ((uint32)data[1] << 8u) | ((uint32)data[2] << 16u) |
                        ((uint32)data[3] << 24u);
    return ECU_OK;
}
static void receive(CbkHandleIdType handle)
{
    assert(handle == 17u);
    ++acknowledgments;
}
static void timeout(CbkHandleIdType handle)
{
    assert(handle == 17u);
    ++timeouts;
}
static void mode(uint8 controller, Can_ControllerStateType state)
{
    assert(controller == 0u);
    assert(state == CAN_CS_STARTED || state == CAN_CS_STOPPED || state == CAN_CS_SLEEP);
    ++modes;
}
static void bus_off(uint8 controller)
{
    assert(controller == 0u);
    ++bus_offs;
}
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code)
{
    assert(module == 60u && instance == 0u);
    last_api = api;
    previous_error = last_error;
    last_error = code;
    ++errors;
    return E_NOT_OK; /* DET runtime return remains E_OK and does not halt. */
}
/* Observe the actual upper callback without replacing its implementation. */
static void confirmed(PduIdType id, Std_ReturnType result)
{
    assert(id == 2u);
    Com_TxConfirmation(id, result);
    if (result == E_OK) {
        ++successes;
    } else {
        assert(result == E_NOT_OK);
        ++failures;
    }
}
static unsigned rejections;
static unsigned lower_returned;
static Std_ReturnType lower(PduIdType id, const PduInfoType *info)
{
    Std_ReturnType result;
    lower_returned = 0u;
    result = PduR_ComTransmit(id, info);
    lower_returned = 1u;
    return result;
}
static void rejection(PduIdType id, Std_ReturnType result)
{
    assert(Com_GetStatus() == COM_INIT);
    assert(lower_returned == 1u && id == 2u && result == E_NOT_OK);
    ++rejections;
}
static const Det_ErrorHookType error_hooks[] = {error};
static const Det_ConfigType det_config = {NULL_PTR, 0u, error_hooks, 1u};
static const Com_PduConfigType pdus[] = {{1u, 10u, TRUE, 7u, 30u, 17u},
                                         {2u, 11u, FALSE, 9u, 0u, 0u}};
static const Com_ConfigType com_config = {pdus, 2u, 0u, receive, timeout, lower, rejection};
static const PduR_RxRouteType pdur_rx[] = {{31u, 1u, Com_RxIndication}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 2u, PDUR_LO_LSDUR, 41u, LSduR_PduRTransmit, confirmed, Com_TriggerTransmit}};
static const PduR_PBConfigType pdur_config = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}};
static const LSduR_PBConfigType ls_config = {29u, ls_rx, 1u, ls_tx, 1u};
static const CanIf_RxPduConfigType canif_rx[] = {{0x123u, 21u, 4u}};
static const CanIf_TxPduConfigType canif_tx[] = {{0x456u, 51u, 4u}};
static const CanIf_ConfigType canif_config = {
    canif_rx, 1u, canif_tx, 1u, mode, bus_off, 0u, 0u, 0u, 0u};
static const Can_ConfigType can_config = {sink};
int main(void)
{
    /* Fixed signatures are independent of the implementation/inventory. */
    void (*pdur_init)(const PduR_PBConfigType *) = PduR_Init;
    void (*ls_init)(const LSduR_PBConfigType *) = LSduR_Init;
    Std_ReturnType (*ls_tp)(PduIdType, const PduInfoType *) = LSduR_CanTpTransmit;
    void (*ls_rx)(PduIdType, const PduInfoType *) = LSduR_CanIfRxIndication;
    void (*ls_confirm)(PduIdType, Std_ReturnType) = LSduR_CanIfTxConfirmation;
    Std_ReturnType (*can_write)(Can_HwHandleType, const Can_PduType *) = Can_Write;
    Std_ReturnType (*can_mode)(uint8, Can_ControllerStateType) = Can_SetControllerMode;
    Std_ReturnType (*manual_send)(PduIdType) = Com_TriggerIPDUSend;
    uint8 bytes[8] = {42u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    PduInfoType info = {bytes, NULL_PTR, 4u};
    uint32 value = 99u;
    CanIf_PduModeType channel = CANIF_ONLINE;
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_PduType direct = {51u, 4u, 0x456u, bytes};
    assert(Can_HostTransmitToken() == UINT64_C(0));
    assert(Can_HostCompleteTransmit(51u, UINT64_C(1)) == E_NOT_OK);
    assert(manual_send(2u) == E_NOT_OK);
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
    Com_EnableReceptionDM(0u);
    /* The driver accepts both frames; CanIf rejects the short one with Det 61 and
     * filters the unconfigured one without an indication or error. */
    assert(Can_Inject(0x123u, 3u, bytes, 0u) == ECU_OK);
    assert(errors == 2u && last_api == 20u && last_error == 61u && acknowledgments == 0u);
    assert(Can_Inject(0x777u, 4u, bytes, 0u) == ECU_OK);
    assert(errors == 2u && acknowledgments == 0u);
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
    assert(rejections == 1u);
    assert(Can_HostFlush() == ECU_OK && frames == 1u && emitted[0] == 0x12345678u);
    Ecu_ComMainFunctionTx(); /* Queued confirmation is still outstanding. */
    assert(rejections == 1u);
    assert(Can_HostFlush() == ECU_OK && frames == 2u && emitted[1] == 43u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[0]) == E_OK);
    assert(successes == 0u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[1]) == CAN_BUSY);
    Can_MainFunction_Write();
    assert(successes == 1u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[0]) == E_NOT_OK);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[1]) == E_OK);
    Can_MainFunction_Write();
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
    /* Actual queued driver confirmation arrives offline: suppressed, and its request
     * is no longer outstanding when STOPPED fails the other one
     * (Verifies: SWS_CANIF_00489, 00739). */
    CanIf_TxConfirmation(51u);
    assert(successes == 2u && failures == 0u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STOPPED);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_NOT_OK);
    assert(PduR_ComTransmit(2u, &info) == E_NOT_OK);
    assert(successes == 2u && failures == 0u && modes == 1u);
    Can_MainFunction_Wakeup();
    assert(modes == 2u && successes == 2u && failures == 1u);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(Can_HostFlush() == ECU_OK && frames == 4u);
    CanIf_TxConfirmation(51u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    Ecu_ComMainFunctionTx();
    assert(Can_HostFlush() == ECU_OK && frames == 5u && emitted[4] == 43u);
    Can_SetMode(CAN_BUS_OFF);
    assert(bus_offs == 1u && failures == 2u);
    assert(CanIf_GetControllerMode(0u, &state) == E_OK && state == CAN_CS_STOPPED);
    /* The driver state machine loops STOPPED to STOPPED; nothing is left to cancel. */
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    assert(failures == 2u);
    Can_SetMode(CAN_BUS_OFF);
    CanIf_TxConfirmation(51u);
    assert(bus_offs == 2u && failures == 2u && successes == 2u);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_TX_OFFLINE);
    assert(CanIf_SetControllerMode(0u, CAN_CS_SLEEP) == E_OK);
    assert(CanIf_GetPduMode(0u, &channel) == E_OK && channel == CANIF_OFFLINE);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(Com_TriggerIPDUSend(1u) == E_NOT_OK); /* Rx is not a Tx request. */
    assert(Com_TriggerIPDUSend(99u) == E_NOT_OK);
    /* Started I-PDU: triggered although the offline lower rejects it, never retried. */
    assert(Com_TriggerIPDUSend(2u) == E_OK);
    assert(rejections == 2u);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    assert(Can_HostFlush() == ECU_OK && frames == 5u); /* No deferred manual trigger. */
    Com_IpduGroupStop(0u); /* Selected group contains Rx only; ungrouped Tx stays started. */
    assert(Com_TriggerIPDUSend(2u) == E_OK);
    value = 44u;
    assert(Com_SendSignal(11u, &value) == E_OK);
    assert(Com_TriggerIPDUSend(2u) == E_OK); /* Actual lower BUSY, no queue. */
    assert(rejections == 3u);
    assert(Can_HostFlush() == ECU_OK && frames == 6u && emitted[5] == 43u);
    /* Old STOP/bus-off outputs cannot consume the recovered same-ID request. */
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[3]) == E_NOT_OK);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[4]) == E_NOT_OK);
    Can_MainFunction_Write();
    assert(successes == 2u && failures == 2u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[5]) == E_OK);
    Can_MainFunction_Write();
    assert(successes == 3u && failures == 2u);
    assert(Can_HostFlush() == ECU_OK && frames == 6u);
    assert(Com_TriggerIPDUSend(2u) == E_OK);
    assert(Can_HostFlush() == ECU_OK && frames == 7u && emitted[6] == 44u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[6]) == E_OK);
    Can_MainFunction_Write();
    assert(successes == 4u);
    Com_DeInit();
    assert(Com_TriggerIPDUSend(2u) == E_NOT_OK);
    Com_Init(&com_config);
    assert(Can_HostFlush() == ECU_OK && frames == 7u);
    Com_DeInit();
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    Can_MainFunction_Wakeup();
    CanIf_DeInit();
    Can_DeInit();
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[6]) == E_NOT_OK);
    assert(CanIf_GetPduMode(0u, &channel) == E_NOT_OK);
    Can_Init(&can_config);
    CanIf_Init(&canif_config);
    Com_Init(&com_config);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    assert(Com_TriggerIPDUSend(2u) == E_OK);
    assert(Can_HostFlush() == ECU_OK && frames == 8u);
    assert(emitted_tokens[7] > emitted_tokens[6]);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[6]) == E_NOT_OK);
    Can_MainFunction_Write();
    assert(successes == 4u);
    assert(Can_HostCompleteTransmit(51u, emitted_tokens[7]) == E_OK);
    Can_MainFunction_Write();
    assert(successes == 5u && failures == 2u);
    /* FALSE selects the absent ComTxModeFalse, i.e. NONE: no periodic request while a
     * manual trigger still sends; TRUE resumes at the next main call, Init restores TRUE.
     * (Verifies: SWS_Com_00784, 00135, 00799, 00239, 00244, 00877, 00203, 00643) */
    {
        void (*switch_mode)(PduIdType, boolean) = Com_SwitchIpduTxMode;
        switch_mode(2u, FALSE);
        switch_mode(1u, TRUE); /* Rx PDUs have no transmission mode. */
        switch_mode(99u, TRUE);
        Ecu_ComMainFunctionTx();
        assert(Can_HostFlush() == ECU_OK && frames == 8u);
        switch_mode(2u, FALSE); /* Already active: no effect. */
        assert(Com_TriggerIPDUSend(2u) == E_OK);
        assert(Can_HostFlush() == ECU_OK && frames == 9u);
        assert(Can_HostCompleteTransmit(51u, emitted_tokens[8]) == E_OK);
        Can_MainFunction_Write();
        Ecu_ComMainFunctionTx();
        assert(Can_HostFlush() == ECU_OK && frames == 9u && successes == 6u);
        switch_mode(2u, TRUE);
        Ecu_ComMainFunctionTx();
        assert(Can_HostFlush() == ECU_OK && frames == 10u);
        assert(Can_HostCompleteTransmit(51u, emitted_tokens[9]) == E_OK);
        Can_MainFunction_Write();
        switch_mode(2u, FALSE);
        Com_DeInit();
        Com_Init(&com_config);
        assert(Com_InvalidateSignal(11u) == COM_SERVICE_NOT_AVAILABLE);
        Ecu_ComMainFunctionTx();
        assert(Can_HostFlush() == ECU_OK && frames == 11u && successes == 7u);
        assert(emitted[10] == 9u);
    }
    return 0;
}
