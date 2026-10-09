/** Independent fixed R24-11 mode/demand oracle with actual CAN/COM chain.
 * Observers/configured user callouts never replace standard module functions.
 */
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Dcm.h"
#include "ComM_Internal.h"
#include "BswM.h"
#include "BswM_ComM.h"
#include "Ecu_HostBusSM.h"
#include "Can.h"
#include "SchM_Can.h"
#include "Com.h"
#include "Com_Internal.h"
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "LSduR_CanIf.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>
static unsigned indications[3];
static unsigned actions;
static unsigned frames;
static unsigned errors;
static unsigned receptions;
static unsigned action_depth;
static boolean postpone;
static Std_ReturnType action_result;
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
    assert(id == 0x456u && length == 4u && data[0] == 42u && data[1] == 0u);
    ++frames;
    return ECU_OK;
}
static void receive(CbkHandleIdType handle) {
    assert(handle == 17u);
    ++receptions;
}
static void timeout(CbkHandleIdType handle) { assert(handle == 17u); }
static void indication(NetworkHandleType Channel, ComM_ModeType ComMode) {
    ComM_ModeType actual = 99u;
    assert(Channel == 7u && ComMode <= 2u);
    assert(Ecu_HostBusSM_GetCurrentComMode(Channel, &actual) == E_OK && actual == ComMode);
    ++indications[ComMode];
}
static Std_ReturnType action(NetworkHandleType Network, ComM_ModeType Mode) {
    ++action_depth;
    assert(action_depth == 1u); /* IMMEDIATE reentrant input is postponed, not recursive. */
    ++actions;
    if (postpone == TRUE) {
        postpone = FALSE;
        BswM_ComM_CurrentMode(Network, COMM_NO_COMMUNICATION);
    }
    action_result = Ecu_HostBusSM_ApplyMode(Network, Mode);
    --action_depth;
    return action_result;
}
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code) {
    assert(module == 60u && instance == 0u && api == 0x49u && code == 70u);
    ++errors;
    return E_NOT_OK;
}
static const Det_ErrorHookType error_hooks[] = {error};
static const Det_ConfigType det = {NULL_PTR, 0u, error_hooks, 1u};
static const Com_PduConfigType pdus[] = {{1u, 10u, TRUE, 7u, 30u, 17u},
                                         {2u, 11u, FALSE, 9u, 0u, 0u}};
static const Com_ConfigType com = {pdus, 2u, 0u, receive, timeout, PduR_ComTransmit};
static const PduR_RxRouteType pdur_rx[] = {{31u, 1u, Com_RxIndication}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 2u, 41u, LSduR_PduRTransmit, Com_TxConfirmation, Com_TriggerTransmit}};
static const PduR_PBConfigType pdur = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 1u};
static const CanIf_RxPduConfigType canif_rx[] = {{0x123u, 21u, 4u}};
static const CanIf_TxPduConfigType canif_tx[] = {{0x456u, 51u, 4u}};
static const CanIf_ConfigType canif = {canif_rx,
                                       1u,
                                       canif_tx,
                                       1u,
                                       Ecu_HostBusSM_ControllerModeIndication,
                                       Ecu_HostBusSM_ControllerBusOff};
static const Can_ConfigType can = {sink};
static const ComM_UserHandleType users[] = {17u, 43u};
static const ComM_ConfigType comm = {
    7u, users, 2u, 3u, Ecu_HostBusSM_RequestComMode, Ecu_HostBusSM_GetCurrentComMode, indication};
static const Ecu_HostBusSM_ConfigType cdd = {7u, 0u};
static const BswM_ConfigType bswm = {7u, COMM_NO_COMMUNICATION, action};
static void current(ComM_ModeType expected) {
    ComM_ModeType value = 99u;
    assert(ComM_GetCurrentComMode(17u, &value) == E_OK && value == expected);
}
static void buffer(void) {
    uint32 value = 0u;
    uint32 sent = 42u;
    assert(Com_ReceiveSignal(10u, &value) == E_OK && value == 42u);
    assert(Com_SendSignal(11u, &sent) == E_OK);
}
int main(void) {
    void (*init)(const ComM_ConfigType *) = ComM_Init;
    Std_ReturnType (*request)(ComM_UserHandleType, ComM_ModeType) = ComM_RequestComMode;
    void (*allowed)(NetworkHandleType, boolean) = ComM_CommunicationAllowed;
    void (*active)(NetworkHandleType) = ComM_DCM_ActiveDiagnostic;
    void (*inactive)(NetworkHandleType) = ComM_DCM_InactiveDiagnostic;
    void (*bus_indication)(NetworkHandleType, ComM_ModeType) = ComM_BusSM_ModeIndication;
    void (*bswm_init)(const BswM_ConfigType *) = BswM_Init;
    void (*bswm_mode)(NetworkHandleType, ComM_ModeType) = BswM_ComM_CurrentMode;
    Std_ReturnType (*bus_request)(NetworkHandleType, ComM_ModeType) = Ecu_HostBusSM_RequestComMode;
    Std_ReturnType (*bus_get)(NetworkHandleType, ComM_ModeType *) = Ecu_HostBusSM_GetCurrentComMode;
    ComM_ModeType value = 99u;
    ComM_InitStatusType status = COMM_INIT;
    Can_ErrorStateType error_state = CAN_ERRORSTATE_ACTIVE;
    uint8 bytes[8] = {42u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    const PduInfoType info = {bytes, NULL_PTR, 4u};
    unsigned saved;
    unsigned i;
    assert(sizeof(ComM_ModeType) == 1u && sizeof(ComM_UserHandleType) == 2u);
    assert(COMM_NO_COMMUNICATION == 0u && COMM_SILENT_COMMUNICATION == 1u &&
           COMM_FULL_COMMUNICATION == 2u && COMM_NOT_USED_USER_ID == 65535u);
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_UNINIT);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_NOT_OK);
    assert(ComM_GetCurrentComMode(17u, &value) == E_NOT_OK && value == 99u);
    assert(bus_get(7u, &value) == E_NOT_OK && value == 99u);
    bswm_mode(7u, COMM_FULL_COMMUNICATION);
    assert(actions == 0u);
    Det_Init(&det);
    Com_Init(&com);
    Com_IpduGroupStart(0u, TRUE);
    Com_EnableReceptionDM(0u);
    PduR_Init(&pdur);
    LSduR_Init(&ls);
    CanIf_Init(&canif);
    Can_Init(&can);
    Ecu_HostBusSM_Init(&cdd);
    bswm_init(&bswm);
    init(&comm);
    assert(actions == 0u && indications[0] == 0u); /* No init mode notification. */
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_INIT);
    assert(ComM_GetStatus(NULL_PTR) == E_NOT_OK);
    assert(request(65535u, COMM_FULL_COMMUNICATION) == E_NOT_OK);
    assert(request(17u, COMM_SILENT_COMMUNICATION) == E_NOT_OK);
    assert(request(17u, 3u) == E_NOT_OK);
    assert(ComM_GetRequestedComMode(99u, &value) == E_NOT_OK && value == 99u);
    assert(ComM_GetRequestedComMode(17u, NULL_PTR) == E_NOT_OK);
    assert(ComM_GetMaxComMode(17u, &value) == E_OK && value == COMM_FULL_COMMUNICATION);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING);
    current(COMM_NO_COMMUNICATION);
    assert(indications[2] == 0u);
    assert(request(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_NO_PENDING_REQUEST);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK);
    allowed(99u, TRUE);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING);
    allowed(7u, TRUE);
    current(COMM_NO_COMMUNICATION); /* Real driver STARTED, but no PDU ONLINE indication yet. */
    assert(indications[2] == 0u && actions == 0u);
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED); /* minimum tick 1 of 3 */
    assert(indications[2] == 0u);
    Can_MainFunction_Wakeup();
    current(COMM_FULL_COMMUNICATION);
    assert(indications[2] == 1u && actions == 1u);
    assert(CanIf_HostRxIndication(0x123u, 4u, bytes, 0u) == ECU_OK && receptions == 1u);
    buffer();
    Ecu_ComMainFunctionTx();
    assert(Can_HostFlush() == ECU_OK && frames == 1u);
    assert(request(43u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(request(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_GetRequestedComMode(17u, &value) == E_OK && value == COMM_NO_COMMUNICATION);
    active(7u);
    assert(request(43u, COMM_NO_COMMUNICATION) == E_OK);
    allowed(7u, FALSE); /* In FULL this cannot act as a stop command. */
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED); /* tick 2 */
    assert(ComM_RunChannel() ==
           COMM_FULL_COM_NETWORK_REQUESTED); /* tick 3, diagnosis still active */
    inactive(99u);                           /* Unknown channel cannot release mapped diagnosis. */
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    inactive(7u);
    assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
    for (i = 0u; i < 10u; ++i) {
        assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
        current(COMM_FULL_COMMUNICATION); /* NM NONE/CDD must not invent automatic shutdown. */
    }
    active(7u); /* Re-entering NETWORK_REQUESTED starts a fresh minimum timer. */
    inactive(7u);
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK); /* repeated FULL does not reset timer */
    assert(request(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
    current(COMM_FULL_COMMUNICATION);
    ComM_DeInit();
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_INIT);
    /* Actual bus-off with persistent FULL: every subsequent main remains factual. */
    allowed(7u, TRUE);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK);
    Can_SetMode(CAN_BUS_OFF);
    current(COMM_NO_COMMUNICATION);
    saved = indications[2];
    for (i = 0u; i < 10u; ++i) {
        assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
        current(COMM_NO_COMMUNICATION);
        assert(Can_GetControllerErrorState(0u, &error_state) == E_OK &&
               error_state == CAN_ERRORSTATE_BUSOFF);
        assert(indications[2] == saved);
        buffer(); /* lower loss never stops COM groups or buffered access */
    }
    assert(CanIf_Transmit(51u, &info) == E_NOT_OK && errors == 1u);
    Can_SetMode(CAN_STARTED); /* Explicit existing controlled-host recovery. */
    current(COMM_FULL_COMMUNICATION);
    assert(indications[2] == saved + 1u);
    /* Normal owner STOP precursor: release persistent user, then actual provider stop. */
    assert(request(17u, COMM_NO_COMMUNICATION) == E_OK);
    saved = indications[0];
    assert(bus_request(7u, COMM_NO_COMMUNICATION) == E_OK);
    current(COMM_NO_COMMUNICATION);
    assert(indications[0] == saved); /* No completion before real polling. */
    assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
    Can_MainFunction_Wakeup();
    for (i = 0u; i < 10u; ++i) {
        assert(ComM_RunChannel() == COMM_NO_COM_NO_PENDING_REQUEST);
        current(COMM_NO_COMMUNICATION);
        buffer();
    }
    /* Failed FULL direction action cannot start stopped driver or fabricate FULL. */
    bswm_mode(7u, COMM_FULL_COMMUNICATION);
    assert(action_result == E_NOT_OK);
    current(COMM_NO_COMMUNICATION);
    assert(bus_request(7u, COMM_SILENT_COMMUNICATION) == E_OK);
    Can_MainFunction_Wakeup();
    current(COMM_SILENT_COMMUNICATION);
    assert(CanIf_HostRxIndication(0x123u, 4u, bytes, 0u) == ECU_OK && receptions == 2u);
    assert(CanIf_Transmit(51u, &info) == E_NOT_OK && errors == 2u);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK);
    current(COMM_FULL_COMMUNICATION);
    saved = actions;
    bswm_mode(99u, COMM_NO_COMMUNICATION);
    bswm_mode(7u, 3u);
    bus_indication(99u, COMM_NO_COMMUNICATION);
    bus_indication(7u, 99u);
    assert(actions == saved);
    bswm_mode(7u, COMM_NO_COMMUNICATION); /* Real IMMEDIATE rule changes lower directions. */
    current(COMM_NO_COMMUNICATION);
    assert(CanIf_HostRxIndication(0x123u, 4u, bytes, 0u) != ECU_OK && receptions == 2u);
    buffer();
    bswm_mode(7u, COMM_FULL_COMMUNICATION);
    current(COMM_FULL_COMMUNICATION);
    postpone = TRUE;
    saved = actions;
    bswm_mode(7u, COMM_FULL_COMMUNICATION);
    assert(actions == saved + 2u && action_depth == 0u);
    current(COMM_NO_COMMUNICATION); /* Postponed NO runs after current FULL action. */
    bswm_mode(7u, COMM_FULL_COMMUNICATION);
    BswM_Deinit();
    saved = actions;
    bswm_mode(7u, COMM_NO_COMMUNICATION);
    assert(actions == saved);
    current(COMM_FULL_COMMUNICATION);
    bswm_init(&bswm);
    /* ComM restart resets demand/Allowed; it must obtain actual mode via provider. */
    saved = indications[0];
    init(&comm);
    current(COMM_NO_COMMUNICATION);  /* Re-init requests actual NO, not inherited FULL. */
    assert(indications[0] == saved); /* Default NO is not a mode change notification. */
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING);
    allowed(7u, TRUE);
    current(COMM_NO_COMMUNICATION); /* Old STOP pending rejects the new START request. */
    Can_MainFunction_Wakeup();
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    current(COMM_NO_COMMUNICATION);
    assert(CanIf_HostRxIndication(0x123u, 4u, bytes, 0u) != ECU_OK);
    Can_MainFunction_Wakeup();
    current(COMM_FULL_COMMUNICATION);
    assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(request(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(bus_request(7u, COMM_NO_COMMUNICATION) == E_OK);
    Can_MainFunction_Wakeup();
    ComM_DeInit();
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_UNINIT);
    assert(request(17u, COMM_FULL_COMMUNICATION) == E_NOT_OK);
    Ecu_HostBusSM_DeInit();
    value = 99u;
    assert(bus_get(7u, &value) == E_NOT_OK && value == 99u);
    BswM_Deinit();
    Com_DeInit();
    CanIf_DeInit();
    Can_DeInit();
    return 0;
}
