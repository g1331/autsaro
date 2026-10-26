/** @file One physical UDS connection; state in DCM_STATE, calls and publication outside it. */
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "PduR_Dcm.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PDUR_AR_RELEASE_MAJOR_VERSION) || !defined(PDUR_AR_RELEASE_MINOR_VERSION)
#error "Dcm.c -> PduR_Dcm.h: missing AUTOSAR release"
#elif (PDUR_AR_RELEASE_MAJOR_VERSION != 4U) || (PDUR_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm.c -> PduR_Dcm.h: incompatible AUTOSAR release"
#endif
#include "ComM_Dcm.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMM_AR_RELEASE_MAJOR_VERSION) || !defined(COMM_AR_RELEASE_MINOR_VERSION)
#error "Dcm.c -> ComM_Dcm.h: missing AUTOSAR release"
#elif (COMM_AR_RELEASE_MAJOR_VERSION != 4U) || (COMM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm.c -> ComM_Dcm.h: incompatible AUTOSAR release"
#endif
#include "SchM_Dcm.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "Dcm.c -> SchM_Dcm.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm.c -> SchM_Dcm.h: incompatible AUTOSAR release"
#endif
#define DCM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
#define DCM_BUFFER_CAPACITY 256u
#define DCM_SESSION_DID     0xf186u
#define DCM_OBD_DATA        0xf400u
#define DCM_OBD_INFO        0xf800u
#define DCM_OBD_BLOCK       32u
#define DCM_DID_FAMILY      0xff00u
#define DCM_SID_SESSION     0x10u
#define DCM_SID_READ        0x22u
#define DCM_SID_TESTER      0x3eu
#define DCM_NEGATIVE        0x7fu
#define DCM_POSITIVE_READ   0x62u
#define DCM_POSITIVE_DSC    0x50u
#define DCM_POSITIVE_TP     0x7eu
#define DCM_DID_DATA_LENGTH 4u
#define DCM_SPRMIB          0x80u
#define DCM_SUBFUNCTION     0x7fu
#define DCM_NO_COM          0u
#define DCM_SILENT_COM      1u
#define DCM_FULL_COM        2u

typedef enum {
    DCM_IDLE,
    DCM_RECEIVING,
    DCM_REQUEST,
    DCM_PROCESSING,
    DCM_RESPONSE,
    DCM_TRANSMITTING
} Dcm_StateType;
typedef enum {
    DCM_PUBLISH_NONE,
    DCM_PUBLISH_INACTIVE,
    DCM_PUBLISH_ACTIVE,
    DCM_PUBLISH_AUTHENTICATION,
    DCM_PUBLISH_SESSION,
    DCM_PUBLISH_SKIP
} Dcm_PublicationKindType;
typedef struct {
    Dcm_PublicationKindType kind;
    NetworkHandleType channel;
    Rte_ModeType_DcmDiagnosticSessionControl mode;
    Ecu_DcmAuthenticationModeType authentication;
} Dcm_PublicationType;
/* A synchronous ReadData writes only this executor's storage. Init may admit a new
 * request meanwhile, but cannot run another ReadData before this executor returns. */
typedef struct {
    const Dcm_ConfigType *configuration;
    uint32 generation;
    uint8 request[DCM_BUFFER_CAPACITY];
    uint8 response[DCM_BUFFER_CAPACITY];
    Dcm_MsgContextType message;
    Dcm_SesCtrlType session;
    Dcm_SesCtrlType pending_session;
    boolean cancelled;
} Dcm_WorkType;
/* All fields below are protected by DCM_STATE. Init resets the connection, but
 * preserves in-flight executors and publishers: reentrant callbacks must not start
 * a second non-reentrant ReadData or publish an older mode after a newer one. */
static const Dcm_ConfigType *dcm_configuration
    DCM_VAR_CLEARED;                        /**< Selected immutable configuration. */
static Dcm_StateType state DCM_VAR_CLEARED; /**< Current request phase; Init resets. */
static uint8
    request[DCM_BUFFER_CAPACITY] DCM_VAR_CLEARED; /**< Rx bytes; Init retires their length. */
static uint8
    response[DCM_BUFFER_CAPACITY] DCM_VAR_CLEARED;    /**< Tx bytes; owned until confirmation. */
static PduLengthType request_length DCM_VAR_CLEARED;  /**< Reserved Rx size; Init resets. */
static PduLengthType received DCM_VAR_CLEARED;        /**< Copied Rx bytes; Init resets. */
static PduLengthType response_length DCM_VAR_CLEARED; /**< Completed Tx size; Init resets. */
static PduLengthType copied DCM_VAR_CLEARED;          /**< Tx copy cursor; Init resets. */
static PduLengthType confirmed DCM_VAR_CLEARED;       /**< Tx retry boundary; Init resets. */
static boolean active_enabled DCM_VAR_CLEARED;        /**< ActiveDiagnostic switch; Init enables. */
static boolean dcm_diagnostic DCM_VAR_CLEARED;        /**< Diagnostic demand; Init clears. */
static uint8 communication DCM_VAR_CLEARED;           /**< Channel gate; Init selects NoCom. */
static Dcm_SesCtrlType session DCM_VAR_CLEARED;       /**< Active session; Init selects default. */
static Dcm_SesCtrlType pending_session
    DCM_VAR_CLEARED; /**< Session awaiting successful completion. */
static boolean session_change_cancelled
    DCM_VAR_CLEARED;                          /**< Reset veto for this request; Init clears. */
static uint32 s3_remaining DCM_VAR_CLEARED;   /**< Idle session ticks; Init resets. */
static uint16 p2_remaining DCM_VAR_CLEARED;   /**< FullCom wait ticks; Init resets. */
static uint32 dcm_generation DCM_VAR_CLEARED; /**< Request identity; Init and Rx advance. */
/* The TP lifetime of the one admitted response. Only the Main executor admits, and
 * only from DCM_LOWER_IDLE, so DCM_TRANSMITTING implies CALLING or AWAITING: Init
 * moves the request out of DCM_TRANSMITTING and no new request can return to it
 * before this lifetime ends. Init cannot recycle a call still on the stack; it
 * retires only AWAITING and requires that old TP lifetime to be quiesced. */
typedef enum {
    DCM_LOWER_IDLE,    /**< No reserved TP identity. */
    DCM_LOWER_CALLING, /**< PduR_DcmTransmit is on the stack. */
    DCM_LOWER_SETTLED, /**< Confirmed before PduR_DcmTransmit returned. */
    DCM_LOWER_AWAITING /**< Accepted; TpTxConfirmation is still due. */
} Dcm_LowerType;
static Dcm_LowerType lower DCM_VAR_CLEARED;      /**< Reserved TP lifetime; see above. */
static PduIdType lower_transmit DCM_VAR_CLEARED; /**< Reserved confirmation ID; Init preserves. */
/* Publication revisions are monotonic. One publisher drains the latest state;
 * a callback may change it, but may not recursively flush it out of order. */
static boolean dcm_publishing DCM_VAR_CLEARED;  /**< Publication owner; Init preserves. */
static uint32 initialization DCM_VAR_CLEARED;   /**< Authentication revision; Init advances. */
static uint32 session_revision DCM_VAR_CLEARED; /**< Session publication; Init advances. */
static uint32 pending_active DCM_VAR_CLEARED;   /**< Per-request effects; publisher drains each. */
static uint32 diagnostic_revision DCM_VAR_CLEARED; /**< Every request/transition; Init advances. */
/* These actors and their publication cursors outlive Init. Resetting them could
 * overlap synchronous application calls or let an old publisher overtake a new one. */
typedef struct {
    boolean executing;                   /**< Main executor is in flight. */
    uint32 published_diagnostic;         /**< Last demand revision. */
    uint32 published_session;            /**< Last session revision. */
    uint32 published_initialization;     /**< Last authentication revision. */
    NetworkHandleType published_channel; /**< Last demand channel. */
    boolean published_active;            /**< Last demand state. */
} Dcm_ActorsType;
static Dcm_ActorsType dcm_actors DCM_VAR_CLEARED; /**< DCM_STATE; preserved by Init. */
#define DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Dcm.c -> Dcm_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm.c -> Dcm_MemMap.h: incompatible AUTOSAR release"
#endif
#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
static DCM_CODE void Dcm_SetSession(Dcm_SesCtrlType next)
{
    session = next;
    ++session_revision;
}
static DCM_CODE void Dcm_Release(void)
{
    state = DCM_IDLE;
    copied = 0u;
    confirmed = 0u;
    pending_session = 0u;
    session_change_cancelled = FALSE;
    s3_remaining = (session == DCM_DEFAULT_SESSION) ? 0u : dcm_configuration->s3_ticks;
    if ((dcm_diagnostic == TRUE) && (session == DCM_DEFAULT_SESSION)) {
        dcm_diagnostic = FALSE;
        ++diagnostic_revision;
    }
}
/* Called only while holding DCM_STATE; claims each effect before its external call. */
static DCM_CODE Dcm_PublicationType Dcm_NextPublication(void)
{
    Dcm_PublicationType effect = {DCM_PUBLISH_NONE, 0u, 0u, NULL_PTR};
    if (pending_active > 0u) {
        --pending_active;
        effect.kind = DCM_PUBLISH_ACTIVE;
        effect.channel = dcm_configuration->channel;
        dcm_actors.published_active = TRUE;
        dcm_actors.published_channel = effect.channel;
    } else if ((dcm_actors.published_active == TRUE) &&
               ((dcm_diagnostic == FALSE) ||
                (dcm_actors.published_channel != dcm_configuration->channel))) {
        effect.kind = DCM_PUBLISH_INACTIVE;
        effect.channel = dcm_actors.published_channel;
        dcm_actors.published_active = FALSE;
    } else if (dcm_actors.published_initialization != initialization) {
        effect.kind = DCM_PUBLISH_AUTHENTICATION;
        effect.authentication = dcm_configuration->authentication_mode;
        dcm_actors.published_initialization = initialization;
    } else if (dcm_actors.published_session != session_revision) {
        effect.kind = DCM_PUBLISH_SESSION;
        effect.mode = (session == DCM_DEFAULT_SESSION)
                          ? RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION
                          : RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION;
        dcm_actors.published_session = session_revision;
    } else if (dcm_actors.published_diagnostic != diagnostic_revision) {
        dcm_actors.published_diagnostic = diagnostic_revision;
        dcm_actors.published_active = dcm_diagnostic;
        dcm_actors.published_channel = dcm_configuration->channel;
        effect.channel = dcm_actors.published_channel;
        effect.kind = DCM_PUBLISH_SKIP;
    } else {
        dcm_publishing = FALSE;
    }
    return effect;
}
static DCM_CODE void Dcm_Publish(void)
{
    boolean owner = FALSE;
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (dcm_publishing == FALSE)) {
        dcm_publishing = TRUE;
        owner = TRUE;
    }
    SchM_Exit_Dcm_DCM_STATE();
    while (owner == TRUE) {
        Dcm_PublicationType effect;
        SchM_Enter_Dcm_DCM_STATE();
        effect = Dcm_NextPublication();
        SchM_Exit_Dcm_DCM_STATE();
        switch (effect.kind) {
            case DCM_PUBLISH_INACTIVE:
                ComM_DCM_InactiveDiagnostic(effect.channel);
                break;
            case DCM_PUBLISH_ACTIVE:
                ComM_DCM_ActiveDiagnostic(effect.channel);
                break;
            case DCM_PUBLISH_AUTHENTICATION:
                effect.authentication(0u);
                break;
            case DCM_PUBLISH_SESSION:
                /* The generated immediate switch serializes its own state in every context. */
                (void)SchM_Switch_Dcm_DcmDiagnosticSessionControl(effect.mode);
                break;
            case DCM_PUBLISH_SKIP:
                break;
            default:
                owner = FALSE;
                break;
        }
    }
}
/* ISO availability identifiers occupy every 0x20th position in these OBD DID families. */
static DCM_CODE boolean Dcm_IsAvailability(uint16 did)
{
    return (((did & DCM_DID_FAMILY) == DCM_OBD_DATA) || ((did & DCM_DID_FAMILY) == DCM_OBD_INFO)) &&
           ((did % DCM_OBD_BLOCK) == 0u);
}
DCM_CODE void Dcm_Init(const Dcm_ConfigType *ConfigPtr)
{
    SchM_Enter_Dcm_DCM_STATE();
    if ((ConfigPtr != NULL_PTR) && (ConfigPtr->p2_ticks > 0u) && (ConfigPtr->s3_ticks > 0u) &&
        (ConfigPtr->buffer_length >= 8u) && (ConfigPtr->buffer_length <= DCM_BUFFER_CAPACITY) &&
        ((PduLengthType)ConfigPtr->buffer_length == ConfigPtr->buffer_length) &&
        (ConfigPtr->did != DCM_SESSION_DID) && (Dcm_IsAvailability(ConfigPtr->did) == FALSE) &&
        (ConfigPtr->authentication_mode != NULL_PTR)) {
        dcm_configuration = ConfigPtr;
        ++dcm_generation;
        ++initialization;
        ++diagnostic_revision;
        state = DCM_IDLE;
        active_enabled = TRUE;
        dcm_diagnostic = FALSE;
        pending_active = 0u;
        communication = DCM_NO_COM;
        Dcm_SetSession(DCM_DEFAULT_SESSION);
        pending_session = 0u;
        session_change_cancelled = FALSE;
        s3_remaining = 0u;
        p2_remaining = 0u;
        request_length = 0u;
        response_length = 0u;
        received = 0u;
        copied = 0u;
        confirmed = 0u;
        if (lower == DCM_LOWER_AWAITING) {
            lower = DCM_LOWER_IDLE;
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    Dcm_Publish();
}
DCM_CODE Std_ReturnType Dcm_GetSecurityLevel(Dcm_SecLevelType *SecLevel)
{
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (SecLevel != NULL_PTR)) {
        *SecLevel = DCM_SEC_LEV_LOCKED;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_GetSesCtrlType(Dcm_SesCtrlType *SesCtrlType)
{
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (SesCtrlType != NULL_PTR)) {
        *SesCtrlType = session;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_ResetToDefaultSession(void)
{
    SchM_Enter_Dcm_DCM_STATE();
    if (dcm_configuration != NULL_PTR) {
        Dcm_SetSession(DCM_DEFAULT_SESSION);
        pending_session = 0u;
        session_change_cancelled = (state != DCM_IDLE);
        s3_remaining = 0u;
        if ((state == DCM_IDLE) && (dcm_diagnostic == TRUE)) {
            Dcm_Release();
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    Dcm_Publish();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_SetActiveDiagnostic(boolean active)
{
    SchM_Enter_Dcm_DCM_STATE();
    if (dcm_configuration != NULL_PTR) {
        active_enabled = active;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE BufReq_ReturnType Dcm_StartOfReception(PduIdType id,
                                                const PduInfoType *info,
                                                PduLengthType TpSduLength,
                                                PduLengthType *bufferSizePtr)
{
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (id == dcm_configuration->receive) &&
        (communication != DCM_NO_COM) && (bufferSizePtr != NULL_PTR) && (TpSduLength > 0u) &&
        ((info == NULL_PTR) ||
         ((info->MetaDataPtr == NULL_PTR) && (info->SduLength <= TpSduLength) &&
          ((info->SduLength == 0u) || (info->SduDataPtr != NULL_PTR))))) {
        if (state == DCM_IDLE) {
            if (TpSduLength > dcm_configuration->buffer_length) {
                result = BUFREQ_E_OVFL;
            } else {
                ++dcm_generation;
                request_length = TpSduLength;
                received = 0u;
                state = DCM_RECEIVING;
                session_change_cancelled = FALSE;
                *bufferSizePtr = (PduLengthType)dcm_configuration->buffer_length;
                result = BUFREQ_OK;
            }
        } else {
            /* SWS_Dcm_00241/01678: a busy physical connection declines further requests;
             * concurrent TesterPresent is functional-only and that addressing is refused. */
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    return result;
}
DCM_CODE BufReq_ReturnType Dcm_CopyRxData(PduIdType id,
                                          const PduInfoType *info,
                                          PduLengthType *bufferSizePtr)
{
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (id == dcm_configuration->receive) &&
        (communication != DCM_NO_COM) && (info != NULL_PTR) && (bufferSizePtr != NULL_PTR) &&
        ((info->SduLength == 0u) || (info->SduDataPtr != NULL_PTR))) {
        if ((state == DCM_RECEIVING) && (info->SduLength <= (request_length - received))) {
            PduLengthType i;
            for (i = 0u; i < info->SduLength; ++i) {
                request[received + i] = info->SduDataPtr[i];
            }
            received += info->SduLength;
            *bufferSizePtr = (PduLengthType)(dcm_configuration->buffer_length - received);
            result = BUFREQ_OK;
        } else {
            /* A rejected copy preserves both the destination and the size output. */
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    return result;
}
DCM_CODE void Dcm_TpRxIndication(PduIdType id, Std_ReturnType result)
{
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (id == dcm_configuration->receive)) {
        if (state == DCM_RECEIVING) {
            if ((result == E_OK) && (received == request_length) && (communication != DCM_NO_COM)) {
                state = DCM_REQUEST;
                dcm_diagnostic = TRUE;
                if (active_enabled == TRUE) {
                    ++pending_active;
                }
                ++diagnostic_revision;
                p2_remaining = dcm_configuration->p2_ticks;
            } else {
                Dcm_Release();
            }
        } else {
            /* Late receive completion cannot process a retired request. */
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    Dcm_Publish();
}
static DCM_CODE void Dcm_Negative(Dcm_WorkType *work, Dcm_NegativeResponseCodeType code)
{
    work->response[0] = DCM_NEGATIVE;
    work->response[1] = work->request[0];
    work->response[2] = code;
    work->message.resDataLen = 2u;
}
static DCM_CODE boolean Dcm_Current(uint32 expected)
{
    boolean current;
    SchM_Enter_Dcm_DCM_STATE();
    current = (dcm_generation == expected);
    SchM_Exit_Dcm_DCM_STATE();
    return current;
}
/* An availability DID exists only for a block leading to the configured data DID.
 * Its last bit advertises the next block; it never calls the application ReadData. */
static DCM_CODE boolean Dcm_Available(const Dcm_WorkType *work, uint16 did)
{
    return (Dcm_IsAvailability(did) == TRUE) && (work->configuration->read != NULL_PTR) &&
           ((did & DCM_DID_FAMILY) == (work->configuration->did & DCM_DID_FAMILY)) &&
           (work->configuration->did > did);
}
static DCM_CODE void Dcm_ReadAvailability(Dcm_WorkType *work, uint16 did, Dcm_MsgLenType offset)
{
    const Dcm_MsgLenType distance = (Dcm_MsgLenType)work->configuration->did - did;
    const Dcm_MsgLenType bit = (distance > DCM_OBD_BLOCK) ? (DCM_OBD_BLOCK - 1u) : (distance - 1u);
    PduLengthType i;
    for (i = 0u; i < DCM_DID_DATA_LENGTH; ++i) {
        work->message.resData[offset + 2u + i] = 0u;
    }
    work->message.resData[offset + 2u + (bit / 8u)] = (uint8)(0x80u >> (bit % 8u));
}
static DCM_CODE void Dcm_ReadValue(Dcm_WorkType *work, uint16 did)
{
    const Dcm_MsgLenType size = (did == DCM_SESSION_DID) ? 3u : 6u;
    const Dcm_MsgLenType offset = work->message.resDataLen;
    if (Dcm_Current(work->generation) == FALSE) {
        work->cancelled = TRUE;
    } else {
        work->message.resData[offset] = (uint8)(did / 256u);
        work->message.resData[offset + 1u] = (uint8)(did % 256u);
        if (did == DCM_SESSION_DID) {
            work->message.resData[offset + 2u] = work->session;
            work->message.resDataLen += size;
        } else if (Dcm_Available(work, did) == TRUE) {
            Dcm_ReadAvailability(work, did, offset);
            work->message.resDataLen += size;
        } else if (work->configuration->read(&work->message.resData[offset + 2u]) == E_OK) {
            work->message.resDataLen += size;
        } else {
            /* The application always supplies valid data; an RTE infrastructure
             * failure may still reject its call (00686). Discard partial bytes. */
            Dcm_Negative(work, DCM_E_GENERALREJECT);
        }
    }
}
static DCM_CODE Dcm_MsgLenType Dcm_ReadSize(const Dcm_WorkType *work, uint16 did)
{
    Dcm_MsgLenType size = 0u;
    if (did == DCM_SESSION_DID) {
        size = 3u;
    } else if ((Dcm_Available(work, did) == TRUE) ||
               ((work->configuration->read != NULL_PTR) && (did == work->configuration->did))) {
        size = 6u;
    } else {
        /* Unsupported DIDs do not contribute to a multi-DID response. */
    }
    return size;
}
static DCM_CODE void Dcm_ReadData(Dcm_WorkType *work)
{
    if ((work->message.reqDataLen < 2u) || ((work->message.reqDataLen % 2u) != 0u) ||
        ((work->configuration->max_did_to_read != 0u) &&
         ((work->message.reqDataLen / 2u) > work->configuration->max_did_to_read))) {
        Dcm_Negative(work, DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT);
    } else {
        Dcm_MsgLenType i;
        Dcm_MsgLenType length = 0u;
        /* ISO 14229-1 Figure 15: validate the complete response before reading data. */
        for (i = 0u; i < work->message.reqDataLen; i += 2u) {
            const uint16 did =
                ((uint16)work->message.reqData[i] * 256u) + work->message.reqData[i + 1u];
            length += Dcm_ReadSize(work, did);
        }
        if (length == 0u) {
            Dcm_Negative(work, DCM_E_REQUESTOUTOFRANGE);
        } else if (length > work->message.resMaxDataLen) {
            Dcm_Negative(work, DCM_E_RESPONSETOOLONG);
        } else {
            work->response[0] = DCM_POSITIVE_READ;
            work->message.resDataLen = 0u;
            i = 0u;
            while ((i < work->message.reqDataLen) && (work->response[0] == DCM_POSITIVE_READ) &&
                   (work->cancelled == FALSE)) {
                const uint16 did =
                    ((uint16)work->message.reqData[i] * 256u) + work->message.reqData[i + 1u];
                if (Dcm_ReadSize(work, did) != 0u) {
                    Dcm_ReadValue(work, did);
                }
                i += 2u;
            }
        }
    }
}
static DCM_CODE void Dcm_SessionRequest(Dcm_WorkType *work)
{
    if (work->message.reqDataLen == 0u) {
        Dcm_Negative(work, DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT);
    } else if ((work->message.reqData[0] != DCM_DEFAULT_SESSION) &&
               (work->message.reqData[0] != DCM_EXTENDED_DIAGNOSTIC_SESSION)) {
        Dcm_Negative(work, DCM_E_SUBFUNCTIONNOTSUPPORTED);
    } else if (work->message.reqDataLen != 1u) {
        Dcm_Negative(work, DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT);
    } else {
        work->pending_session = work->message.reqData[0];
        work->response[0] = DCM_POSITIVE_DSC;
        work->message.resData[0] = work->message.reqData[0];
        work->message.resData[1] = (uint8)(work->configuration->p2_ms / 256u);
        work->message.resData[2] = (uint8)(work->configuration->p2_ms % 256u);
        work->message.resData[3] = (uint8)((work->configuration->p2_star_ms / 10u) / 256u);
        work->message.resData[4] = (uint8)((work->configuration->p2_star_ms / 10u) % 256u);
        work->message.resDataLen = 5u;
    }
}
static DCM_CODE void Dcm_Process(Dcm_WorkType *work)
{
    if (((work->request[0] == DCM_SID_SESSION) || (work->request[0] == DCM_SID_TESTER)) &&
        (work->message.reqDataLen >= 1u)) {
        work->message.msgAddInfo.suppressPosResponse =
            ((work->message.reqData[0] & DCM_SPRMIB) != 0u) ? 1u : 0u;
        work->message.reqData[0] &= DCM_SUBFUNCTION;
    }
    if (work->request[0] == DCM_SID_READ) {
        Dcm_ReadData(work);
    } else if (work->request[0] == DCM_SID_SESSION) {
        Dcm_SessionRequest(work);
    } else if (work->request[0] == DCM_SID_TESTER) {
        if (work->message.reqDataLen == 0u) {
            Dcm_Negative(work, DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT);
        } else if (work->message.reqData[0] != 0u) {
            Dcm_Negative(work, DCM_E_SUBFUNCTIONNOTSUPPORTED);
        } else if (work->message.reqDataLen != 1u) {
            Dcm_Negative(work, DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT);
        } else {
            work->response[0] = DCM_POSITIVE_TP;
            work->message.resData[0] = 0u;
            work->message.resDataLen = 1u;
        }
    } else if (((work->request[0] >= 0x40u) && (work->request[0] <= 0x7fu)) ||
               (work->request[0] >= 0xc0u)) {
        /* SWS_Dcm_00084: the selected DcmRespondAllRequest is false. */
    } else {
        Dcm_Negative(work, DCM_E_SERVICENOTSUPPORTED);
    }
}
/* Called inside DCM_STATE; the resulting payloads belong only to this executor. */
static DCM_CODE void Dcm_PrepareWork(Dcm_WorkType *work)
{
    PduLengthType i;
    state = DCM_PROCESSING;
    work->configuration = dcm_configuration;
    work->generation = dcm_generation;
    work->message.reqData = &work->request[1];
    work->message.reqDataLen = (Dcm_MsgLenType)request_length - 1u;
    work->message.resData = &work->response[1];
    work->message.resDataLen = 0u;
    work->message.msgAddInfo.reqType = 0u;
    work->message.msgAddInfo.suppressPosResponse = 0u;
    work->message.resMaxDataLen = (Dcm_MsgLenType)dcm_configuration->buffer_length - 1u;
    work->message.idContext = 0u;
    work->message.dcmRxPduId = dcm_configuration->receive;
    work->response[0] = 0u;
    work->session = session;
    work->pending_session = 0u;
    work->cancelled = FALSE;
    /* Snapshot the SID and subfunction byte explicitly. The length guards below
     * decide whether that byte belongs to this request. Both source slots are static. */
    work->request[0] = request[0];
    work->request[1] = request[1];
    for (i = 2u; i < request_length; ++i) {
        work->request[i] = request[i];
    }
}
static DCM_CODE void Dcm_ProcessRequest(void)
{
    Dcm_WorkType work;
    boolean process = FALSE;
    SchM_Enter_Dcm_DCM_STATE();
    if (state == DCM_REQUEST) {
        Dcm_PrepareWork(&work);
        process = TRUE;
    }
    SchM_Exit_Dcm_DCM_STATE();
    if (process == TRUE) {
        Dcm_Process(&work);
        SchM_Enter_Dcm_DCM_STATE();
        if ((dcm_generation == work.generation) && (state == DCM_PROCESSING)) {
            PduLengthType i;
            pending_session = (session_change_cancelled == TRUE) ? 0u : work.pending_session;
            response_length =
                ((work.response[0] == 0u) || ((work.message.msgAddInfo.suppressPosResponse == 1u) &&
                                              (work.response[0] != DCM_NEGATIVE)))
                    ? 0u
                    : (PduLengthType)(work.message.resDataLen + 1u);
            for (i = 0u; i < response_length; ++i) {
                response[i] = work.response[i];
            }
            if (response_length == 0u) {
                if (pending_session != 0u) {
                    Dcm_SetSession(pending_session);
                }
                Dcm_Release();
            } else {
                state = DCM_RESPONSE;
                copied = 0u;
                confirmed = 0u;
            }
        }
        SchM_Exit_Dcm_DCM_STATE();
    }
}
static DCM_CODE void Dcm_TransmitResponse(void)
{
    boolean send = FALSE;
    PduIdType id = 0u;
    PduInfoType info = {NULL_PTR, NULL_PTR, 0u};
    SchM_Enter_Dcm_DCM_STATE();
    if ((state == DCM_RESPONSE) && (communication == DCM_FULL_COM) && (lower == DCM_LOWER_IDLE)) {
        state = DCM_TRANSMITTING;
        lower = DCM_LOWER_CALLING;
        lower_transmit = dcm_configuration->transmit;
        id = dcm_configuration->router_transmit;
        info.SduLength = response_length;
        send = TRUE;
    }
    SchM_Exit_Dcm_DCM_STATE();
    /* The admission is committed above: a later mode change or Init only affects the next
     * admission, and this call keeps its reserved TP identity until it is confirmed. */
    if (send == TRUE) {
        const Std_ReturnType result = PduR_DcmTransmit(id, &info);
        SchM_Enter_Dcm_DCM_STATE();
        if (lower == DCM_LOWER_SETTLED) {
            lower = DCM_LOWER_IDLE;
        } else if (result == E_OK) {
            lower = DCM_LOWER_AWAITING;
        } else {
            lower = DCM_LOWER_IDLE;
            if (state == DCM_TRANSMITTING) {
                /* SWS_Dcm_00118: neither a refusal nor an error confirmation is retried. */
                Dcm_Release();
            }
        }
        SchM_Exit_Dcm_DCM_STATE();
    }
}
DCM_CODE void Dcm_MainFunction(void)
{
    boolean owner = FALSE;
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (dcm_actors.executing == FALSE) &&
        (dcm_publishing == FALSE)) {
        dcm_actors.executing = TRUE;
        owner = TRUE;
        if ((session != DCM_DEFAULT_SESSION) && (state == DCM_IDLE) && (s3_remaining > 0u)) {
            --s3_remaining;
            if (s3_remaining == 0u) {
                Dcm_SetSession(DCM_DEFAULT_SESSION);
                Dcm_Release();
            }
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    if (owner == TRUE) {
        Dcm_ProcessRequest();
        Dcm_TransmitResponse();
        SchM_Enter_Dcm_DCM_STATE();
        if ((state == DCM_RESPONSE) && (p2_remaining > 0u)) {
            --p2_remaining;
            if (p2_remaining == 0u) {
                Dcm_Release();
            }
        }
        dcm_actors.executing = FALSE;
        SchM_Exit_Dcm_DCM_STATE();
        Dcm_Publish();
    }
}
DCM_CODE BufReq_ReturnType Dcm_CopyTxData(PduIdType id,
                                          const PduInfoType *info,
                                          const RetryInfoType *retry,
                                          PduLengthType *availableDataPtr)
{
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (id == dcm_configuration->transmit) &&
        (state == DCM_TRANSMITTING) && (info != NULL_PTR) && (availableDataPtr != NULL_PTR) &&
        ((info->SduLength == 0u) || (info->SduDataPtr != NULL_PTR))) {
        PduLengthType position = copied;
        PduLengthType acknowledged = confirmed;
        boolean valid = TRUE;
        if (retry != NULL_PTR) {
            if (retry->TpDataState == TP_DATARETRY) {
                if (retry->TxTpDataCnt <= (copied - confirmed)) {
                    position -= retry->TxTpDataCnt;
                } else {
                    valid = FALSE;
                }
            } else if (retry->TpDataState == TP_DATACONF) {
                acknowledged = copied;
            } else if (retry->TpDataState != TP_CONFPENDING) {
                valid = FALSE;
            } else {
                /* Pending data remains available for a bounded retry. */
            }
        } else {
            acknowledged = copied;
        }
        if (valid == TRUE) {
            if (info->SduLength > (response_length - position)) {
                result = BUFREQ_E_BUSY;
            } else {
                PduLengthType i;
                for (i = 0u; i < info->SduLength; ++i) {
                    info->SduDataPtr[i] = response[position + i];
                }
                copied = position + info->SduLength;
                confirmed = (retry == NULL_PTR) ? copied : acknowledged;
                *availableDataPtr = response_length - copied;
                result = BUFREQ_OK;
            }
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    return result;
}
DCM_CODE void Dcm_TpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    SchM_Enter_Dcm_DCM_STATE();
    if (((lower == DCM_LOWER_CALLING) || (lower == DCM_LOWER_AWAITING)) && (id == lower_transmit)) {
        lower = (lower == DCM_LOWER_CALLING) ? DCM_LOWER_SETTLED : DCM_LOWER_IDLE;
        if (state == DCM_TRANSMITTING) {
            if ((result == E_OK) && (pending_session != 0u)) {
                Dcm_SetSession(pending_session);
            }
            Dcm_Release();
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    Dcm_Publish();
}
static DCM_CODE void Dcm_Mode(uint8 NetworkId, uint8 mode)
{
    SchM_Enter_Dcm_DCM_STATE();
    if ((dcm_configuration != NULL_PTR) && (NetworkId == dcm_configuration->channel)) {
        communication = mode;
    }
    SchM_Exit_Dcm_DCM_STATE();
}
DCM_CODE void Dcm_ComM_NoComModeEntered(uint8 NetworkId) { Dcm_Mode(NetworkId, DCM_NO_COM); }
DCM_CODE void Dcm_ComM_SilentComModeEntered(uint8 NetworkId)
{
    Dcm_Mode(NetworkId, DCM_SILENT_COM);
}
DCM_CODE void Dcm_ComM_FullComModeEntered(uint8 NetworkId) { Dcm_Mode(NetworkId, DCM_FULL_COM); }
#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
