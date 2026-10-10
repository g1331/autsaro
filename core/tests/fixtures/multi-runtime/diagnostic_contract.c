/** Independent physical UDS/TP expectations through real driver and mode chain. */
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "CanTp.h"
#include "PduR.h"
#include "PduR_CanTp.h"
#include "PduR_Dcm.h"
#include "LSduR.h"
#include "LSduR_CanIf.h"
#include "LSduR_CanTp.h"
#include "ComM.h"
#include "ComM_Internal.h"
#include "ComM_BswM.h"
#include "BswM.h"
#include "Ecu_HostBusSM.h"
#include "Ecu_Config.h"
#include "Can.h"
#include "SchM_Can.h"
#include "SchM_CanTp.h"
#include "SchM_Dcm.h"
#include "Det.h"
#include <assert.h>
#include <string.h>
static unsigned count;
static uint8 output[128][8];
#if defined(CAN_DIAGNOSTIC_QUEUED)
static uint64_t output_tokens[128];
#endif
static unsigned reads;
static unsigned reports;
const EcuPolicyConfig Ecu_Policy = {
#if defined(CAN_DIAGNOSTIC_QUEUED)
    ECU_TX_QUEUED,
#else
    ECU_TX_SYNCHRONOUS,
#endif
    ECU_RX_BEFORE_DEADLINE, 0u, 0u, ECU_WAIT_ABORT, 0u, 0u, NULL_PTR, 0u, 50u, 5000u};
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    assert(id == 0x708u && length == 8u && count < 128u);
#if defined(CAN_DIAGNOSTIC_QUEUED)
    output_tokens[count] = Can_HostTransmitToken();
    assert(output_tokens[count] != UINT64_C(0));
#endif
    memcpy(output[count], data, 8u);
    ++count;
    return ECU_OK;
}
static Std_ReturnType read_did(uint8 *data) {
    data[0] = 0x12u;
    data[1] = 0x34u;
    data[2] = 0x56u;
    data[3] = 0x78u;
    ++reads;
    return E_OK;
}
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code) {
    assert(instance == 0u);
    assert(((module == 60u) && (api == 0x49u) && (code == 70u)) ||
           ((module == 35u) && (api == 0x42u) && ((code == 0xb0u) || (code == 0x70u))) ||
           ((module == 35u) && (api == 0x4cu) && (code == 0xa0u)));
    ++reports;
    return E_OK;
}
static void mode(NetworkHandleType channel, ComM_ModeType value) {
    if (value == COMM_FULL_COMMUNICATION) {
        Dcm_ComM_FullComModeEntered(channel);
    } else if (value == COMM_SILENT_COMMUNICATION) {
        Dcm_ComM_SilentComModeEntered(channel);
    } else {
        Dcm_ComM_NoComModeEntered(channel);
    }
}
static const Det_ErrorHookType hooks[] = {error};
static const Det_ConfigType det = {NULL_PTR, 0u, hooks, 1u};
static const PduR_TpRouteType tp_routes[] = {{71u, 17u, 72u, 18u, Dcm_StartOfReception,
                                              Dcm_CopyRxData, Dcm_TpRxIndication, Dcm_CopyTxData,
                                              Dcm_TpTxConfirmation}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_DCM, 81u, 12u, CanTp_Transmit, Dcm_TpTxConfirmation, NULL_PTR}};
static const PduR_PBConfigType pdur = {23u, NULL_PTR, 0u, pdur_tx, 1u, tp_routes, 1u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, CanTp_RxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_CANTP, 41u, 51u, CanTp_TxConfirmation, NULL_PTR}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 1u};
static const CanIf_RxPduConfigType canif_rx[] = {{0x700u, 21u, 8u}};
static const CanIf_TxPduConfigType canif_tx[] = {{0x708u, 51u, 8u}};
static const CanIf_ConfigType canif = {canif_rx,
                                       1u,
                                       canif_tx,
                                       1u,
                                       Ecu_HostBusSM_ControllerModeIndication,
                                       Ecu_HostBusSM_ControllerBusOff,
                                       0u,
                                       0u,
                                       0u,
                                       0u};
static const Can_ConfigType can = {sink};
static const ComM_UserHandleType users[] = {43u};
static const ComM_ConfigType comm = {
    7u, users, 1u, 3u, Ecu_HostBusSM_RequestComMode, Ecu_HostBusSM_GetCurrentComMode, mode, 3u};
static const Ecu_HostBusSM_ConfigType cdd = {7u, 0u};
static const BswM_ConfigType bswm = {7u, COMM_NO_COMMUNICATION, Ecu_HostBusSM_ApplyMode};
static uint8 authentication_mode = 255u;
static void publish_authentication(uint8 state) { authentication_mode = state; }
static const Dcm_ConfigType dcm = {17u, 18u,   7u,  0x1234u, read_did, 50u,
                                   50u, 5000u, 20u, 256u,    81u,      publish_authentication};
static const CanTp_ConfigType tp = {11u, 12u, 31u, 41u, 256u, 5u,  5u, 5u,
                                    5u,  5u,  1u,  2u,  0u,   71u, 72u};
static void receive(const uint8 bytes[8]) {
    assert(Can_Inject(0x700u, 8u, bytes, 0u) == ECU_OK);
    Can_MainFunction_Read();
}
static void flush(void) { assert(Can_HostFlush() == ECU_OK); }
static void tick(void) {
    CanTp_MainFunction();
    Dcm_MainFunction();
}
static void request_read(void) {
    const uint8 bytes[8] = {3u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    receive(bytes);
}
static void start(void) {
    Det_Init(&det);
    PduR_Init(&pdur);
    LSduR_Init(&ls);
    CanIf_Init(&canif);
    Can_Init(&can);
    Ecu_HostBusSM_Init(&cdd);
    BswM_Init(&bswm);
    Dcm_Init(&dcm);
    assert(authentication_mode == 0u);
    assert(SCHM_E_OK == 0u && SCHM_E_LIMIT == 130u);
    CanTp_Init(&tp);
    ComM_Init(&comm);
    ComM_CommunicationAllowed(7u, TRUE);
    assert(ComM_RequestComMode(43u, COMM_FULL_COMMUNICATION) == E_OK);
    Can_MainFunction_Wakeup();
}
int main(void) {
    BufReq_ReturnType (*start_rx)(PduIdType, const PduInfoType *, PduLengthType, PduLengthType *) =
        Dcm_StartOfReception;
    BufReq_ReturnType (*copy_rx)(PduIdType, const PduInfoType *, PduLengthType *) = Dcm_CopyRxData;
    BufReq_ReturnType (*copy_tx)(PduIdType, const PduInfoType *, const RetryInfoType *,
                                 PduLengthType *) = Dcm_CopyTxData;
    void (*rx_done)(PduIdType, Std_ReturnType) = Dcm_TpRxIndication;
    void (*tx_done)(PduIdType, Std_ReturnType) = Dcm_TpTxConfirmation;
    Std_ReturnType (*transmit)(PduIdType, const PduInfoType *) = CanTp_Transmit;
    Std_ReturnType (*cancel_rx)(PduIdType) = CanTp_CancelReceive;
    uint8 buffer[256];
    PduLengthType available = 99u;
    PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
    PduInfoType info = {buffer, NULL_PTR, 3u};
    RetryInfoType retry = {TP_CONFPENDING, 0u};
    Std_ReturnType (*session_get)(Dcm_SesCtrlType *) = Dcm_GetSesCtrlType;
    Std_ReturnType (*security_get)(Dcm_SecLevelType *) = Dcm_GetSecurityLevel;
    Std_ReturnType (*session_reset)(void) = Dcm_ResetToDefaultSession;
    Rte_ModeType_DcmDiagnosticSessionControl (*published_session)(void) =
        SchM_Mode_Dcm_DcmDiagnosticSessionControl;
    Dcm_SesCtrlType session_value = 99u;
    Dcm_SecLevelType security_value = 99u;
    unsigned saved;
    unsigned i;
#if defined(CAN_DIAGNOSTIC_QUEUED)
    start();
    request_read();
    tick();
    assert(count == 0u);
    flush();
    assert(count == 1u && reads == 1u && output[0][0] == 7u && output[0][1] == 0x62u);
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    Can_MainFunction_Wakeup();
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    request_read();
    tick();
    flush();
    assert(count == 2u && reads == 2u && output_tokens[1] != output_tokens[0]);
    assert(memcmp(output[0], output[1], 8u) == 0);
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[0]) == E_NOT_OK);
    Can_MainFunction_Write();
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    assert(count == 2u && reads == 2u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[1]) == E_OK);
    Can_MainFunction_Write();
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[1]) == E_NOT_OK);
    Can_MainFunction_Write();
    request_read();
    tick();
    flush();
    assert(count == 3u && reads == 3u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[2]) == E_OK);
    Can_MainFunction_Write();
    /* A queued physical frame survives a TP-only restart, but cannot finish its successor. */
    request_read();
    tick();
    flush();
    assert(count == 4u && reads == 4u);
    CanTp_Shutdown();
    Dcm_Init(&dcm);
    CanTp_Init(&tp);
    Dcm_ComM_FullComModeEntered(7u);
    request_read();
    tick();
    flush();
    assert(count == 4u && reads == 5u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[3]) == E_OK);
    Can_MainFunction_Write();
    tick();
    flush();
    assert(count == 5u && reads == 5u);
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    assert(Can_HostCompleteTransmit(51u, output_tokens[4]) == E_OK);
    Can_MainFunction_Write();
    available = 99u;
    assert(copy_tx(18u, &query, NULL_PTR, &available) == BUFREQ_E_NOT_OK && available == 99u);
    return 0;
#endif
    assert(cancel_rx(11u) == E_NOT_OK);
    assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(Dcm_SetActiveDiagnostic(FALSE) == E_OK);
    assert(sizeof(Dcm_SesCtrlType) == 1u && sizeof(Dcm_SecLevelType) == 1u);
    assert(DCM_DEFAULT_SESSION == 1u && DCM_EXTENDED_DIAGNOSTIC_SESSION == 3u &&
           DCM_SEC_LEV_LOCKED == 0u);
    assert(session_get(&session_value) == E_OK && session_value == 99u);
    assert(security_get(&security_value) == E_OK && security_value == 99u);
    assert(session_get(NULL_PTR) == E_OK && security_get(NULL_PTR) == E_OK);
    assert(session_reset() == E_OK);
    {
        Dcm_ConfigType collision = dcm;
        collision.did = 0xf186u;
        Dcm_Init(&collision);
        assert(session_get(&session_value) == E_OK && session_value == 99u);
    }
    start();
    assert(session_get(&session_value) == E_OK && session_value == 1u);
    assert(transmit(72u, &info) == E_NOT_OK); /* Router ID is not a CanTp NSdu ID. */
    assert(cancel_rx(71u) == E_NOT_OK);       /* Router callback ID is not an own Rx NSdu ID. */

    assert(security_get(&security_value) == E_OK && security_value == 0u);
    assert(published_session() == 0u);
    assert(start_rx(99u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(start_rx(17u, NULL_PTR, 0u, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(start_rx(17u, NULL_PTR, 257u, &available) == BUFREQ_E_OVFL && available == 99u);
    assert(PduR_CanTpStartOfReception(99u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK &&
           available == 99u);
    Dcm_ComM_NoComModeEntered(99u);
    assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_OK && available == 256u);
    assert(copy_rx(17u, &query, &available) == BUFREQ_OK && available == 256u);
    buffer[0] = 0x22u;
    buffer[1] = 0x12u;
    buffer[2] = 0x34u;
    assert(copy_rx(17u, &info, &available) == BUFREQ_OK && available == 253u);
    available = 99u;
    assert(copy_rx(17u, &info, &available) == BUFREQ_E_NOT_OK && available == 99u);
    rx_done(17u, E_OK);
    assert(ComM_RequestComMode(43u, COMM_NO_COMMUNICATION) == E_OK);
    for (i = 0u; i < 4u; ++i) {
        assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
    }
    Dcm_ComM_SilentComModeEntered(7u);
    Dcm_MainFunction();
    flush();
    assert(count == 0u && reads == 1u);
    assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
    Dcm_ComM_FullComModeEntered(99u);
    Dcm_MainFunction();
    flush();
    assert(count == 0u);
    Dcm_ComM_FullComModeEntered(7u);
    Dcm_MainFunction();
    assert(copy_tx(18u, &query, &retry, &available) == BUFREQ_OK && available == 0u);
    assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK);
    flush();
    assert(count == 1u);
    assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
    assert(ComM_RequestComMode(43u, COMM_FULL_COMMUNICATION) == E_OK);
    {
        const uint8 expected[8] = {7u, 0x62u, 0x12u, 0x34u, 0x12u, 0x34u, 0x56u, 0x78u};
        assert(memcmp(output[0], expected, 8u) == 0);
    }
    request_read();
    Dcm_MainFunction();
    flush();
    assert(count == 2u);
    /* Failed TP releases buffer exactly once; late confirmations have no effect. */
    request_read();
    Dcm_MainFunction();
    for (i = 0u; i < 5u; ++i) {
        CanTp_MainFunction();
    }
    assert(transmit(12u, &info) == E_NOT_OK); /* N-PDU quarantined until real late confirmation. */
    tx_done(18u, E_OK);
    flush();
    assert(count == 3u);
    request_read();
    Dcm_MainFunction();
    flush();
    assert(count == 4u);
    /* Segmented request, complete reservation, FC then a CF. */
    {
        const uint8 ff[8] = {0x10u, 9u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u};
        const uint8 cf[8] = {0x21u, 0x34u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
        const uint8 fc[8] = {0x30u, 1u, 0u, 0u, 0u, 0u, 0u, 0u};
        receive(ff);
        flush();
        assert(output[count - 1u][0] == 0x30u);
        receive(cf);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 0x10u && output[count - 1u][1] == 25u);
        receive(fc);
        tick();
        flush();
        assert(output[count - 1u][0] == 0x21u);
        receive(fc);
        tick();
        flush();
        assert(output[count - 1u][0] == 0x22u);
        receive(fc);
        tick();
        flush();
        assert(output[count - 1u][0] == 0x23u);
    }
    /* Independent CopyTx retry states act on Dcm-owned bytes, not a TP message copy. */
    {
        const uint8 bytes[8] = {7u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
        uint8 first[2];
        PduInfoType two = {buffer, NULL_PTR, 2u};
        receive(bytes);
        Dcm_MainFunction();
        flush();
        retry.TpDataState = TP_CONFPENDING;
        assert(copy_tx(18u, &two, &retry, &available) == BUFREQ_OK && available == 11u);
        first[0] = buffer[0];
        first[1] = buffer[1];
        retry.TpDataState = TP_DATARETRY;
        retry.TxTpDataCnt = 2u;
        assert(copy_tx(18u, &two, &retry, &available) == BUFREQ_OK && available == 11u);
        assert(buffer[0] == first[0] && buffer[1] == first[1]);
        retry.TpDataState = TP_DATACONF;
        assert(copy_tx(18u, &query, &retry, &available) == BUFREQ_OK && available == 11u);
        retry.TpDataState = TP_DATARETRY;
        retry.TxTpDataCnt = 1u;
        available = 99u;
        buffer[0] = 0xa5u;
        assert(copy_tx(18u, &two, &retry, &available) == BUFREQ_E_NOT_OK);
        assert(available == 99u && buffer[0] == 0xa5u);
        retry.TpDataState = TP_CONFPENDING;
        info.SduLength = 256u;
        assert(copy_tx(18u, &info, &retry, &available) == BUFREQ_E_BUSY);
        assert(available == 99u && buffer[0] == 0xa5u);
        info.SduLength = 3u;
        CanTp_Shutdown();
        Dcm_Init(&dcm);
        assert(authentication_mode == 0u);
        assert(SCHM_E_OK == 0u && SCHM_E_LIMIT == 130u);
        CanTp_Init(&tp);
        Dcm_ComM_FullComModeEntered(7u);
    }
    /* Wrong SN and N_Cr abort accepted receives, making the Dcm buffer reusable. */
    {
        const uint8 ff[8] = {0x10u, 9u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u};
        const uint8 wrong[8] = {0x22u, 0x34u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
        receive(ff);
        flush();
        receive(wrong);
        request_read();
        Dcm_MainFunction();
        flush();
        receive(ff);
        flush();
        for (i = 0u; i < 5u; ++i) {
            CanTp_MainFunction();
        }
        request_read();
        Dcm_MainFunction();
        flush();
    }
    /* FF overflow sends only FC overflow; SF overflow never begins a connection. */
    {
        const uint8 ff[8] = {0x11u, 1u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u};
        saved = reads;
        receive(ff);
        flush();
        assert(output[count - 1u][0] == 0x32u && reads == saved);
    }
    /* Shutdown discards without synthesizing final upper callbacks. */
    request_read();
    Dcm_MainFunction();
    CanTp_Shutdown();
    Dcm_Init(&dcm);
    assert(authentication_mode == 0u);
    assert(SCHM_E_OK == 0u && SCHM_E_LIMIT == 130u);
    flush();
    assert(transmit(12u, &info) == E_NOT_OK);
    CanTp_Init(&tp);
    Dcm_ComM_FullComModeEntered(7u);
    request_read();
    Dcm_MainFunction();
    flush();
    /* NoCom rejects receive; ActiveDiagnostic false does not bypass Full gating. */
    Dcm_ComM_NoComModeEntered(7u);
    available = 99u;
    assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
    assert(Dcm_SetActiveDiagnostic(FALSE) == E_OK);
    Dcm_ComM_SilentComModeEntered(7u);
    saved = count;
    request_read();
    Dcm_MainFunction();
    flush();
    assert(count == saved);
    Dcm_ComM_FullComModeEntered(7u);
    Dcm_MainFunction();
    flush();
    assert(count == saved + 1u);
    /* Non-default session keeps diagnostic demand; S3 main ticks release it. */
    assert(Dcm_SetActiveDiagnostic(TRUE) == E_OK);
    assert(ComM_RequestComMode(43u, COMM_NO_COMMUNICATION) == E_OK);
    {
        const uint8 bytes[8] = {2u, 0x10u, 3u, 0u, 0u, 0u, 0u, 0u};
        receive(bytes);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 6u && output[count - 1u][1] == 0x50u);
        assert(session_get(&session_value) == E_OK && session_value == 3u);
        assert(published_session() == 2u && security_get(&security_value) == E_OK &&
               security_value == 0u);
        for (i = 0u; i < 4u; ++i) {
            assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
        }
        for (i = 0u; i < 19u; ++i) {
            Dcm_MainFunction();
        }
        assert(ComM_RunChannel() == COMM_FULL_COM_NETWORK_REQUESTED);
        Dcm_MainFunction();
        assert(ComM_RunChannel() == COMM_FULL_COM_READY_SLEEP);
        assert(session_get(&session_value) == E_OK && session_value == 1u);
        assert(published_session() == 0u);
    }
    /* N_Bs and malformed padded CF release accepted requests without replay. */
    assert(ComM_RequestComMode(43u, COMM_FULL_COMMUNICATION) == E_OK);
    {
        const uint8 bytes[8] = {7u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
        const uint8 ff[8] = {0x10u, 9u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u};
        uint8 short_cf[4] = {0x21u, 0x34u, 0x12u, 0x34u};
        const PduInfoType short_info = {short_cf, NULL_PTR, 4u};
        receive(bytes);
        Dcm_MainFunction();
        flush();
        for (i = 0u; i < 5u; ++i) {
            CanTp_MainFunction();
        }
        request_read();
        Dcm_MainFunction();
        flush();
        receive(ff);
        flush();
        CanTp_RxIndication(31u, &short_info);
        request_read();
        Dcm_MainFunction();
        flush();
    }
    /* Peer STmin may exceed N_Cs: separation is respected before the copy timer. */
    {
        const uint8 bytes[8] = {7u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
        const uint8 fc[8] = {0x30u, 0u, 10u, 0u, 0u, 0u, 0u, 0u};
        const uint8 wt[8] = {0x31u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        receive(bytes);
        Dcm_MainFunction();
        flush();
        for (i = 0u; i < 4u; ++i) {
            unsigned j;
            for (j = 0u; j < 4u; ++j) {
                CanTp_MainFunction();
            }
            receive(wt); /* Every incoming WAIT restarts N_Bs, independent of sent WFTmax2. */
        }
        receive(fc);
        saved = count;
        for (i = 0u; i < 9u; ++i) {
            CanTp_MainFunction();
            flush();
            assert(count == saved);
        }
        CanTp_MainFunction();
        flush();
        assert(count == saved + 1u && output[count - 1u][0] == 0x21u);
        CanTp_Shutdown();
        Dcm_Init(&dcm);
        assert(authentication_mode == 0u);
        assert(SCHM_E_OK == 0u && SCHM_E_LIMIT == 130u);
    }
    /* Real driver BUSY refuses the SF immediately, releasing Dcm without a retry. */
    CanTp_Init(&tp);
    Dcm_ComM_FullComModeEntered(7u);
    {
        uint8 payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        const Can_PduType pending = {51u, 8u, 0x708u, payload};
        request_read();
        assert(Can_Write(0u, &pending) == E_OK);
        saved = count;
        Dcm_MainFunction();
        assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_OK);
        rx_done(17u, E_NOT_OK);
        flush();
        assert(count == saved + 1u && output[count - 1u][0] == 0u);
        for (i = 0u; i < 5u; ++i) {
            tick();
            flush();
        }
        assert(count == saved + 1u);
        request_read();
        Dcm_MainFunction();
        flush();
        assert(count == saved + 2u && output[count - 1u][0] == 7u);
    }
    assert(reports == 2u);
    /* Configured 64-byte admission and response bounds, independent of physical storage. */
    {
        Dcm_ConfigType bounded = dcm;
        bounded.buffer_length = 64u;
        Dcm_Init(&bounded);
        Dcm_ComM_FullComModeEntered(7u);
        available = 99u;
        assert(start_rx(17u, NULL_PTR, 65u, &available) == BUFREQ_E_OVFL && available == 99u);
        assert(start_rx(17u, NULL_PTR, 63u, &available) == BUFREQ_OK && available == 64u);
        assert(copy_rx(17u, &query, &available) == BUFREQ_OK && available == 64u);
        buffer[0] = 0x22u;
        for (i = 1u; i < 63u; i += 2u) {
            buffer[i] = 0x12u;
            buffer[i + 1u] = 0x34u;
        }
        info.SduLength = 63u;
        assert(copy_rx(17u, &info, &available) == BUFREQ_OK && available == 1u);
        rx_done(17u, E_OK);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 3u && output[count - 1u][1] == 0x7fu &&
               output[count - 1u][2] == 0x22u && output[count - 1u][3] == 0x14u);
        Dcm_Init(&dcm);
        assert(authentication_mode ==
               0u); /* Retire the caller-owned configuration before its lifetime ends. */
    }
    /* Public reset, suppressed session completion and internal F186 use actual state. */
    Dcm_ComM_FullComModeEntered(7u);
    {
        const uint8 active_session[8] = {3u, 0x22u, 0xf1u, 0x86u, 0u, 0u, 0u, 0u};
        const uint8 extended[8] = {2u, 0x10u, 3u, 0u, 0u, 0u, 0u, 0u};
        const uint8 suppressed[8] = {2u, 0x10u, 0x83u, 0u, 0u, 0u, 0u, 0u};
        const uint8 unknown[8] = {2u, 0x10u, 0x85u, 0u, 0u, 0u, 0u, 0u};
        const uint8 tester_unknown[8] = {2u, 0x3eu, 0x81u, 0u, 0u, 0u, 0u, 0u};
        const uint8 tester_suppressed[8] = {2u, 0x3eu, 0x80u, 0u, 0u, 0u, 0u, 0u};
        saved = reads;
        receive(active_session);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 4u && output[count - 1u][1] == 0x62u &&
               output[count - 1u][2] == 0xf1u && output[count - 1u][3] == 0x86u &&
               output[count - 1u][4] == 1u && reads == saved);
        receive(extended);
        Dcm_MainFunction(); /* Accepted response remains owned until actual confirmation. */
        assert(session_get(&session_value) == E_OK && session_value == 1u &&
               published_session() == 0u);
        assert(session_reset() == E_OK);
        available = 99u;
        assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
        flush(); /* Late successful confirmation must not re-enter extended. */
        assert(output[count - 1u][1] == 0x50u && output[count - 1u][2] == 3u);
        assert(session_get(&session_value) == E_OK && session_value == 1u &&
               published_session() == 0u);
        receive(extended);
        assert(session_reset() == E_OK); /* Also cancels an accepted, unprocessed 0x10. */
        Dcm_MainFunction();
        flush();
        assert(session_get(&session_value) == E_OK && session_value == 1u &&
               published_session() == 0u);
        saved = count;
        receive(suppressed);
        Dcm_MainFunction();
        flush();
        assert(count == saved && session_get(&session_value) == E_OK && session_value == 3u);
        assert(published_session() == 2u);
        receive(active_session);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][4] == 3u);
        assert(session_reset() == E_OK && published_session() == 0u);
        receive(active_session);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][4] == 1u);
        saved = count;
        receive(tester_suppressed);
        Dcm_MainFunction();
        flush();
        assert(count == saved);
        receive(unknown);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 3u && output[count - 1u][1] == 0x7fu &&
               output[count - 1u][2] == 0x10u && output[count - 1u][3] == 0x12u);
        receive(tester_unknown);
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 3u && output[count - 1u][1] == 0x7fu &&
               output[count - 1u][2] == 0x3eu && output[count - 1u][3] == 0x12u);
        request_read();
        Dcm_MainFunction();
        flush();
        assert(output[count - 1u][0] == 7u && output[count - 1u][1] == 0x62u &&
               output[count - 1u][2] == 0x12u && output[count - 1u][3] == 0x34u);
    }
    request_read(); /* SF already completed to Dcm; cancellation cannot revoke it. */
    saved = reports;
    assert(cancel_rx(11u) == E_NOT_OK && reports == saved + 1u);
    Dcm_MainFunction();
    flush();
    assert(output[count - 1u][0] == 7u && output[count - 1u][1] == 0x62u);
    /* Last-CF exclusion starts at actual FC confirmation/N_Cr, not at FF admission. */
    for (i = 0u; i < 4u; ++i) {
        uint8 first[8] = {0x10u, 13u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u};
        const uint8 last[8] = {0x21u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
        boolean confirmed = (i >= 2u);
        first[1] = (uint8)(13u + (i % 2u)); /* Seven/eight bytes remain after FF. */
        Dcm_Init(&dcm);
        assert(authentication_mode == 0u);
        assert(SCHM_E_OK == 0u && SCHM_E_LIMIT == 130u);
        Dcm_ComM_FullComModeEntered(7u);
        receive(first);
        CanTp_MainFunction(); /* Actual FC accepted by the driver. */
        if (confirmed) {
            flush();
        }
        saved = reports;
        assert(cancel_rx(99u) == E_NOT_OK && reports == saved);
        if (confirmed && first[1] == 13u) {
            assert(cancel_rx(11u) == E_NOT_OK && reports == saved);
            available = 99u;
            assert(start_rx(17u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 99u);
            receive(last); /* Preserved reservation completes normally. */
            Dcm_Init(
                &dcm); /* Retire the accepted request without creating an unrelated response. */
        } else {
            assert(cancel_rx(11u) == E_OK && reports == saved);
            assert(cancel_rx(11u) == E_NOT_OK && reports == saved + 1u);
            request_read(); /* Negative Rx indication really released the Dcm reservation. */
            saved = count;
            Dcm_MainFunction();
            if (!confirmed) {
                assert(count == saved); /* Outstanding FC owns the lower mailbox. */
                flush(); /* Old real FC confirmation releases quarantine, never new Rx state. */
                assert(count == saved + 1u && output[count - 1u][0] == 0x30u);
                Dcm_MainFunction();
            }
            flush();
            assert(output[count - 1u][0] == 7u && output[count - 1u][1] == 0x62u);
        }
    }
    return 0;
}
