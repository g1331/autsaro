#include "Dcm.h"
#include "Dem.h"
#include "PduR.h"
#include "Security.h"
#include "Dcm_Internal.h"
#include "Ecu_Execution.h"

static const EcuDiagnosticConfig *dcm_config;
static uint8_t active_session;
static uint8_t pending_session;
static uint64_t last_request_ms;


static EcuStatus NegativeResponse(uint8_t service, uint8_t code, uint64_t now_ms) {
    const uint8_t response[3] = {0x7fu, service, code};
    return PduR_DcmTransmit(response, sizeof(response), now_ms);
}

void Dcm_Init(const EcuDiagnosticConfig *config) {
    dcm_config = config;
    active_session = 0x01u;
    pending_session = 0u;
    last_request_ms = 0u;
}

void Dcm_TpTxConfirmation(EcuStatus status, uint64_t now_ms) {
    if (pending_session != 0u) {
        if (status == ECU_OK) {
            if (active_session != 0x01u) {
                Security_Lock();
            }
            active_session = pending_session;
            if ((active_session == 0x01u) && (dcm_config != NULL) && (dcm_config->dtc != NULL)) {
                (void)Dem_EnableDTCSetting();
            }
            last_request_ms = now_ms;
        }
        pending_session = 0u;
    }
}

void Dcm_AdvanceTime(uint64_t now_ms) {
    if ((dcm_config != NULL) && (active_session != 0x01u) &&
        ((now_ms - last_request_ms) >= dcm_config->s3_ms)) {
        active_session = 0x01u;
        Security_Lock();
        if (dcm_config->dtc != NULL) {
            (void)Dem_EnableDTCSetting();
        }
        pending_session = 0u;
    }
}

static EcuStatus HandleSessionControl(const uint8_t *request, size_t length, uint64_t now_ms) {
    EcuStatus result;
    if (length != 2u) {
        result = NegativeResponse(0x10u, 0x13u, now_ms);
    } else if ((request[1] != 0x01u) && (request[1] != 0x03u)) {
        result = NegativeResponse(0x10u, 0x12u, now_ms);
    } else {
        uint8_t response[6];
        response[0] = 0x50u;
        response[1] = request[1];
        response[2] = (uint8_t)(Ecu_Policy.p2_ms / 256u);
        response[3] = (uint8_t)(Ecu_Policy.p2_ms % 256u);
        response[4] = (uint8_t)((Ecu_Policy.p2_star_ms / 10u) / 256u);
        response[5] = (uint8_t)((Ecu_Policy.p2_star_ms / 10u) % 256u);
        pending_session = request[1];
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
        if (result != ECU_OK) {
            pending_session = 0u;
        }
    }
    return result;
}

static EcuStatus HandleTesterPresent(const uint8_t *request, size_t length, uint64_t now_ms) {
    const uint8_t response[2] = {0x7eu, 0x00u};
    EcuStatus result;
    if (length != 2u) {
        result = NegativeResponse(0x3eu, 0x13u, now_ms);
    } else if (request[1] == 0x80u) {
        result = ECU_OK;
    } else if (request[1] != 0x00u) {
        result = NegativeResponse(0x3eu, 0x12u, now_ms);
    } else {
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    }
    return result;
}

static EcuStatus HandleSecurityAccess(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t response[2u + ECU_SECURITY_SEED_SIZE];
    uint8_t code = 0u;
    EcuStatus result;
    if (dcm_config->security_enabled == 0u) {
        code = 0x11u;
    } else if (length < 2u) {
        code = 0x13u;
    } else if (active_session != 0x03u) {
        code = 0x7fu;
    } else if ((request[1] != 0x01u) && (request[1] != 0x02u)) {
        code = 0x12u;
    } else if (length != ((request[1] == 0x01u) ? 2u : (2u + ECU_SECURITY_KEY_SIZE))) {
        code = 0x13u;
    } else {
        response[0] = 0x67u;
        response[1] = request[1];
        if (request[1] == 0x01u) {
            code = Security_RequestSeed(&response[2], now_ms);
        } else {
            code = Security_SendKey(&request[2], now_ms);
        }
    }
    if (code != 0u) {
        result = NegativeResponse(0x27u, code, now_ms);
    } else if (request[1] == 0x01u) {
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    } else {
        result = PduR_DcmTransmit(response, 2u, now_ms);
    }
    return result;
}

static EcuStatus HandleReadData(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t response[ECU_MAX_PDU_PAYLOAD];
    size_t response_length = 1u;
    uint8_t nrc = 0u;
    EcuStatus result;
    response[0] = 0x62u;
    if ((length < 3u) || ((length & 1u) == 0u) ||
        ((Ecu_Policy.max_read_dids != 0u) && (((length - 1u) / 2u) > Ecu_Policy.max_read_dids))) {
        nrc = 0x13u;
    } else {
        size_t offset;
        for (offset = 1u; (offset < length) && (nrc == 0u); offset += 2u) {
            uint16_t did = (uint16_t)(((uint16_t)request[offset] << 8u) | request[offset + 1u]);
            size_t data_length = 0u;
            uint8_t supported = 0u;
            if (did == 0xf186u) {
                data_length = 1u;
                supported = 1u;
            } else if ((did == dcm_config->did) &&
                       ((Ecu_Policy.read_did_sessions & (UINT32_C(1) << active_session)) != 0u)) {
                data_length = 4u * dcm_config->did_signal_count;
                supported = 1u;
            } else {
                /* Unsupported DID values are skipped in this host profile. */
            }
            if (supported != 0u) {
                if (((response_length + 2u) + data_length) > sizeof(response)) {
                    nrc = 0x14u;
                } else {
                    response[response_length] = request[offset];
                    ++response_length;
                    response[response_length] = request[offset + 1u];
                    ++response_length;
                    if (did == 0xf186u) {
                        response[response_length] = active_session;
                        ++response_length;
                    } else {
                        size_t i;
                        for (i = 0u; (i < dcm_config->did_signal_count) && (nrc == 0u); ++i) {
                            if (dcm_config->did_readers[i](&response[response_length]) != E_OK) {
                                nrc = 0x22u;
                            } else {
                                response_length += 4u;
                            }
                        }
                    }
                }
            }
        }
    }
    if (nrc != 0u) {
        result = NegativeResponse(0x22u, nrc, now_ms);
    } else if (response_length == 1u) {
        result = NegativeResponse(0x22u, 0x31u, now_ms);
    } else {
        result = PduR_DcmTransmit(response, response_length, now_ms);
    }
    return result;
}

static EcuStatus HandleWriteData(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t nrc = 0u;
    uint8_t write_failed = 0u;
    EcuStatus result;
    if (dcm_config->did_writers == NULL) {
        nrc = 0x11u;
    } else if (length < 3u) {
        nrc = 0x13u;
    } else if (((((uint16_t)request[1] << 8u) | request[2]) != dcm_config->did) ||
               (active_session != 0x03u)) {
        nrc = 0x31u;
    } else if (length != (3u + (4u * dcm_config->did_signal_count))) {
        nrc = 0x13u;
    } else if ((dcm_config->security_enabled != 0u) && (Security_IsUnlocked() == 0)) {
        nrc = 0x33u;
    } else {
        size_t i;
        for (i = 0u; (i < dcm_config->did_signal_count) && (write_failed == 0u); ++i) {
            Dcm_NegativeResponseCodeType error_code = DCM_E_GENERALPROGRAMMINGFAILURE;
            if (dcm_config->did_writers[i](&request[3u + (4u * i)], &error_code) != E_OK) {
                nrc = error_code;
                write_failed = 1u;
            }
        }
    }
    if ((nrc != 0u) || (write_failed != 0u)) {
        result = NegativeResponse(0x2eu, nrc, now_ms);
    } else {
        uint8_t response[3] = {0x6eu, request[1], request[2]};
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    }
    return result;
}

static EcuStatus HandleRoutineControl(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t nrc = 0u;
    EcuStatus result;
    if (dcm_config->reset_routine == NULL) {
        nrc = 0x11u;
    } else if (length < 4u) {
        nrc = 0x13u;
    } else if (((((uint16_t)request[2] << 8u) | request[3]) != dcm_config->reset_routine->id) ||
               (active_session != 0x03u)) {
        nrc = 0x31u;
    } else if (request[1] != 0x01u) {
        nrc = 0x12u;
    } else if (length != 4u) {
        nrc = 0x13u;
    } else if ((dcm_config->security_enabled != 0u) && (Security_IsUnlocked() == 0)) {
        nrc = 0x33u;
    } else if (dcm_config->reset_routine->start() != ECU_OK) {
        nrc = 0x22u;
    } else {
        /* Request accepted. */
    }
    if (nrc != 0u) {
        result = NegativeResponse(0x31u, nrc, now_ms);
    } else {
        const uint8_t response[4] = {0x71u, 0x01u, request[2], request[3]};
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    }
    return result;
}

static EcuStatus HandleDtcSetting(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t nrc = 0u;
    EcuStatus result;
    if (dcm_config->dtc == NULL) {
        nrc = 0x11u;
    } else if (length != 2u) {
        nrc = 0x13u;
    } else if (active_session != 0x03u) {
        nrc = 0x7fu;
    } else if ((request[1] != 0x01u) && (request[1] != 0x02u)) {
        nrc = 0x12u;
    } else if ((dcm_config->security_enabled != 0u) && (Security_IsUnlocked() == 0)) {
        nrc = 0x33u;
    } else {
        result = (request[1] == 0x01u) ? Dem_EnableDTCSetting() : Dem_DisableDTCSetting();
        if (result != ECU_OK) {
            nrc = 0x22u;
        }
    }
    if (nrc != 0u) {
        result = NegativeResponse(0x85u, nrc, now_ms);
    } else {
        const uint8_t response[2] = {0xc5u, request[1]};
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    }
    return result;
}

static EcuStatus HandleReadDtc(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t response[7] = {0x59u, 0u, 0x7fu, 0u, 0u, 0u, 0u};
    uint8_t nrc = 0u;
    size_t response_length = 3u;
    EcuStatus result;
    if (dcm_config->dtc == NULL) {
        nrc = 0x11u;
    } else if (length < 2u) {
        nrc = 0x13u;
    } else if ((request[1] != 0x01u) && (request[1] != 0x02u) && (request[1] != 0x0au)) {
        nrc = 0x12u;
    } else if (length != ((request[1] == 0x0au) ? 2u : 3u)) {
        nrc = 0x13u;
    } else {
        uint32_t code;
        uint8_t status;
        uint8_t found = (request[1] == 0x0au) ? Dem_GetSupportedDtc(&code, &status)
                                              : Dem_FilterDtc(request[2], &code, &status);
        response[1] = request[1];
        if (request[1] == 0x01u) {
            response[3] = 0x01u; /* ISO 14229-1 DTC format. */
            response[5] = found;
            response_length = 6u;
        } else if (found != 0u) {
            response[3] = (uint8_t)(code >> 16u);
            response[4] = (uint8_t)(code >> 8u);
            response[5] = (uint8_t)code;
            response[6] = status;
            response_length = 7u;
        } else {
            /* No DTC matches the requested mask. */
        }
    }
    if (nrc != 0u) {
        result = NegativeResponse(0x19u, nrc, now_ms);
    } else {
        result = PduR_DcmTransmit(response, response_length, now_ms);
    }
    return result;
}

static EcuStatus HandleClearDtc(const uint8_t *request, size_t length, uint64_t now_ms) {
    uint8_t nrc = 0u;
    EcuStatus result;
    if (dcm_config->dtc == NULL) {
        nrc = 0x11u;
    } else if (length != 4u) {
        nrc = 0x13u;
    } else if (active_session != 0x03u) {
        nrc = 0x7fu;
    } else if ((request[1] != 0xffu) || (request[2] != 0xffu) || (request[3] != 0xffu)) {
        nrc = 0x31u;
    } else if ((dcm_config->security_enabled != 0u) && (Security_IsUnlocked() == 0)) {
        nrc = 0x33u;
    } else if (Dem_ClearAll() != ECU_OK) {
        nrc = 0x72u;
    } else {
        /* All configured DTCs were cleared. */
    }
    if (nrc != 0u) {
        result = NegativeResponse(0x14u, nrc, now_ms);
    } else {
        const uint8_t response[1] = {0x54u};
        result = PduR_DcmTransmit(response, sizeof(response), now_ms);
    }
    return result;
}

EcuStatus Dcm_DispatchRequest(const uint8_t *request, size_t length, uint64_t now_ms) {
    EcuStatus result = ECU_ERR_CONFIG;
    if ((dcm_config != NULL) && (request != NULL) && (length != 0u)) {
        last_request_ms = now_ms;
        uint8_t allowed = 0u;
        for (size_t index = 0u; index < Ecu_Policy.allowed_sid_count; ++index) {
            if (Ecu_Policy.allowed_sids[index] == request[0]) {
                allowed = 1u;
                break;
            }
        }
        if (allowed == 0u) {
            return NegativeResponse(request[0], 0x11u, now_ms);
        }
        switch (request[0]) {
        case 0x10u:
            result = HandleSessionControl(request, length, now_ms);
            break;
        case 0x3eu:
            result = HandleTesterPresent(request, length, now_ms);
            break;
        case 0x27u:
            result = HandleSecurityAccess(request, length, now_ms);
            break;
        case 0x22u:
            result = HandleReadData(request, length, now_ms);
            break;
        case 0x2eu:
            result = HandleWriteData(request, length, now_ms);
            break;
        case 0x31u:
            result = HandleRoutineControl(request, length, now_ms);
            break;
        case 0x85u:
            result = HandleDtcSetting(request, length, now_ms);
            break;
        case 0x19u:
            result = HandleReadDtc(request, length, now_ms);
            break;
        case 0x14u:
            result = HandleClearDtc(request, length, now_ms);
            break;
        default:
            result = NegativeResponse(request[0], 0x11u, now_ms);
            break;
        }
    }
    return result;
}

EcuStatus Dcm_RxIndication(const uint8_t *request, size_t length, uint64_t now_ms) {
    return Ecu_DiagnosticAdmit(dcm_config, request, length, now_ms);
}

void Dcm_RecordRequestTime(uint64_t now_ms) { last_request_ms = now_ms; }
