/** libFuzzer driver of the delivered Dcm through its public interface (R24-11).
 * The input is a configuration prefix and a sequence of operations a transport layer,
 * ComM and the application can perform: receptions in any split, indications, main
 * functions, response copies with every retry state, confirmations, communication modes,
 * session reset and the next answers of PduR and the DID reader. The stubs check what
 * PduR and ComM may rely on and the responses of the selected service table (0x10, 0x22,
 * 0x3E); afterwards the S3Server returns Dcm to the default session, inactive, and a new
 * TesterPresent is answered. A failed check prints its line and aborts.
 */
#include "Rte_Dcm_Type.h"
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "SchM_Dcm.h"
#include "ComM_Dcm.h"
#include "PduR_Dcm.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) ((condition) ? (void)0 : DcmFuzz_Fail(__LINE__, #condition))

enum { RX = 17u, TX = 19u, ROUTER = 41u, CHANNEL = 7u, SETTLE = 64u };
enum { MODE_NO, MODE_SILENT, MODE_FULL };

static Std_ReturnType read_did(uint8 *data);
static void authentication(uint8 state);
/* Restored before each input, so that an input runs alone as it does when replayed. */
static const Dcm_ConfigType defaults = {
    RX, TX, CHANNEL, 0x1234u, read_did, 4u, 50u, 5000u, 5u, 64u, ROUTER, authentication, 0u};
static Dcm_ConfigType config;
static const uint8 *input;
static size_t input_size;
static size_t input_at;
/* What PduR and the application answer next. */
static Std_ReturnType transmit_result;
static boolean copy_inside;
static boolean confirm_inside;
static Std_ReturnType confirm_inside_result;
static Std_ReturnType read_result;
/* Communication mode last entered on the configured channel. */
static uint8 mode;
/* The reception Dcm accepted, until its indication. */
static boolean rx_open;
static PduLengthType rx_length;
static PduLengthType rx_copied;
static uint8 rx_bytes[256];
/* The last complete request, its processing and its one response. */
static boolean request_current;
static boolean request_waiting;
static boolean request_answered;
/* Indicated in full communication, which has not been left since, and the main functions
 * run since the indication. */
static boolean request_full;
static unsigned request_periods;
static uint8 request[256];
static PduLengthType request_length;
/* The response transmission from PduR_DcmTransmit until its confirmation. */
static boolean tx_pending;
static PduLengthType tx_length;
static PduLengthType tx_position;
static PduLengthType tx_confirmed;
static uint8 response[256];
static boolean known[256];
static boolean checked;
static boolean comm_active;
static Rte_ModeType_DcmDiagnosticSessionControl session_mode;

static void DcmFuzz_Fail(int line, const char *condition)
{
    (void)fprintf(stderr, "dcm_request.c:%d: check failed: %s\n", line, condition);
    abort();
}
static boolean take(uint8 *value)
{
    boolean available = FALSE;
    if (input_at < input_size) {
        *value = input[input_at];
        ++input_at;
        available = TRUE;
    }
    return available;
}
static boolean no_response(uint8 sid)
{
    return ((sid >= 0x40u) && (sid <= 0x7fu)) || (sid >= 0xc0u);
}
static boolean known_service(uint8 sid)
{
    return (sid == 0x10u) || (sid == 0x22u) || (sid == 0x3eu);
}

static Std_ReturnType read_did(uint8 *data)
{
    data[0] = 0x12u;
    data[1] = 0x34u;
    data[2] = 0x56u;
    data[3] = 0x78u;
    return read_result;
}
static void authentication(uint8 state) { CHECK(state == 0u); }
/* Diagnostic state notifications name the connection's channel (SWS_Dcm_01376, 01377). */
void ComM_DCM_ActiveDiagnostic(NetworkHandleType channel)
{
    CHECK(channel == CHANNEL);
    comm_active = TRUE;
}
void ComM_DCM_InactiveDiagnostic(NetworkHandleType channel)
{
    CHECK(channel == CHANNEL);
    comm_active = FALSE;
}
Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(
    Rte_ModeType_DcmDiagnosticSessionControl next)
{
    CHECK(next == RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION ||
          next == RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION);
    session_mode = next;
    return E_OK;
}

/* A complete response answers the request per ISO 14229-1 (SWS_Dcm_01622): `7F SID NRC` or
 * SID + 0x40. A service outside the table is refused with NRC 0x11 (SWS_Dcm_00197); a set
 * suppressPosRspMsgIndicationBit allows only negative responses (SWS_Dcm_00200). */
static void check_response(void)
{
    const uint8 sid = request[0];
    const boolean suppressed = ((sid == 0x10u) || (sid == 0x3eu)) && (request_length >= 2u) &&
                               ((request[1] & 0x80u) != 0u);
    if (response[0] == 0x7fu) {
        CHECK(tx_length == 3u && response[1] == sid && response[2] != 0u);
        CHECK(known_service(sid) || response[2] == 0x11u);
    } else {
        CHECK(known_service(sid) && !suppressed && response[0] == (uint8)(sid + 0x40u));
    }
}
/* Copies through Dcm_CopyTxData with the retry semantics of SWS_COMTYPE_00027: DATACONF
 * confirms the bytes copied so far, CONFPENDING keeps them, DATARETRY copies again from
 * TxTpDataCnt bytes back, which needs unconfirmed bytes; no retry information confirms. Data
 * never exceeds the response, available bytes never exceed what is left (SWS_Dcm_00350),
 * and a retried byte equals its first copy. */
static BufReq_ReturnType copy(PduIdType id, PduLengthType length, const RetryInfoType *retry)
{
    uint8 *data = (length > 0u) ? malloc(length) : NULL_PTR;
    const PduInfoType info = {data, NULL_PTR, length};
    PduLengthType available = 0xffffu;
    PduLengthType start = tx_position;
    boolean valid = TRUE;
    BufReq_ReturnType result;
    CHECK(length == 0u || data != NULL_PTR);
    if ((retry != NULL_PTR) && (retry->TpDataState == TP_DATARETRY)) {
        valid = (retry->TxTpDataCnt <= (PduLengthType)(tx_position - tx_confirmed));
        start = valid ? (PduLengthType)(tx_position - retry->TxTpDataCnt) : start;
    } else if ((retry != NULL_PTR) && (retry->TpDataState != TP_DATACONF) &&
               (retry->TpDataState != TP_CONFPENDING)) {
        valid = FALSE;
    } else {
        /* Copy at the current position. */
    }
    result = Dcm_CopyTxData(id, &info, retry, &available);
    if (result == BUFREQ_OK) {
        PduLengthType i;
        CHECK(id == TX && tx_pending && valid && length <= tx_length - start);
        for (i = 0u; i < length; ++i) {
            CHECK(!known[start + i] || response[start + i] == data[i]);
            response[start + i] = data[i];
            known[start + i] = TRUE;
        }
        if (retry == NULL_PTR) {
            tx_confirmed = (PduLengthType)(start + length);
        } else if (retry->TpDataState == TP_DATACONF) {
            tx_confirmed = tx_position;
        } else {
            /* Pending or retried bytes stay unconfirmed. */
        }
        tx_position = (PduLengthType)(start + length);
        CHECK(available <= (PduLengthType)(tx_length - tx_position));
        for (i = 0u; (i < tx_length) && known[i]; ++i) {
        }
        if ((i == tx_length) && !checked) {
            checked = TRUE;
            check_response();
        }
    }
    free(data);
    return result;
}
/* From the request's acceptance to the confirmation of its response the connection is
 * blocked, and one request has at most one response (SWS_Dcm_00241, 01678, 00118). */
static void confirm(PduIdType id, Std_ReturnType result)
{
    const boolean ends = (id == TX) && tx_pending;
    Dcm_TpTxConfirmation(id, result);
    if (ends) {
        tx_pending = FALSE;
        request_current = FALSE;
    }
}
/* The response of the current request, within the configured buffer (SWS_Dcm_00115), never
 * for the SIDs excluded by DcmRespondAllRequest = FALSE (SWS_Dcm_00084). */
Std_ReturnType PduR_DcmTransmit(PduIdType id, const PduInfoType *info)
{
    CHECK(id == ROUTER && info != NULL_PTR && info->MetaDataPtr == NULL_PTR);
    CHECK(info->SduLength > 0u && info->SduLength <= config.buffer_length);
    CHECK(!tx_pending && request_current && !request_answered && !no_response(request[0]));
    request_answered = TRUE;
    tx_pending = TRUE;
    tx_length = info->SduLength;
    tx_position = 0u;
    tx_confirmed = 0u;
    checked = FALSE;
    (void)memset(known, 0, sizeof(known));
    if (transmit_result == E_OK) {
        if (copy_inside) {
            CHECK(copy(TX, tx_length, NULL_PTR) == BUFREQ_OK);
        }
        if (confirm_inside) {
            confirm(TX, confirm_inside_result);
        }
    } else {
        /* A refused transmission ends the request without a repetition. */
        tx_pending = FALSE;
        request_current = FALSE;
    }
    return transmit_result;
}

static void start(PduIdType id, PduLengthType length, const PduInfoType *info)
{
    PduLengthType size = 0xffffu;
    const BufReq_ReturnType result = Dcm_StartOfReception(id, info, length, &size);
    if (result == BUFREQ_OK) {
        /* A blocked connection declines; a request larger than the buffer overflows
         * (SWS_Dcm_00241, 01678, 00444). */
        CHECK(id == RX && !request_waiting && !tx_pending);
        CHECK(length > 0u && length <= config.buffer_length && size >= length);
        rx_open = TRUE;
        rx_length = length;
        rx_copied = 0u;
        request_current = FALSE;
        request_full = FALSE;
    }
}
/* Accepted data fits the announced request and the free space is reported (SWS_Dcm_00443). */
static void copy_rx(PduIdType id, const uint8 *bytes, PduLengthType length)
{
    uint8 *data = (length > 0u) ? malloc(length) : NULL_PTR;
    const PduInfoType info = {data, NULL_PTR, length};
    PduLengthType size = 0xffffu;
    BufReq_ReturnType result;
    CHECK(length == 0u || data != NULL_PTR);
    if (length > 0u) {
        memcpy(data, bytes, length);
    }
    result = Dcm_CopyRxData(id, &info, &size);
    if (result == BUFREQ_OK) {
        CHECK(id == RX && rx_open && length <= rx_length - rx_copied);
        memcpy(&rx_bytes[rx_copied], bytes, length);
        rx_copied = (PduLengthType)(rx_copied + length);
        CHECK(size == config.buffer_length - rx_copied);
    }
    free(data);
}
/* Only a complete reception indicated with E_OK is a request (SWS_Dcm_00111, 00344). Without
 * communication its processing is not required, so it is not expected to block. */
static void indicate(PduIdType id, Std_ReturnType result)
{
    if ((id == RX) && rx_open) {
        rx_open = FALSE;
        if ((result == E_OK) && (rx_copied == rx_length)) {
            request_current = TRUE;
            request_waiting = (mode != MODE_NO);
            request_full = (mode == MODE_FULL);
            request_periods = 0u;
            request_answered = FALSE;
            request_length = rx_length;
            memcpy(request, rx_bytes, rx_length);
        }
    }
    Dcm_TpRxIndication(id, result);
}
/* A service outside the table is answered with NRC 0x11 (SWS_Dcm_00197); in full
 * communication something is sent by P2ServerMax, one period of slack for the indication
 * (SWS_Dcm_00024). */
static void main_function(void)
{
    Dcm_MainFunction();
    request_waiting = FALSE;
    if (request_full && !known_service(request[0]) && !no_response(request[0])) {
        ++request_periods;
        CHECK(request_answered || (request_periods <= config.p2_ticks));
    }
}
static void enter(uint8 next, NetworkHandleType channel)
{
    if (channel == CHANNEL) {
        mode = next;
        request_full = request_full && (next == MODE_FULL);
    }
    if (next == MODE_NO) {
        Dcm_ComM_NoComModeEntered(channel);
    } else if (next == MODE_SILENT) {
        Dcm_ComM_SilentComModeEntered(channel);
    } else {
        Dcm_ComM_FullComModeEntered(channel);
    }
}
static void restart(void)
{
    Dcm_Init(&config);
    /* Init ends every request and retires an awaited response; communication restarts with
     * no mode. */
    mode = MODE_NO;
    rx_open = FALSE;
    request_current = FALSE;
    request_waiting = FALSE;
    request_full = FALSE;
    tx_pending = FALSE;
}
static void receive(const uint8 *bytes, PduLengthType length)
{
    const PduInfoType info = {NULL_PTR, NULL_PTR, 0u};
    start(RX, length, &info);
    copy_rx(RX, bytes, length);
    indicate(RX, E_OK);
}
static void behave(uint8 value)
{
    transmit_result = ((value & 1u) != 0u) ? E_NOT_OK : E_OK;
    copy_inside = ((value & 2u) != 0u) ? TRUE : FALSE;
    confirm_inside = ((value & 4u) != 0u) ? TRUE : FALSE;
    confirm_inside_result = ((value & 8u) != 0u) ? E_NOT_OK : E_OK;
    read_result = ((value & 16u) != 0u) ? E_NOT_OK : E_OK;
}
static boolean take_bytes(uint8 *bytes, PduLengthType length)
{
    PduLengthType i;
    boolean complete = TRUE;
    for (i = 0u; (i < length) && complete; ++i) {
        complete = take(&bytes[i]);
    }
    return complete;
}
static void configure(void)
{
    static const uint16 dids[4] = {0x1234u, 0xf40du, 0xf81fu, 0xf4ffu};
    uint8 value = 0u;
    if (take(&value)) {
        config.buffer_length = (uint16)(8u + (value % 249u));
    }
    if (take(&value)) {
        config.p2_ticks = (uint16)(1u + (value & 7u));
        config.s3_ticks = 1u + ((uint32)value >> 3u);
    }
    if (take(&value)) {
        config.did = dids[value & 3u];
        config.read = ((value & 4u) != 0u) ? NULL_PTR : read_did;
        config.max_did_to_read = (uint16)(((uint32)value >> 3u) & 3u);
    }
}
static void operate(void)
{
    uint8 op = 0u;
    uint8 value = 0u;
    uint8 bytes[256];
    while (take(&op) && take(&value)) {
        switch (op % 12u) {
            case 0u: {
                uint8 low = 0u;
                PduInfoType info = {bytes, NULL_PTR, 0u};
                PduLengthType length;
                if (!take(&low)) {
                    return;
                }
                length = (PduLengthType)((((uint32)value << 8u) | low) % 300u);
                info.SduLength = (PduLengthType)(((uint32)low & 7u) <= length ? (low & 7u) : 0u);
                if (!take_bytes(bytes, info.SduLength)) {
                    return;
                }
                start(((value & 0x80u) != 0u) ? (PduIdType)(RX + 1u) : RX,
                      length,
                      ((low & 8u) != 0u) ? NULL_PTR : &info);
                break;
            }
            case 1u:
                if (!take_bytes(bytes, value)) {
                    return;
                }
                copy_rx(RX, bytes, value);
                break;
            case 2u:
                indicate(((value & 2u) != 0u) ? (PduIdType)(RX + 1u) : RX,
                         ((value & 1u) != 0u) ? E_NOT_OK : E_OK);
                break;
            case 3u: {
                uint8 i;
                for (i = 0u; i <= (value & 3u); ++i) {
                    main_function();
                }
                break;
            }
            case 4u: {
                uint8 kind = 0u;
                uint8 count = 0u;
                RetryInfoType retry = {TP_DATACONF, 0u};
                if (!take(&kind) || !take(&count)) {
                    return;
                }
                retry.TpDataState = (TpDataStateType)((kind % 5u) - 1u);
                retry.TxTpDataCnt = count;
                (void)copy(((kind & 0x80u) != 0u) ? (PduIdType)(TX + 1u) : TX,
                           value,
                           ((kind % 5u) == 0u) ? NULL_PTR : &retry);
                break;
            }
            case 5u:
                confirm(((value & 2u) != 0u) ? (PduIdType)(TX + 1u) : TX,
                        ((value & 1u) != 0u) ? E_NOT_OK : E_OK);
                break;
            case 6u:
                enter((uint8)(value % 3u),
                      ((value & 0x80u) != 0u) ? (NetworkHandleType)(CHANNEL + 1u) : CHANNEL);
                break;
            case 7u:
                if ((value & 1u) != 0u) {
                    CHECK(Dcm_ResetToDefaultSession() == E_OK);
                } else {
                    CHECK(Dcm_SetActiveDiagnostic(((value & 2u) != 0u) ? TRUE : FALSE) == E_OK);
                }
                break;
            case 8u:
                behave(value);
                break;
            case 9u:
                restart();
                break;
            default: {
                const PduLengthType length = (PduLengthType)(1u + (value % 64u));
                if (!take_bytes(bytes, length)) {
                    return;
                }
                receive(bytes, length);
                break;
            }
        }
    }
}
/* With the transport finishing its sessions, the request in progress ends, the S3Server
 * returns to the default session and Dcm becomes inactive (SWS_Dcm_01670, 01374, 01375);
 * then a TesterPresent is accepted and answered positively (SWS_Dcm_00241, 00251). */
static void converge(void)
{
    static const uint8 tester[2] = {0x3eu, 0x00u};
    Dcm_SesCtrlType session = 0u;
    unsigned i;
    behave(0x06u);
    if (rx_open) {
        indicate(RX, E_NOT_OK);
    }
    if (tx_pending) {
        confirm(TX, E_NOT_OK);
    }
    enter(MODE_FULL, CHANNEL);
    for (i = 0u; i < SETTLE + config.s3_ticks; ++i) {
        main_function();
    }
    CHECK(!tx_pending && !comm_active);
    CHECK(Dcm_GetSesCtrlType(&session) == E_OK && session == DCM_DEFAULT_SESSION);
    CHECK(session_mode == RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION);
    receive(tester, sizeof(tester));
    CHECK(request_current && request_waiting);
    main_function();
    CHECK(!request_current && request_answered && tx_length == 2u);
    CHECK(response[0] == 0x7eu && response[1] == 0x00u && !comm_active);
}

int DcmFuzz_TestOneInput(const uint8 *data, size_t size)
{
    input = data;
    input_size = size;
    input_at = 0u;
    config = defaults;
    configure();
    behave(0x06u);
    comm_active = FALSE;
    restart();
    enter(MODE_FULL, CHANNEL);
    operate();
    converge();
    return 0;
}
