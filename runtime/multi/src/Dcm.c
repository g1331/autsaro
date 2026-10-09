#include "Dcm.h"
#include "Dcm_ComM.h"
#include "PduR_Dcm.h"
#include "ComM_Dcm.h"
#include "SchM_Dcm.h"
#define DCM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
typedef enum { DCM_IDLE, DCM_RECEIVING, DCM_REQUEST, DCM_RESPONSE, DCM_TRANSMITTING } Dcm_StateType;
static const Dcm_ConfigType *configuration DCM_VAR_CLEARED;
static Dcm_StateType state DCM_VAR_CLEARED;
static uint8 request[256] DCM_VAR_CLEARED;
static uint8 response[256] DCM_VAR_CLEARED;
static PduLengthType request_length DCM_VAR_CLEARED;
static PduLengthType received DCM_VAR_CLEARED;
static PduLengthType response_length DCM_VAR_CLEARED;
static PduLengthType copied DCM_VAR_CLEARED;
static PduLengthType confirmed DCM_VAR_CLEARED;
static boolean active_enabled DCM_VAR_CLEARED;
static boolean diagnostic DCM_VAR_CLEARED;
static uint8 communication DCM_VAR_CLEARED;
static Dcm_SesCtrlType session DCM_VAR_CLEARED;
static Dcm_SesCtrlType pending_session DCM_VAR_CLEARED;
static boolean session_change_cancelled DCM_VAR_CLEARED;
static uint32 s3_remaining DCM_VAR_CLEARED;
static uint16 p2_remaining DCM_VAR_CLEARED;
#define DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Dcm_MemMap.h"
#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
static DCM_CODE void Dcm_Release(void) {
    state = DCM_IDLE;
    copied = 0u;
    confirmed = 0u;
    pending_session = 0u;
    session_change_cancelled = FALSE;
    if ((diagnostic == TRUE) && (session == 1u)) {
        diagnostic = FALSE;
        ComM_DCM_InactiveDiagnostic(configuration->channel);
    }
}
static DCM_CODE void Dcm_SetSession(Dcm_SesCtrlType next) {
    const Rte_ModeType_DcmDiagnosticSessionControl mode =
        (next == DCM_DEFAULT_SESSION)
            ? RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION
            : RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION;
    session = next;
    (void)SchM_Switch_Dcm_DcmDiagnosticSessionControl(mode);
}
DCM_CODE void Dcm_Init(const Dcm_ConfigType *ConfigPtr) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((ConfigPtr != NULL_PTR) && (ConfigPtr->read != NULL_PTR) && (ConfigPtr->p2_ticks > 0u) &&
        (ConfigPtr->s3_ticks > 0u) && (ConfigPtr->buffer_length >= 7u) &&
        (ConfigPtr->buffer_length <= 256u) && (ConfigPtr->did != 0xf186u)) {
        if ((configuration != NULL_PTR) && (diagnostic == TRUE)) {
            ComM_DCM_InactiveDiagnostic(configuration->channel);
        }
        configuration = ConfigPtr;
        state = DCM_IDLE;
        active_enabled = TRUE;
        diagnostic = FALSE;
        communication = 0u;
        Dcm_SetSession(DCM_DEFAULT_SESSION);
        pending_session = 0u;
        session_change_cancelled = FALSE;
        s3_remaining = 0u;
        copied = 0u;
        confirmed = 0u;
    }
    SchM_Exit_Dcm_DCM_STATE();
}
DCM_CODE Std_ReturnType Dcm_GetSecurityLevel(Dcm_SecLevelType *SecLevel) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (SecLevel != NULL_PTR)) {
        /* No SecurityAccess service is selected; no transition can unlock it. */
        *SecLevel = DCM_SEC_LEV_LOCKED;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_GetSesCtrlType(Dcm_SesCtrlType *SesCtrlType) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (SesCtrlType != NULL_PTR)) {
        *SesCtrlType = session;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_ResetToDefaultSession(void) {
    SchM_Enter_Dcm_DCM_STATE();
    if (configuration != NULL_PTR) {
        Dcm_SetSession(DCM_DEFAULT_SESSION);
        pending_session = 0u; /* An older queued 0x10 response cannot undo the reset. */
        session_change_cancelled = (state != DCM_IDLE);
        s3_remaining = 0u;
        if ((state == DCM_IDLE) && (diagnostic == TRUE)) {
            Dcm_Release();
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE Std_ReturnType Dcm_SetActiveDiagnostic(boolean active) {
    SchM_Enter_Dcm_DCM_STATE();
    if (configuration != NULL_PTR) {
        active_enabled = active;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return E_OK;
}
DCM_CODE BufReq_ReturnType Dcm_StartOfReception(PduIdType id, const PduInfoType *info,
                                                PduLengthType TpSduLength,
                                                PduLengthType *bufferSizePtr) {
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (id == configuration->receive) && (communication != 0u) &&
        (state == DCM_IDLE) && (bufferSizePtr != NULL_PTR) && (TpSduLength > 0u) &&
        ((info == NULL_PTR) ||
         ((info->MetaDataPtr == NULL_PTR) && (info->SduLength <= TpSduLength) &&
          ((info->SduLength == 0u) || (info->SduDataPtr != NULL_PTR))))) {
        if (TpSduLength > configuration->buffer_length) {
            result = BUFREQ_E_OVFL;
        } else {
            /* Start reserves the whole message; CopyRxData supplies its bytes. */
            request_length = TpSduLength;
            received = 0u;
            state = DCM_RECEIVING;
            session_change_cancelled = FALSE;
            *bufferSizePtr = configuration->buffer_length;
            result = BUFREQ_OK;
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
    return result;
}
DCM_CODE BufReq_ReturnType Dcm_CopyRxData(PduIdType id, const PduInfoType *info,
                                          PduLengthType *bufferSizePtr) {
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (id == configuration->receive) && (communication != 0u) &&
        (state == DCM_RECEIVING) && (info != NULL_PTR) && (bufferSizePtr != NULL_PTR) &&
        ((info->SduLength == 0u) || (info->SduDataPtr != NULL_PTR)) &&
        (info->SduLength <= (request_length - received))) {
        PduLengthType i;
        for (i = 0u; i < info->SduLength; ++i) {
            request[received + i] = info->SduDataPtr[i];
        }
        received += info->SduLength;
        *bufferSizePtr = configuration->buffer_length - received;
        result = BUFREQ_OK;
    }
    SchM_Exit_Dcm_DCM_STATE();
    return result;
}
DCM_CODE void Dcm_TpRxIndication(PduIdType id, Std_ReturnType result) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (id == configuration->receive) && (state == DCM_RECEIVING)) {
        if ((result == E_OK) && (received == request_length) && (communication != 0u)) {
            state = DCM_REQUEST;
            diagnostic = TRUE;
            s3_remaining = configuration->s3_ticks;
            p2_remaining = configuration->p2_ticks;
            if (active_enabled == TRUE) {
                ComM_DCM_ActiveDiagnostic(configuration->channel);
            }
        } else {
            Dcm_Release();
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
}
static DCM_CODE void Dcm_Negative(uint8 code) {
    response[0] = 0x7fu;
    response[1] = request[0];
    response[2] = code;
    response_length = 3u;
}
static DCM_CODE void Dcm_Process(void) {
    boolean suppress = FALSE;
    response_length = 0u;
    if (((request[0] == 0x10u) || (request[0] == 0x3eu)) && (request_length >= 2u)) {
        suppress = (request[1] & 0x80u) != 0u;
        request[1] &= 0x7fu;
    }
    if (request[0] == 0x22u) {
        if ((request_length < 3u) || ((request_length % 2u) == 0u)) {
            Dcm_Negative(0x13u);
        } else {
            PduLengthType i;
            response[0] = 0x62u;
            response_length = 1u;
            for (i = 1u; (i < request_length) && (response[0] == 0x62u); i += 2u) {
                const uint16 did = ((uint16)request[i] * 256u) + request[i + 1u];
                if (did == 0xf186u) {
                    if (response_length > (configuration->buffer_length - 3u)) {
                        Dcm_Negative(0x14u);
                    } else {
                        response[response_length] = 0xf1u;
                        response[response_length + 1u] = 0x86u;
                        response[response_length + 2u] = session;
                        response_length += 3u;
                    }
                } else if (did == configuration->did) {
                    if (response_length > (configuration->buffer_length - 6u)) {
                        Dcm_Negative(0x14u);
                    } else {
                        response[response_length] = request[i];
                        response[response_length + 1u] = request[i + 1u];
                        if (configuration->read(&response[response_length + 2u]) == E_OK) {
                            response_length += 6u;
                        } else {
                            Dcm_Negative(0x22u);
                        }
                    }
                }
            }
            if (response_length == 1u) {
                Dcm_Negative(0x31u);
            }
        }
    } else if (request[0] == 0x10u) {
        if (request_length != 2u) {
            Dcm_Negative(0x13u);
        } else if ((request[1] != 1u) && (request[1] != 3u)) {
            Dcm_Negative(0x12u);
        } else {
            if (session_change_cancelled == FALSE) {
                pending_session = request[1];
            }
            response[0] = 0x50u;
            response[1] = request[1];
            response[2] = (uint8)(configuration->p2_ms / 256u);
            response[3] = (uint8)(configuration->p2_ms % 256u);
            response[4] = (uint8)((configuration->p2_star_ms / 10u) / 256u);
            response[5] = (uint8)((configuration->p2_star_ms / 10u) % 256u);
            response_length = 6u;
        }
    } else if (request[0] == 0x3eu) {
        if (request_length != 2u) {
            Dcm_Negative(0x13u);
        } else if (request[1] != 0u) {
            Dcm_Negative(0x12u);
        } else {
            response[0] = 0x7eu;
            response[1] = 0u;
            response_length = 2u;
        }
    } else {
        Dcm_Negative(0x11u);
    }
    if ((suppress == TRUE) && (response_length != 0u) && (response[0] != 0x7fu)) {
        /* Suppressed positives still complete DSP processing (00238/00240). */
        if (pending_session != 0u) {
            Dcm_SetSession(pending_session);
        }
        response_length = 0u;
        Dcm_Release();
    }
    if (response_length != 0u) {
        state = DCM_RESPONSE;
        copied = 0u;
        confirmed = 0u;
    }
}
DCM_CODE void Dcm_MainFunction(void) {
    SchM_Enter_Dcm_DCM_STATE();
    if (configuration != NULL_PTR) {
        if ((session != 1u) && (state == DCM_IDLE) && (s3_remaining > 0u)) {
            --s3_remaining;
            if (s3_remaining == 0u) {
                Dcm_SetSession(DCM_DEFAULT_SESSION);
                Dcm_Release();
            }
        }
        if (state == DCM_REQUEST) {
            Dcm_Process();
        }
        if (state == DCM_RESPONSE) {
            if (communication == 2u) {
                const PduInfoType info = {NULL_PTR, NULL_PTR, response_length};
                /* Publish before calling lower: confirmation can be synchronous. */
                state = DCM_TRANSMITTING;
                if (PduR_DcmTransmit(configuration->transmit, &info) != E_OK) {
                    state = DCM_RESPONSE;
                }
            }
            if ((state == DCM_RESPONSE) && (p2_remaining > 0u)) {
                --p2_remaining;
                if (p2_remaining == 0u) {
                    Dcm_Release();
                }
            }
        }
    }
    SchM_Exit_Dcm_DCM_STATE();
}
DCM_CODE BufReq_ReturnType Dcm_CopyTxData(PduIdType id, const PduInfoType *info,
                                          const RetryInfoType *retry,
                                          PduLengthType *availableDataPtr) {
    BufReq_ReturnType result = BUFREQ_E_NOT_OK;
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (id == configuration->transmit) &&
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
DCM_CODE void Dcm_TpTxConfirmation(PduIdType id, Std_ReturnType result) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (id == configuration->transmit) &&
        (state == DCM_TRANSMITTING)) {
        if ((result == E_OK) && (pending_session != 0u)) {
            Dcm_SetSession(pending_session);
        }
        Dcm_Release();
    }
    SchM_Exit_Dcm_DCM_STATE();
}
static DCM_CODE void Dcm_Mode(uint8 NetworkId, uint8 mode) {
    SchM_Enter_Dcm_DCM_STATE();
    if ((configuration != NULL_PTR) && (NetworkId == configuration->channel)) {
        communication = mode;
    }
    SchM_Exit_Dcm_DCM_STATE();
}
DCM_CODE void Dcm_ComM_NoComModeEntered(uint8 NetworkId) { Dcm_Mode(NetworkId, 0u); }
DCM_CODE void Dcm_ComM_SilentComModeEntered(uint8 NetworkId) { Dcm_Mode(NetworkId, 1u); }
DCM_CODE void Dcm_ComM_FullComModeEntered(uint8 NetworkId) { Dcm_Mode(NetworkId, 2u); }
#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
