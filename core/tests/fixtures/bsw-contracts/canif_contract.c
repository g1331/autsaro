/** Independent R24-11 CanIf oracle: expectations come from the SWS, not the
 * implementation. Real Can driver and LSduR; test upper layer and Det hook only.
 */
#include "CanIf.h"
#include "LSduR.h"
#include "Can.h"
#include "Can_Host.h"
#include "SchM_Can.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>

/* Input pointers are const (Verifies[CanIf]: SWS_BSW_00186). */
/* Standard signatures over the imported Can types (Verifies: SWS_CANIF_00142), in order
 * (Verifies: SWS_CANIF_00001, 91002, 00005, 00003, 00229, 91001, 91003, 91004, 00008, 00009,
 * 00006, 00007, 00699, 00218). */
static void (*const init)(const CanIf_ConfigType *) = CanIf_Init;
static void (*const deinit)(void) = CanIf_DeInit;
static Std_ReturnType (*const transmit)(PduIdType, const PduInfoType *) = CanIf_Transmit;
static Std_ReturnType (*const set_controller)(uint8,
                                              Can_ControllerStateType) = CanIf_SetControllerMode;
static Std_ReturnType (*const get_controller)(uint8,
                                              Can_ControllerStateType *) = CanIf_GetControllerMode;
static Std_ReturnType (*const error_state)(uint8,
                                           Can_ErrorStateType *) = CanIf_GetControllerErrorState;
static Std_ReturnType (*const rx_counter)(uint8, uint8 *) = CanIf_GetControllerRxErrorCounter;
static Std_ReturnType (*const tx_counter)(uint8, uint8 *) = CanIf_GetControllerTxErrorCounter;
static Std_ReturnType (*const set_pdu)(uint8, CanIf_PduModeType) = CanIf_SetPduMode;
static Std_ReturnType (*const get_pdu)(uint8, CanIf_PduModeType *) = CanIf_GetPduMode;
static void (*const rx_indication)(const Can_HwType *, const PduInfoType *) = CanIf_RxIndication;
static void (*const tx_confirmation)(PduIdType) = CanIf_TxConfirmation;
static void (*const mode_indication)(uint8,
                                     Can_ControllerStateType) = CanIf_ControllerModeIndication;
static void (*const bus_off_indication)(uint8) = CanIf_ControllerBusOff;
/* Verifies: SWS_CANIF_00137 enumeration values. */
typedef char pdu_mode_values[(CANIF_OFFLINE == 0 && CANIF_TX_OFFLINE == 1 &&
                              CANIF_TX_OFFLINE_ACTIVE == 2 && CANIF_ONLINE == 3)
                                 ? 1
                                 : -1];

const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
static unsigned indications[2];
static PduLengthType indicated_length;
static unsigned successes;
static unsigned failures;
static unsigned frames;
static unsigned errors[100];
static unsigned modes;
static unsigned started;
static unsigned bus_offs;
static void indicated(PduIdType id, const PduInfoType *info) {
    assert((id == 10u || id == 11u) && info != NULL_PTR && info->SduDataPtr != NULL_PTR);
    indicated_length = info->SduLength;
    ++indications[id - 10u];
}
static void confirmed(PduIdType id, Std_ReturnType result) {
    assert(id == 20u);
    if (result == E_OK) {
        ++successes;
    } else {
        assert(result == E_NOT_OK);
        ++failures;
    }
}
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    /* Written through the HTH with the configured CAN ID and classic
     * standard-frame flags (Verifies: SWS_CANIF_00292, 00666, 00318, 00243). */
    assert(id == 0x200u && length == 4u && data[0] == 0x11u);
    ++frames;
    return ECU_OK;
}
/* The configured CDD dispatch targets (Verifies: SWS_CANIF_00711／00724). */
static void mode(uint8 id, Can_ControllerStateType state) {
    assert(id == 0u);
    ++modes;
    if (state == CAN_CS_STARTED) {
        ++started;
    }
}
static void bus_off(uint8 id) {
    assert(id == 0u);
    ++bus_offs;
}
/* uint8 runtime errors reported to Det (Verifies[CanIf]: SWS_BSW_00219, 00222). */
/* Runtime errors only (Verifies: SWS_CANIF_91007); module 60, instance 0, API 0x49
 * Transmit and 0x14 RxIndication. */
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code) {
    assert(module == 60u && instance == 0u && code < 100u);
    assert((api == 0x49u && (code == 62u || code == 70u || code == 90u)) ||
           (api == 0x14u && code == 61u));
    ++errors[code];
    return E_OK;
}
static const Det_ErrorHookType hooks[] = {error};
static const Det_ConfigType det = {NULL_PTR, 0u, hooks, 1u};
static const LSduR_RxRouteType ls_rx[] = {{0u, 10u, indicated}, {1u, 11u, indicated}};
static const LSduR_TxRouteType ls_tx[] = {{LSDUR_UP_PDUR, 20u, 0u, confirmed, NULL_PTR}};
static const LSduR_PBConfigType ls = {1u, ls_rx, 2u, ls_tx, 1u};
static const CanIf_RxPduConfigType rx[] = {{0x100u, 0u, 4u}, {0x101u, 1u, 2u}};
static const CanIf_TxPduConfigType tx[] = {{0x200u, 0u, 4u}};
/* Verifies: SWS_CANIF_00144／00523: the set carries every L-PDU and the dispatch targets. */
static const CanIf_ConfigType canif = {rx, 2u, tx, 1u, mode, bus_off, 0u, 0u, 0u, 0u};
/* Two Rx PDUs on one CAN ID cannot be routed. */
static const CanIf_RxPduConfigType ambiguous[] = {{0x100u, 0u, 4u}, {0x100u, 1u, 4u}};
static const CanIf_ConfigType invalid = {ambiguous, 2u, tx, 1u, mode, bus_off, 0u, 0u, 0u, 0u};
static const Can_ConfigType can = {sink};
static uint8 payload[9] = {0x11u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 9u};

static void receive(uint8 controller, Can_HwHandleType hoh, Can_IdType id, PduLengthType length) {
    const Can_HwType mailbox = {id, hoh, controller};
    const PduInfoType info = {payload, NULL_PTR, length};
    rx_indication(&mailbox, &info);
}
static Std_ReturnType send(PduLengthType length) {
    const PduInfoType info = {payload, NULL_PTR, length};
    return transmit(0u, &info);
}
static void controller(Can_ControllerStateType state) {
    /* Verifies: SWS_CANIF_00308: forwarded to the driver, which reports the transition. */
    assert(set_controller(0u, state) == E_OK);
    Can_MainFunction_Wakeup(); /* Host driver completes the transition asynchronously. */
}
static void uninitialized(void) {
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_ErrorStateType error_status = CAN_ERRORSTATE_ACTIVE;
    CanIf_PduModeType pdu = CANIF_ONLINE;
    uint8 counter = 0u;
    const unsigned before = indications[0] + successes + failures + modes + bus_offs;
    /* Verifies: SWS_CANIF_00661: no service other than Init runs before a valid Init. */
    assert(send(4u) == E_NOT_OK);
    assert(set_controller(0u, CAN_CS_STARTED) == E_NOT_OK);
    assert(get_controller(0u, &state) == E_NOT_OK);
    assert(error_state(0u, &error_status) == E_NOT_OK);
    assert(rx_counter(0u, &counter) == E_NOT_OK && tx_counter(0u, &counter) == E_NOT_OK);
    assert(set_pdu(0u, CANIF_ONLINE) == E_NOT_OK && get_pdu(0u, &pdu) == E_NOT_OK);
    /* Callbacks are ignored (Verifies: SWS_CANIF_00421, 00412, 00702, 00431). */
    receive(0u, 0u, 0x100u, 4u);
    tx_confirmation(0u);
    mode_indication(0u, CAN_CS_STOPPED);
    bus_off_indication(0u);
    assert(indications[0] + successes + failures + modes + bus_offs == before);
}

int main(void) {
    /* R24-11 BSWGeneral Appendix A.2 and the module API tables, independent of runtime. */
    assert(CANIF_MODULE_ID == 60u && CANIF_SID_TRANSMIT == 0x49u &&
           CANIF_SID_RX_INDICATION == 0x14u);
    CanIf_PduModeType pdu = CANIF_ONLINE;
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_ErrorStateType error_status = CAN_ERRORSTATE_BUSOFF;
    Det_Init(&det);
    LSduR_Init(&ls);
    uninitialized();
    init(&canif);
    Can_Init(&can);
    /* Verifies: SWS_CANIF_00864: Init leaves every channel OFFLINE (Verifies: SWS_CANIF_00085)
     * and the module initialized (Verifies[CanIf]: SWS_BSW_00071). */
    assert(get_pdu(0u, &pdu) == E_OK && pdu == CANIF_OFFLINE);
    /* Verifies: SWS_CANIF_00874: PDU mode requests need a STARTED controller. */
    assert(set_pdu(0u, CANIF_ONLINE) == E_NOT_OK);
    controller(CAN_CS_STARTED);
    assert(started == 1u);
    assert(get_controller(0u, &state) == E_OK && state == CAN_CS_STARTED);
    assert(error_state(0u, &error_status) == E_OK && error_status == CAN_ERRORSTATE_ACTIVE);
    assert(set_pdu(0u, CANIF_ONLINE) == E_OK);
    assert(get_pdu(0u, &pdu) == E_OK && pdu == CANIF_ONLINE);

    /* Reception routes by controller, HRH and CAN ID (Verifies: SWS_CANIF_00291, 00662,
     * 00665, 00415, 00030, 00211, 00389); the standard-only ID type drops extended
     * and FD flagged identifiers (Verifies: SWS_CANIF_00877). */
    receive(0u, 0u, 0x100u, 4u);
    assert(indications[0] == 1u && indicated_length == 4u);
    receive(1u, 0u, 0x100u, 4u);
    receive(0u, 1u, 0x100u, 4u);
    receive(0u, 0u, 0x300u, 4u);
    receive(0u, 0u, 0x80000100u, 4u);
    receive(0u, 0u, 0x40000100u, 4u);
    assert(indications[0] == 1u && indications[1] == 0u && errors[61] == 0u);
    /* A DLC below the configured length is dropped with CANIF_E_INVALID_DATA_LENGTH
     * (Verifies: SWS_CANIF_00026, 00168, 00390); an accepted one passes the received length
     * (Verifies: SWS_CANIF_00829, 00056). */
    receive(0u, 0u, 0x100u, 3u);
    assert(indications[0] == 1u && errors[61] == 1u);
    receive(0u, 0u, 0x100u, 6u);
    assert(indications[0] == 2u && indicated_length == 6u);
    /* Can→CanIf seam: the driver reports the CanIf controller ID and HRH. */
    assert(Can_Inject(0x101u, 2u, payload, 0u) == ECU_OK);
    assert(indications[1] == 1u && indicated_length == 2u);

    /* TX_OFFLINE keeps reception and refuses transmission without CANIF_E_STOPPED
     * (Verifies: SWS_CANIF_00489, 00382). */
    assert(set_pdu(0u, CANIF_TX_OFFLINE) == E_OK);
    receive(0u, 0u, 0x100u, 4u);
    assert(indications[0] == 3u);
    assert(send(4u) == E_NOT_OK && errors[70] == 0u);
    /* OFFLINE drops reception; Transmit reports CANIF_E_STOPPED
     * (Verifies: SWS_CANIF_00073, 00382). */
    assert(set_pdu(0u, CANIF_OFFLINE) == E_OK);
    receive(0u, 0u, 0x100u, 4u);
    assert(indications[0] == 3u);
    assert(send(4u) == E_NOT_OK && errors[70] == 1u);
    /* TX_OFFLINE_ACTIVE is not selected. */
    assert(set_pdu(0u, CANIF_TX_OFFLINE_ACTIVE) == E_NOT_OK);

    /* Length checks refuse without truncation (Verifies: SWS_CANIF_00893, 00900). */
    assert(set_pdu(0u, CANIF_ONLINE) == E_OK);
    assert(send(9u) == E_NOT_OK && errors[62] == 1u && errors[90] == 1u);
    assert(send(5u) == E_NOT_OK && errors[62] == 1u && errors[90] == 2u);
    assert(frames == 0u);

    /* ONLINE forwards and confirms once (Verifies: SWS_CANIF_00075, 00317, 00162, 00383); a
     * repeated driver confirmation has no outstanding request. */
    assert(send(4u) == E_OK);
    assert(Can_HostFlush() == ECU_OK);
    assert(frames == 1u && successes == 1u && failures == 0u);
    tx_confirmation(0u);
    assert(successes == 1u);

    /* A confirmation while TX_OFFLINE is suppressed and the request is closed:
     * a later STOPPED must not report it as failed (Verifies: SWS_CANIF_00489, 00739). */
    assert(send(4u) == E_OK);
    assert(set_pdu(0u, CANIF_TX_OFFLINE) == E_OK);
    assert(Can_HostFlush() == ECU_OK);
    assert(frames == 2u && successes == 1u && failures == 0u);
    controller(CAN_CS_STOPPED);
    assert(failures == 0u);
    /* STOPPED sets TX_OFFLINE and no Can_Write follows (Verifies: SWS_CANIF_00866, 00677). */
    assert(get_pdu(0u, &pdu) == E_OK && pdu == CANIF_TX_OFFLINE);
    assert(send(4u) == E_NOT_OK && set_pdu(0u, CANIF_ONLINE) == E_NOT_OK);
    assert(Can_HostFlush() == ECU_OK && frames == 2u);

    /* STOPPED fails each outstanding request exactly once (Verifies: SWS_CANIF_00739). */
    controller(CAN_CS_STARTED);
    assert(set_pdu(0u, CANIF_ONLINE) == E_OK);
    assert(send(4u) == E_OK);
    controller(CAN_CS_STOPPED);
    assert(successes == 1u && failures == 1u);
    assert(Can_HostFlush() == ECU_OK);
    tx_confirmation(0u);
    assert(successes == 1u && failures == 1u);

    /* BusOff is dispatched even after the Tx path went offline and sets TX_OFFLINE
     * (Verifies: SWS_CANIF_00118, 00724, 00866). */
    controller(CAN_CS_STARTED);
    assert(set_pdu(0u, CANIF_OFFLINE) == E_OK);
    bus_off_indication(0u);
    assert(bus_offs == 1u && get_pdu(0u, &pdu) == E_OK && pdu == CANIF_TX_OFFLINE);
    /* Verifies: SWS_CANIF_00865: SLEEP (reachable from STOPPED) sets OFFLINE. */
    controller(CAN_CS_STOPPED);
    assert(set_controller(0u, CAN_CS_SLEEP) == E_OK);
    assert(get_pdu(0u, &pdu) == E_OK && pdu == CANIF_OFFLINE);
    Can_MainFunction_Wakeup();

    /* An invalid set after a valid one leaves CanIf uninitialized, not on the
     * previous set (Verifies: SWS_CANIF_00661). */
    init(&invalid);
    uninitialized();
    init(&canif);
    assert(get_pdu(0u, &pdu) == E_OK && pdu == CANIF_OFFLINE);
    init(NULL_PTR);
    uninitialized();
    /* Verifies: SWS_CANIF_91002: DeInit returns to the uninitialized state. */
    init(&canif);
    deinit();
    uninitialized();
    Can_DeInit();
    return 0;
}
