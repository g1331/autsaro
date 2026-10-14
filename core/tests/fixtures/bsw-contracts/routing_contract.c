/** Independently authored R24-11 PduR/LSduR consumer: actual routers, recorded neighbours.
 * Equal numbers recur across namespaces (5 for upper modules and CanIf, 35 for PduR's IF
 * and TP lower routes), so only direction and module identity tell the routes apart; each
 * translated ID differs from its source. CanIf_Transmit is wrapped.
 */
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_Dcm.h"
#include "PduR_CanIf.h"
#include "PduR_CanTp.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "LSduR_CanTp.h"
#include "LSduR_CanIf.h"
#include "Ecu_Config.h"
#include <assert.h>

/* Standard signatures over the imported ComStack types, in order (Verifies: SWS_PduR_00334,
 * 00341, 00615, 00617, 00406, 00362, 00365, 00369, 00507, 00512, 00375, 00518, 00381;
 * Verifies: CP_SWS_LSduR_91006, 91008, 91009, 91011, 91013, 91014). Input pointers are
 * const (Verifies[PduR]: SWS_BSW_00186; Verifies[LSduR]: SWS_BSW_00186). */
static void (*const pdur_init)(const PduR_PBConfigType *) = PduR_Init;
static PduR_PBConfigIdType (*const pdur_id)(void) = PduR_GetConfigurationId;
static void (*const pdur_enable)(PduR_RoutingPathGroupIdType) = PduR_EnableRouting;
static void (*const pdur_disable)(PduR_RoutingPathGroupIdType, boolean) = PduR_DisableRouting;
static Std_ReturnType (*const pdur_com)(PduIdType, const PduInfoType *) = PduR_ComTransmit;
static Std_ReturnType (*const pdur_dcm)(PduIdType, const PduInfoType *) = PduR_DcmTransmit;
static void (*const pdur_indication)(PduIdType, const PduInfoType *) = PduR_CanIfRxIndication;
static void (*const pdur_conf)(PduIdType, Std_ReturnType) = PduR_CanIfTxConfirmation;
static Std_ReturnType (*const pdur_trigger)(PduIdType, PduInfoType *) = PduR_CanIfTriggerTransmit;
static BufReq_ReturnType (*const pdur_start)(PduIdType, const PduInfoType *, PduLengthType,
                                             PduLengthType *) = PduR_CanTpStartOfReception;
static BufReq_ReturnType (*const pdur_copy_rx)(PduIdType, const PduInfoType *,
                                               PduLengthType *) = PduR_CanTpCopyRxData;
static void (*const pdur_tp_rx)(PduIdType, Std_ReturnType) = PduR_CanTpRxIndication;
static BufReq_ReturnType (*const pdur_copy_tx)(PduIdType, const PduInfoType *, const RetryInfoType *,
                                               PduLengthType *) = PduR_CanTpCopyTxData;
static void (*const pdur_tp_conf)(PduIdType, Std_ReturnType) = PduR_CanTpTxConfirmation;
static void (*const lsdur_init)(const LSduR_PBConfigType *) = LSduR_Init;
static LSduR_PBConfigIdType (*const lsdur_id)(void) = LSduR_GetConfigurationId;
static Std_ReturnType (*const lsdur_pdur)(PduIdType, const PduInfoType *) = LSduR_PduRTransmit;
static Std_ReturnType (*const lsdur_cantp)(PduIdType, const PduInfoType *) = LSduR_CanTpTransmit;
static void (*const lsdur_rx)(PduIdType, const PduInfoType *) = LSduR_CanIfRxIndication;
static void (*const lsdur_conf)(PduIdType, Std_ReturnType) = LSduR_CanIfTxConfirmation;
static Std_ReturnType (*const lsdur_trigger)(PduIdType, PduInfoType *) = LSduR_CanIfTriggerTransmit;
/* Verifies: SWS_PduR_00771, 00654; Verifies: CP_SWS_LSduR_91004: identifiers derived from uint16. */
typedef char configuration_ids[(sizeof(PduR_PBConfigIdType) == sizeof(uint16) &&
                                sizeof(PduR_RoutingPathGroupIdType) == sizeof(uint16) &&
                                sizeof(LSduR_PBConfigIdType) == sizeof(uint16))
                                   ? 1
                                   : -1];

const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};

enum { CANIF_TX, CANTP_TX, COM_RX, COM_CONF, COM_TRIGGER, CANTP_CONF, START, COPY_RX, TP_RX,
       COPY_TX, TP_CONF, CALLS };
static unsigned calls[CALLS];
static PduIdType last_id;
static const PduInfoType *last_info;
static Std_ReturnType last_result;
static Std_ReturnType lower_result = E_OK;
static uint8 bytes[64];
static PduInfoType info = {bytes, NULL_PTR, 64u};
static PduLengthType available;
static const RetryInfoType retry = {TP_DATACONF, 0u};

static void record(unsigned which, PduIdType id, const PduInfoType *pdu, Std_ReturnType result) {
    ++calls[which];
    last_id = id;
    last_info = pdu;
    last_result = result;
}
static unsigned total(void) {
    unsigned which;
    unsigned sum = 0u;
    for (which = 0u; which < CALLS; ++which) {
        sum += calls[which];
    }
    return sum;
}
/* Asserts that exactly one call happened since the last check, and on which interface. */
static void once(unsigned which, PduIdType id) {
    unsigned other;
    for (other = 0u; other < CALLS; ++other) {
        assert(calls[other] == ((other == which) ? 1u : 0u));
        calls[other] = 0u;
    }
    assert(last_id == id);
}
static void none(void) { assert(total() == 0u); }

Std_ReturnType __wrap_CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    record(CANIF_TX, TxPduId, PduInfoPtr, E_OK);
    return lower_result;
}
static Std_ReturnType cantp_transmit(PduIdType id, const PduInfoType *pdu) {
    record(CANTP_TX, id, pdu, E_OK);
    return lower_result;
}
static void com_rx(PduIdType id, const PduInfoType *pdu) { record(COM_RX, id, pdu, E_OK); }
static void com_conf(PduIdType id, Std_ReturnType result) {
    record(COM_CONF, id, NULL_PTR, result);
}
static Std_ReturnType com_trigger(PduIdType id, PduInfoType *pdu) {
    record(COM_TRIGGER, id, pdu, E_OK);
    return lower_result;
}
static void cantp_conf(PduIdType id, Std_ReturnType result) {
    record(CANTP_CONF, id, NULL_PTR, result);
}
static BufReq_ReturnType start(PduIdType id, const PduInfoType *pdu, PduLengthType length,
                               PduLengthType *size) {
    record(START, id, pdu, E_OK);
    *size = length;
    return BUFREQ_OK;
}
static BufReq_ReturnType copy_rx(PduIdType id, const PduInfoType *pdu, PduLengthType *size) {
    record(COPY_RX, id, pdu, E_OK);
    *size = 7u;
    return BUFREQ_E_BUSY;
}
static void tp_rx(PduIdType id, Std_ReturnType result) { record(TP_RX, id, NULL_PTR, result); }
static BufReq_ReturnType copy_tx(PduIdType id, const PduInfoType *pdu, const RetryInfoType *again,
                                 PduLengthType *size) {
    assert(again == &retry);
    record(COPY_TX, id, pdu, E_OK);
    *size = 3u;
    return BUFREQ_E_OVFL;
}
static void tp_conf(PduIdType id, Std_ReturnType result) { record(TP_CONF, id, NULL_PTR, result); }

/* Post-build structures with implementation-specific content
 * (Verifies: SWS_PduR_00743; Verifies: CP_SWS_LSduR_91003). */
static const LSduR_RxRouteType ls_rx[] = {{5u, 25u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 35u, 5u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit},
    {LSDUR_UP_CANTP, 5u, 6u, cantp_conf, NULL_PTR}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 2u};
static const PduR_RxRouteType pdur_rx[] = {{25u, 5u, com_rx}};
/* The CanTp route comes first: IF callbacks must skip it although its lower ID matches. */
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_DCM, 5u, PDUR_LO_CANTP, 35u, cantp_transmit, NULL_PTR, NULL_PTR},
    {PDUR_UP_COM, 5u, PDUR_LO_LSDUR, 35u, LSduR_PduRTransmit, com_conf, com_trigger}};
static const PduR_TpRouteType tp_routes[] = {
    {35u, 45u, 35u, 55u, start, copy_rx, tp_rx, copy_tx, tp_conf}};
static const PduR_PBConfigType pdur = {23u, pdur_rx, 1u, pdur_tx, 2u, tp_routes, 1u};
/* Valid but different: a later Init must not replace the published configuration. */
static const PduR_RxRouteType other_rx[] = {{25u, 9u, com_rx}};
static const PduR_PBConfigType other = {24u, other_rx, 1u, NULL_PTR, 0u, NULL_PTR, 0u};
static const LSduR_PBConfigType ls_other = {30u, NULL_PTR, 0u, NULL_PTR, 0u};
/* Invalid: equal lower IDs within one namespace, or an IF route without confirmation. */
static const PduR_TxRouteType clash_tx[] = {
    {PDUR_UP_COM, 1u, PDUR_LO_LSDUR, 35u, LSduR_PduRTransmit, com_conf, NULL_PTR},
    {PDUR_UP_COM, 2u, PDUR_LO_LSDUR, 35u, LSduR_PduRTransmit, com_conf, NULL_PTR}};
static const PduR_PBConfigType clash = {25u, NULL_PTR, 0u, clash_tx, 2u, NULL_PTR, 0u};
static const PduR_TxRouteType silent_tx[] = {
    {PDUR_UP_COM, 1u, PDUR_LO_LSDUR, 35u, LSduR_PduRTransmit, NULL_PTR, NULL_PTR}};
static const PduR_PBConfigType silent = {26u, NULL_PTR, 0u, silent_tx, 1u, NULL_PTR, 0u};
static const LSduR_TxRouteType ls_clash_tx[] = {
    {LSDUR_UP_PDUR, 1u, 5u, cantp_conf, NULL_PTR}, {LSDUR_UP_CANTP, 2u, 5u, cantp_conf, NULL_PTR}};
static const LSduR_PBConfigType ls_clash = {31u, NULL_PTR, 0u, ls_clash_tx, 2u};

/* Every interface through its standard signature; nothing is routed before Init. */
static void uninitialized(void) {
    PduInfoType pdu = info;
    assert(pdur_id() == 0u && lsdur_id() == 0u);
    assert(pdur_com(5u, &info) == E_NOT_OK && pdur_dcm(5u, &info) == E_NOT_OK);
    assert(lsdur_pdur(35u, &info) == E_NOT_OK && lsdur_cantp(5u, &info) == E_NOT_OK);
    lsdur_rx(5u, &info);
    lsdur_conf(5u, E_OK);
    assert(lsdur_trigger(5u, &pdu) == E_NOT_OK);
    pdur_indication(25u, &info);
    pdur_conf(35u, E_OK);
    assert(pdur_trigger(35u, &pdu) == E_NOT_OK);
    assert(pdur_start(35u, &info, 8u, &available) == BUFREQ_E_NOT_OK);
    assert(pdur_copy_rx(35u, &info, &available) == BUFREQ_E_NOT_OK);
    pdur_tp_rx(35u, E_OK);
    assert(pdur_copy_tx(35u, &info, &retry, &available) == BUFREQ_E_NOT_OK);
    pdur_tp_conf(35u, E_OK);
    pdur_enable(0u);
    pdur_disable(0u, FALSE);
    none();
}

int main(void) {
    PduInfoType pdu = info;
    /* UNINIT after power up and while every Init was invalid: no routing
     * (Verifies: SWS_PduR_00325, 00330; Verifies: CP_SWS_LSduR_00027, 00028, 00031). */
    uninitialized();
    pdur_init(NULL_PTR);
    pdur_init(&clash);
    pdur_init(&silent);
    lsdur_init(NULL_PTR);
    lsdur_init(&ls_clash);
    uninitialized();
    /* A valid Init makes the router ONLINE and identifies its configuration
     * (Verifies: SWS_PduR_00326, 00280; Verifies: CP_SWS_LSduR_00029, 00036;
     * Verifies[PduR]: SWS_BSW_00071; Verifies[LSduR]: SWS_BSW_00071). */
    lsdur_init(&ls);
    pdur_init(&pdur);
    assert(PduR_GetConfigurationId() == 23u && LSduR_GetConfigurationId() == 29u);
    /* No ONLINE -> ONLINE transition: a second Init keeps the published configuration. */
    PduR_Init(&other);
    LSduR_Init(&ls_other);
    assert(PduR_GetConfigurationId() == 23u && LSduR_GetConfigurationId() == 29u);

    /* Upper transmit: module identity selects the route, the lower ID is translated and
     * the info pointer and lower result pass through unchanged, unbuffered and unchecked
     * (Verifies: SWS_PduR_00328, 00160, 00161, 00766, 00629, 00745, 00625, 00626, 00634,
     * 00432; Verifies: CP_SWS_LSduR_00030, 00005, 00006, 00007, 00015, 00017, 00018, 00019). */
    assert(PduR_ComTransmit(5u, &info) == E_OK);
    once(CANIF_TX, 5u);
    assert(last_info == &info);
    assert(PduR_DcmTransmit(5u, &info) == E_OK);
    once(CANTP_TX, 35u);
    assert(last_info == &info);
    assert(LSduR_CanTpTransmit(5u, &info) == E_OK);
    once(CANIF_TX, 6u);
    lower_result = E_NOT_OK;
    assert(PduR_ComTransmit(5u, &info) == E_NOT_OK);
    once(CANIF_TX, 5u);
    assert(PduR_DcmTransmit(5u, &info) == E_NOT_OK);
    once(CANTP_TX, 35u);
    lower_result = E_OK;
    /* An unknown length (0) reaches CanTp unchanged (Verifies: SWS_PduR_00822). */
    {
        const PduInfoType unknown = {bytes, NULL_PTR, 0u};
        assert(PduR_DcmTransmit(5u, &unknown) == E_OK);
        once(CANTP_TX, 35u);
        assert(last_info == &unknown);
    }

    /* Lower callbacks: CanIf IDs reach the upper module of their own route only, with
     * results passed through (Verifies: SWS_PduR_00621, 00744, 00627, 00430, 00661;
     * Verifies: CP_SWS_LSduR_00011, 00012, 00016, 00022, 00023). */
    LSduR_CanIfRxIndication(5u, &info);
    once(COM_RX, 5u);
    assert(last_info == &info);
    LSduR_CanIfTxConfirmation(5u, E_NOT_OK);
    once(COM_CONF, 5u);
    assert(last_result == E_NOT_OK);
    LSduR_CanIfTxConfirmation(6u, E_OK);
    once(CANTP_CONF, 5u);
    assert(last_result == E_OK);
    assert(LSduR_CanIfTriggerTransmit(5u, &pdu) == E_OK);
    once(COM_TRIGGER, 5u);
    assert(last_info == &pdu);
    lower_result = E_NOT_OK;
    assert(LSduR_CanIfTriggerTransmit(5u, &pdu) == E_NOT_OK);
    once(COM_TRIGGER, 5u);
    lower_result = E_OK;
    /* The CanTp route has no trigger; IF callbacks for IDs of no IF route do nothing. */
    assert(LSduR_CanIfTriggerTransmit(6u, &pdu) == E_NOT_OK);
    PduR_CanIfTxConfirmation(5u, E_OK);
    assert(PduR_CanIfTriggerTransmit(5u, &pdu) == E_NOT_OK);
    none();
    assert(PduR_CanIfTriggerTransmit(35u, &pdu) == E_OK);
    once(COM_TRIGGER, 5u);

    /* Transport: CanTp's PduR IDs reach Dcm with Dcm's IDs and buffer results unchanged
     * (Verifies: SWS_PduR_00673, 00549, 00623, 00428, 00429, 00207, 00299, 00676, 00301). */
    available = 0u;
    assert(PduR_CanTpStartOfReception(35u, &info, 12u, &available) == BUFREQ_OK);
    once(START, 45u);
    assert(available == 12u && last_info == &info);
    /* An unknown total length (0) is forwarded as 0 (Verifies: SWS_PduR_00821). */
    assert(PduR_CanTpStartOfReception(35u, &info, 0u, &available) == BUFREQ_OK);
    once(START, 45u);
    assert(available == 0u);
    assert(PduR_CanTpCopyRxData(35u, &info, &available) == BUFREQ_E_BUSY);
    once(COPY_RX, 45u);
    assert(available == 7u);
    PduR_CanTpRxIndication(35u, E_NOT_OK);
    once(TP_RX, 45u);
    assert(last_result == E_NOT_OK);
    assert(PduR_CanTpCopyTxData(35u, &info, &retry, &available) == BUFREQ_E_OVFL);
    once(COPY_TX, 55u);
    assert(available == 3u);
    PduR_CanTpTxConfirmation(35u, E_OK);
    once(TP_CONF, 55u);
    PduR_CanIfTxConfirmation(35u, E_NOT_OK);
    once(COM_CONF, 5u);

    /* Only configured routes are used; wrong routes and null data cause no call and no
     * partial success (Verifies: SWS_PduR_00162; Verifies: CP_SWS_LSduR_00008). */
    assert(PduR_ComTransmit(6u, &info) == E_NOT_OK && PduR_ComTransmit(5u, NULL_PTR) == E_NOT_OK);
    assert(PduR_DcmTransmit(6u, &info) == E_NOT_OK && PduR_DcmTransmit(5u, NULL_PTR) == E_NOT_OK);
    assert(LSduR_PduRTransmit(5u, &info) == E_NOT_OK && LSduR_PduRTransmit(35u, NULL_PTR) == E_NOT_OK);
    assert(LSduR_CanTpTransmit(35u, &info) == E_NOT_OK);
    LSduR_CanIfRxIndication(6u, &info);
    LSduR_CanIfRxIndication(5u, NULL_PTR);
    LSduR_CanIfTxConfirmation(7u, E_OK);
    assert(LSduR_CanIfTriggerTransmit(7u, &pdu) == E_NOT_OK);
    assert(LSduR_CanIfTriggerTransmit(5u, NULL_PTR) == E_NOT_OK);
    PduR_CanIfRxIndication(5u, &info);
    PduR_CanIfRxIndication(25u, NULL_PTR);
    assert(PduR_CanIfTriggerTransmit(35u, NULL_PTR) == E_NOT_OK);
    assert(PduR_CanTpStartOfReception(45u, &info, 8u, &available) == BUFREQ_E_NOT_OK);
    assert(PduR_CanTpCopyRxData(5u, &info, &available) == BUFREQ_E_NOT_OK);
    PduR_CanTpRxIndication(55u, E_OK);
    assert(PduR_CanTpCopyTxData(55u, &info, &retry, &available) == BUFREQ_E_NOT_OK);
    PduR_CanTpTxConfirmation(45u, E_OK);
    none();
    /* Routing path groups do not exist in the selected configuration: an unknown group
     * causes no action and paths without a group stay enabled (Verifies: SWS_PduR_00647,
     * 00716, 00891). */
    PduR_EnableRouting(0u);
    PduR_DisableRouting(0u, TRUE);
    PduR_DisableRouting(65535u, FALSE);
    none();
    /* The next legal request after every rejection still succeeds exactly once. */
    assert(PduR_ComTransmit(5u, &info) == E_OK);
    once(CANIF_TX, 5u);
    LSduR_CanIfRxIndication(5u, &info);
    once(COM_RX, 5u);
    return 0;
}
