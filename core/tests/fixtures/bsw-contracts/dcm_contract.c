/** Independent Dcm service, timing and public-interface vectors; only lower services are stubs.
 * Verifies: SWS_Dcm_00022, SWS_Dcm_00034, SWS_Dcm_00037, SWS_Dcm_00334
 * Verifies: SWS_Dcm_00111, SWS_Dcm_00241, SWS_Dcm_00443, SWS_Dcm_00444
 * Verifies: SWS_Dcm_00642, SWS_Dcm_00996, SWS_Dcm_00342, SWS_Dcm_00344
 * Verifies: SWS_Dcm_00346, SWS_Dcm_00350, SWS_Dcm_00352, SWS_Dcm_00353
 * Verifies: SWS_Dcm_00114, SWS_Dcm_00115, SWS_Dcm_00117, SWS_Dcm_00118
 * Verifies: SWS_Dcm_00084, SWS_Dcm_00192, SWS_Dcm_00193, SWS_Dcm_00195
 * Verifies: SWS_Dcm_00197, SWS_Dcm_00198, SWS_Dcm_00200, SWS_Dcm_00201
 * Verifies: SWS_Dcm_00204, SWS_Dcm_00273, SWS_Dcm_00696, SWS_Dcm_01411
 * Verifies: SWS_Dcm_00271
 * Verifies: SWS_Dcm_00222, SWS_Dcm_00225, SWS_Dcm_00231, SWS_Dcm_00232
 * Verifies: SWS_Dcm_00235, SWS_Dcm_00237, SWS_Dcm_00238, SWS_Dcm_00240
 * Verifies: SWS_Dcm_00307, SWS_Dcm_00311, SWS_Dcm_00085, SWS_Dcm_01335
 * Verifies: SWS_Dcm_00438, SWS_Dcm_00433, SWS_Dcm_00434, SWS_Dcm_00437
 * Verifies: SWS_Dcm_00251, SWS_Dcm_01558
 * Verifies: SWS_Dcm_01062, SWS_Dcm_01068, SWS_Dcm_01069, SWS_Dcm_01070
 * Verifies: SWS_Dcm_01071, SWS_Dcm_01142, SWS_Dcm_00148, SWS_Dcm_00151
 * Verifies: SWS_Dcm_00152, SWS_Dcm_00153, SWS_Dcm_00156, SWS_Dcm_00157
 * Verifies: SWS_Dcm_00161, SWS_Dcm_01324, SWS_Dcm_01325, SWS_Dcm_01326
 * Verifies: SWS_Dcm_01373, SWS_Dcm_01374, SWS_Dcm_01375, SWS_Dcm_01376
 * Verifies: SWS_Dcm_01377, SWS_Dcm_01378, SWS_Dcm_01670, SWS_Dcm_00793
 */
#include "Rte_Dcm_Type.h"
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "SchM_Dcm.h"
#include "ComM_Dcm.h"
#include "PduR_Dcm.h"
#include "Can_HostLock.h"
#include <assert.h>
#include <string.h>

enum { RX = 17u, TX = 19u, ROUTER = 41u, CHANNEL = 7u, APP_DID = 0x1234u };
enum { READ_OK, READ_FAIL, READ_RESTART, READ_REENTER };
static unsigned read_mode;
static unsigned reads;
static unsigned transmissions;
static unsigned active;
static unsigned inactive;
static unsigned authentication_calls;
static unsigned mode_depth;
static boolean reset_from_mode;
static boolean synchronous = TRUE;
static boolean refuse;
static uint8 published_mode;
static uint8 sent[256];
static PduLengthType sent_length;
static void receive(const uint8 *data, PduLengthType length);
static void start(void);
static Std_ReturnType read_did(uint8 *data)
{
    ++reads;
    data[0] = 0x12u;
    data[1] = 0x34u;
    data[2] = 0x56u;
    data[3] = 0x78u;
    if (read_mode == READ_RESTART) {
        const uint8 next[] = {0x22u, 0xf1u, 0x86u};
        read_mode = READ_OK;
        start();
        receive(next, sizeof(next));
        Dcm_MainFunction();
    } else if (read_mode == READ_REENTER) {
        Dcm_MainFunction();
    }
    return (read_mode == READ_FAIL) ? E_NOT_OK : E_OK;
}
static void authentication(uint8 value)
{
    assert(value == 0u);
    ++authentication_calls;
}
static const Dcm_ConfigType config = {
    RX, TX, CHANNEL, APP_DID, read_did, 4u, 50u, 5000u, 5u, 64u, ROUTER, authentication, 0u};
void ComM_DCM_ActiveDiagnostic(NetworkHandleType channel)
{
    assert(channel == CHANNEL);
    /* Reentrant getters also prove that notification is outside DCM_STATE. */
    Dcm_SesCtrlType value = 0u;
    assert(Dcm_GetSesCtrlType(&value) == E_OK && (value == 1u || value == 3u));
    ++active;
}
void ComM_DCM_InactiveDiagnostic(NetworkHandleType channel)
{
    assert(channel == CHANNEL);
    assert(Dcm_SetActiveDiagnostic(TRUE) == E_OK);
    ++inactive;
}
Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(
    Rte_ModeType_DcmDiagnosticSessionControl mode)
{
    assert(++mode_depth == 1u);
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    published_mode = mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    if (reset_from_mode && mode == 2u) {
        reset_from_mode = FALSE;
        assert(Dcm_ResetToDefaultSession() == E_OK);
    }
    --mode_depth;
    return E_OK;
}
Std_ReturnType PduR_DcmTransmit(PduIdType id, const PduInfoType *info)
{
    PduLengthType available = 999u;
    PduInfoType copy = {sent, NULL_PTR, info->SduLength};
    assert(id == ROUTER && info->MetaDataPtr == NULL_PTR && info->SduDataPtr == NULL_PTR);
    ++transmissions;
    if (refuse) {
        return E_NOT_OK;
    }
    assert(Dcm_CopyTxData(TX, &copy, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    sent_length = info->SduLength;
    if (synchronous) {
        Dcm_TpTxConfirmation(TX, E_OK);
    }
    return E_OK;
}
static void start(void)
{
    Dcm_Init(&config);
    Dcm_ComM_FullComModeEntered(CHANNEL);
}
static void receive(const uint8 *data, PduLengthType length)
{
    PduLengthType available = 999u;
    /* PduInfoType specifies a mutable pointer, but CopyRxData must only read it. */
    uint8 bytes[256];
    const PduInfoType info = {bytes, NULL_PTR, length};
    assert(length <= sizeof(bytes));
    memcpy(bytes, data, length);
    assert(Dcm_StartOfReception(RX, &info, length, &available) == BUFREQ_OK);
    assert(Dcm_CopyRxData(RX, &info, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(RX, E_OK);
}
static void exchange(const uint8 *data, PduLengthType length)
{
    receive(data, length);
    Dcm_MainFunction();
}
static void expect(const uint8 *data, PduLengthType length)
{
    assert(sent_length == length && memcmp(sent, data, length) == 0);
}
static void negative(const uint8 *data, PduLengthType length, uint8 nrc)
{
    const uint8 expected[] = {0x7fu, data[0], nrc};
    exchange(data, length);
    expect(expected, sizeof(expected));
}
static void admission_and_copy(void)
{
    uint8 bytes[64] = {0x22u, 0xf1u, 0x86u};
    PduLengthType available = 999u;
    PduInfoType info = {bytes, NULL_PTR, 3u};
    PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
    assert(Dcm_StartOfReception(RX + 1u, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK);
    assert(available == 999u);
    assert(Dcm_StartOfReception(RX, NULL_PTR, 0u, &available) == BUFREQ_E_NOT_OK);
    assert(Dcm_StartOfReception(RX, NULL_PTR, 65u, &available) == BUFREQ_E_OVFL);
    assert(available == 999u);
    assert(Dcm_StartOfReception(RX, &info, 3u, &available) == BUFREQ_OK && available == 64u);
    assert(Dcm_CopyRxData(RX, &query, &available) == BUFREQ_OK && available == 64u);
    assert(Dcm_CopyRxData(RX, &info, &available) == BUFREQ_OK && available == 61u);
    available = 999u;
    assert(Dcm_CopyRxData(RX, &info, &available) == BUFREQ_E_NOT_OK && available == 999u);
    const unsigned before = transmissions;
    Dcm_MainFunction();
    assert(transmissions == before);
    Dcm_TpRxIndication(RX, E_NOT_OK);
    Dcm_MainFunction();
    assert(transmissions == before);
    exchange(bytes, 3u);
    assert(transmissions == before + 1u);
}
static void ordered_reads_and_limits(void)
{
    const uint8 request[] = {0x22u, 0x12u, 0x34u, 0xabu, 0xcdu, 0xf1u, 0x86u, 0x12u, 0x34u};
    const uint8 response[] = {0x62u,
                              0x12u,
                              0x34u,
                              0x12u,
                              0x34u,
                              0x56u,
                              0x78u,
                              0xf1u,
                              0x86u,
                              1u,
                              0x12u,
                              0x34u,
                              0x12u,
                              0x34u,
                              0x56u,
                              0x78u};
    const uint8 only_unknown[] = {0x22u, 0xabu, 0xcdu};
    const uint8 reversed[] = {0x22u, 0xf1u, 0x86u, 0x12u, 0x34u};
    const uint8 reverse_response[] = {
        0x62u, 0xf1u, 0x86u, 1u, 0x12u, 0x34u, 0x12u, 0x34u, 0x56u, 0x78u};
    unsigned before = reads;
    exchange(request, sizeof(request));
    expect(response, sizeof(response));
    assert(reads == before + 2u);
    exchange(reversed, sizeof(reversed));
    expect(reverse_response, sizeof(reverse_response));
    negative(only_unknown, sizeof(only_unknown), 0x31u);
    Dcm_ConfigType limited = config;
    limited.max_did_to_read = 2u;
    Dcm_Init(&limited);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    before = reads;
    negative(request, sizeof(request), 0x13u);
    assert(reads == before);
    exchange(reversed, sizeof(reversed));
    expect(reverse_response, sizeof(reverse_response));
    limited.max_did_to_read = 0u;
    limited.buffer_length = 7u;
    Dcm_Init(&limited);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    const uint8 too_long[] = {0x22u, 0x12u, 0x34u, 0x12u, 0x34u};
    negative(too_long, sizeof(too_long), 0x14u);
    start(); /* Retire the caller-owned configuration before it leaves scope. */
}
static void failure_and_dispatch(void)
{
    /* Verifies: SWS_Dcm_00273, SWS_Dcm_00696, SWS_Dcm_00272
     * DSD subfunction refusal precedes the DSP exact-length check. */
    const uint8 bad_session[] = {0x10u, 0x82u, 0u};
    const uint8 bad_tester[] = {0x3eu, 0x81u, 0u};
    const uint8 long_session[] = {0x10u, 3u, 0u};
    const uint8 long_tester[] = {0x3eu, 0u, 0u};
    const uint8 short_session[] = {0x10u};
    negative(bad_session, sizeof(bad_session), 0x12u);
    negative(bad_tester, sizeof(bad_tester), 0x12u);
    negative(long_session, sizeof(long_session), 0x13u);
    negative(long_tester, sizeof(long_tester), 0x13u);
    negative(short_session, sizeof(short_session), 0x13u);

    const uint8 request[] = {0x22u, 0x12u, 0x34u};
    read_mode = READ_FAIL;
    negative(request, sizeof(request), 0x10u);
    read_mode = READ_OK;
    const uint8 expected[] = {0x62u, 0x12u, 0x34u, 0x12u, 0x34u, 0x56u, 0x78u};
    exchange(request, sizeof(request));
    expect(expected, sizeof(expected));
    refuse = TRUE;
    unsigned before = transmissions;
    exchange(request, sizeof(request));
    assert(transmissions == before + 1u);
    for (unsigned i = 0u; i < 6u; ++i) {
        Dcm_MainFunction();
    }
    assert(transmissions == before + 1u);
    refuse = FALSE;
    exchange(request, sizeof(request));
    expect(expected, sizeof(expected));
    for (unsigned i = 0u; i < 256u; ++i) {
        const uint8 unknown[] = {(uint8)i};
        if (i == 0x10u || i == 0x22u || i == 0x3eu) {
            continue;
        }
        before = transmissions;
        exchange(unknown, sizeof(unknown));
        if ((i >= 0x40u && i <= 0x7fu) || i >= 0xc0u) {
            assert(transmissions == before);
        } else {
            const uint8 unsupported[] = {0x7fu, (uint8)i, 0x11u};
            assert(transmissions == before + 1u);
            expect(unsupported, sizeof(unsupported));
        }
    }
}
static void sessions_and_keepalive(void)
{
    const uint8 extended[] = {0x10u, 3u};
    const uint8 suppressed[] = {0x10u, 0x83u};
    const uint8 tester[] = {0x3eu, 0x80u};
    const uint8 wrong[] = {0x10u, 0x85u};
    const uint8 read[] = {0x22u, 0xf1u, 0x86u};
    Dcm_SesCtrlType session = 99u;
    synchronous = FALSE;
    exchange(extended, sizeof(extended));
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u && published_mode == 0u);
    Dcm_TpTxConfirmation(TX, E_NOT_OK);
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u);
    exchange(extended, sizeof(extended));
    Dcm_TpTxConfirmation(TX, E_OK);
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 3u && published_mode == 2u);
    exchange(read, sizeof(read));
    const unsigned before = transmissions;
    /* Verifies: SWS_Dcm_01677, SWS_Dcm_01678
     * A busy connection declines a keep-alive or any other request without a response. */
    PduLengthType declined = 999u;
    assert(Dcm_StartOfReception(RX, NULL_PTR, sizeof(tester), &declined) == BUFREQ_E_NOT_OK);
    assert(Dcm_StartOfReception(RX, NULL_PTR, sizeof(read), &declined) == BUFREQ_E_NOT_OK);
    assert(declined == 999u);
    Dcm_MainFunction();
    assert(transmissions == before);
    uint8 bytes[4] = {0u};
    PduLengthType available = 999u;
    const PduInfoType query = {bytes, NULL_PTR, 0u};
    assert(Dcm_CopyTxData(TX, &query, NULL_PTR, &available) == BUFREQ_OK && available == 0u);
    assert(sent_length == 4u && sent[0] == 0x62u && sent[3] == 3u);
    Dcm_TpTxConfirmation(TX, E_OK);
    for (unsigned i = 0u; i < 4u; ++i) {
        Dcm_MainFunction();
    }
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 3u);
    Dcm_MainFunction();
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u && published_mode == 0u);
    synchronous = TRUE;
    negative(wrong, sizeof(wrong), 0x12u);
    exchange(suppressed, sizeof(suppressed));
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 3u);
    assert(Dcm_ResetToDefaultSession() == E_OK && published_mode == 0u);
    reset_from_mode = TRUE;
    exchange(suppressed, sizeof(suppressed));
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u && published_mode == 0u);
}
static void gates_and_reentrancy(void)
{
    const uint8 request[] = {0x22u, 0x12u, 0x34u};
    PduLengthType available = 999u;
    Dcm_ComM_NoComModeEntered(CHANNEL);
    assert(Dcm_StartOfReception(RX, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK);
    Dcm_ComM_FullComModeEntered(CHANNEL + 1u);
    assert(Dcm_StartOfReception(RX, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK);
    Dcm_ComM_SilentComModeEntered(CHANNEL);
    unsigned before = transmissions;
    receive(request, sizeof(request));
    for (unsigned i = 0u; i < 4u; ++i) {
        Dcm_MainFunction();
    }
    assert(transmissions == before);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    Dcm_MainFunction();
    assert(transmissions == before);
    assert(Dcm_SetActiveDiagnostic(FALSE) == E_OK);
    const unsigned demand = active;
    exchange(request, sizeof(request));
    assert(active == demand && transmissions == before + 1u);
    assert(Dcm_SetActiveDiagnostic(TRUE) == E_OK);
    read_mode = READ_REENTER;
    before = reads;
    exchange(request, sizeof(request));
    assert(reads == before + 1u);
    read_mode = READ_RESTART;
    before = transmissions;
    exchange(request, sizeof(request));
    assert(transmissions == before);
    Dcm_MainFunction();
    const uint8 expected[] = {0x62u, 0xf1u, 0x86u, 1u};
    expect(expected, sizeof(expected));
    assert(transmissions == before + 1u && inactive > 0u);
}
/* One TP lifetime per admitted response: a confirmation for another ID leaves it
 * reserved, and Init retires an accepted lifetime whose old TP is quiesced. */
static void lower_lifetime(void)
{
    const uint8 request[] = {0x3eu, 0x00u};
    const uint8 positive[] = {0x7eu, 0x00u};
    PduLengthType available = 999u;
    start();
    synchronous = FALSE;
    const unsigned before = transmissions;
    exchange(request, sizeof(request));
    Dcm_TpTxConfirmation(TX + 1u, E_OK);
    assert(Dcm_StartOfReception(RX, NULL_PTR, sizeof(request), &available) == BUFREQ_E_NOT_OK);
    Dcm_TpTxConfirmation(TX, E_OK);
    exchange(request, sizeof(request));
    assert(transmissions == before + 2u);
    start();
    exchange(request, sizeof(request));
    assert(transmissions == before + 3u);
    expect(positive, sizeof(positive));
    Dcm_TpTxConfirmation(TX, E_OK);
    synchronous = TRUE;
}
/* Verifies: SWS_Dcm_01197, SWS_Dcm_01379, SWS_Dcm_01380
 * Verifies: SWS_Dcm_01387, SWS_Dcm_01389, SWS_Dcm_01385
 */
static void obd_data_and_availability(void)
{
    Dcm_ConfigType selected = config;
    for (uint16 family = 0xf400u; family <= 0xf800u; family += 0x400u) {
        selected.did = family + 0x21u;
        Dcm_Init(&selected);
        Dcm_ComM_FullComModeEntered(CHANNEL);
        unsigned before = reads;
        const uint8 request[] = {0x22u, (uint8)(family / 256u), 0u, (uint8)(family / 256u), 0x20u};
        const uint8 expected[] = {0x62u,
                                  (uint8)(family / 256u),
                                  0u,
                                  0u,
                                  0u,
                                  0u,
                                  1u,
                                  (uint8)(family / 256u),
                                  0x20u,
                                  0x80u,
                                  0u,
                                  0u,
                                  0u};
        exchange(request, sizeof(request));
        expect(expected, sizeof(expected));
        assert(reads == before);
        const uint8 data[] = {0x22u, (uint8)(family / 256u), 0x21u};
        const uint8 value[] = {0x62u, (uint8)(family / 256u), 0x21u, 0x12u, 0x34u, 0x56u, 0x78u};
        exchange(data, sizeof(data));
        expect(value, sizeof(value));
        assert(reads == before + 1u);
        const uint8 absent[] = {0x22u, (uint8)(family / 256u), 0x40u};
        negative(absent, sizeof(absent), 0x31u);
        selected.buffer_length = 7u;
        Dcm_Init(&selected);
        Dcm_ComM_FullComModeEntered(CHANNEL);
        negative(request, sizeof(request), 0x14u);
        selected.buffer_length = config.buffer_length;
        selected.did = family + 0xffu;
        Dcm_Init(&selected);
        Dcm_ComM_FullComModeEntered(CHANNEL);
        const uint8 last[] = {0x22u, (uint8)(family / 256u), 0xe0u};
        const uint8 bit[] = {0x62u, (uint8)(family / 256u), 0xe0u, 0u, 0u, 0u, 2u};
        exchange(last, sizeof(last));
        expect(bit, sizeof(bit));
    }
    /* Configuration is retained; restore it before the local copy goes out of scope. */
    start();
}
int main(void)
{
    /* Verifies: SWS_Dcm_00979, SWS_Dcm_00980, SWS_Dcm_00983, SWS_Dcm_00984 */
    assert(sizeof(Dcm_ProtocolType) == 1u && DCM_UDS_ON_CAN == 3u);
    assert(DCM_NO_ACTIVE_PROTOCOL == 12u && DCM_SUPPLIER_15 == 254u);
    assert(sizeof(Dcm_ConfirmationStatusType) == 1u && DCM_RES_NEG_NOT_OK == 3u);
    assert(sizeof(Dcm_OpStatusType) == 1u && DCM_FORCE_RCRRP_OK == 3u);
    assert(sizeof(Dcm_NegativeResponseCodeType) == 1u && DCM_E_GENERALREJECT == 0x10u);
    assert(DCM_E_REQUESTOUTOFRANGE == 0x31u && DCM_E_VMSCNC_E == 0xfeu);
    /* Verifies: SWS_Dcm_00989, SWS_Dcm_00990, SWS_Dcm_00991, SWS_Dcm_00992
     * Verifies: SWS_Dcm_00993, SWS_Dcm_00994, SWS_Dcm_00202, SWS_Dcm_00223
     * Verifies: SWS_Dcm_00224, SWS_Dcm_00231
     */
    /* Verifies: SWS_Dcm_91051, SWS_Dcm_91063, SWS_Dcm_01121 */
    assert(sizeof(Dcm_DataElement_ApplicationValueType) == 4u);
    assert(sizeof(Dcm_DataElement_ApplicationValue_ArrayType) == 4u);
    assert(sizeof(Dcm_DataArrayTypeUint8_ApplicationValueType) == 4u);
    Dcm_MsgAddInfoType attributes = {0u, 1u};
    uint8 representation;
    memcpy(&representation, &attributes, sizeof(representation));
    assert((representation & 3u) == 2u && sizeof(Dcm_MsgLenType) == 4u);
    attributes.reqType = 1u;
    attributes.suppressPosResponse = 0u;
    memcpy(&representation, &attributes, sizeof(representation));
    assert((representation & 3u) == 1u && sizeof(Dcm_IdContextType) == 1u);
    Std_ReturnType (*session_get)(Dcm_SesCtrlType *) = Dcm_GetSesCtrlType;
    Std_ReturnType (*security_get)(Dcm_SecLevelType *) = Dcm_GetSecurityLevel;
    BufReq_ReturnType (*start_rx)(PduIdType, const PduInfoType *, PduLengthType, PduLengthType *) =
        Dcm_StartOfReception;
    Dcm_SesCtrlType session = 99u;
    Dcm_SecLevelType security = 99u;
    PduLengthType available = 999u;
    assert(session_get(&session) == E_OK && session == 99u);
    assert(security_get(&security) == E_OK && security == 99u);
    assert(start_rx(RX, NULL_PTR, 3u, &available) == BUFREQ_E_NOT_OK && available == 999u);
    assert(Dcm_ResetToDefaultSession() == E_OK);
    start();
    assert(authentication_calls == 1u && published_mode == 0u);
    assert(session_get(&session) == E_OK && session == 1u);
    assert(security_get(&security) == E_OK && security == 0u);
    admission_and_copy();
    ordered_reads_and_limits();
    failure_and_dispatch();
    sessions_and_keepalive();
    gates_and_reentrancy();
    obd_data_and_availability();
    lower_lifetime();
    return 0;
}
