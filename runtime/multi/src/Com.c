/** @file Immediate UINT32 COM with real Rx group and PDU-level reception DM. */
#include "Com.h"
#include "Com_Internal.h"
#include <stddef.h>
#include <string.h>

#define COM_MAX_PDUS 32u
#define COM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"
static const Com_ConfigType *configuration COM_VAR_CLEARED;
static uint32 values[COM_MAX_PDUS] COM_VAR_CLEARED;
static uint32 remaining[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean reloaded_before_main[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean receive_started COM_VAR_CLEARED;
static boolean monitoring COM_VAR_CLEARED;
static boolean reception_before_main COM_VAR_CLEARED;
#define COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"

#define COM_START_SEC_CODE
#include "Com_MemMap.h"
static COM_CODE uint16 Com_FindPdu(PduIdType id) {
    uint16 index = 0u;
    while ((index < configuration->pdu_count) && (configuration->pdus[index].pdu != id)) {
        ++index;
    }
    return index;
}
static COM_CODE uint16 Com_FindSignal(Com_SignalIdType id) {
    uint16 index = 0u;
    while ((index < configuration->pdu_count) && (configuration->pdus[index].signal != id)) {
        ++index;
    }
    return index;
}
static COM_CODE boolean Com_ValidConfig(const Com_ConfigType *config) {
    boolean valid = FALSE;
    uint16 index;
    if ((config != NULL_PTR) && (config->pdus != NULL_PTR) && (config->pdu_count > 0u) &&
        (config->pdu_count <= COM_MAX_PDUS)) {
        valid = TRUE;
        for (index = 0u; index < config->pdu_count; ++index) {
            uint16 previous;
            const Com_PduConfigType *pdu = &config->pdus[index];
            if (((pdu->receive == TRUE) && ((config->receive_notification == NULL_PTR) ||
                                            (config->timeout_notification == NULL_PTR))) ||
                ((pdu->receive == FALSE) && (config->transmit == NULL_PTR))) {
                valid = FALSE;
            }
            for (previous = 0u; previous < index; ++previous) {
                if ((pdu->pdu == config->pdus[previous].pdu) ||
                    (pdu->signal == config->pdus[previous].signal) ||
                    ((pdu->receive == TRUE) && (config->pdus[previous].receive == TRUE) &&
                     (pdu->callback_handle == config->pdus[previous].callback_handle))) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
COM_CODE void Com_Init(const Com_ConfigType *config) {
    uint16 index;
    Com_DeInit();
    if (Com_ValidConfig(config) == TRUE) {
        configuration = config;
        for (index = 0u; index < config->pdu_count; ++index) {
            values[index] = config->pdus[index].initial_value;
        }
    }
}
COM_CODE void Com_DeInit(void) {
    uint16 index;
    configuration = NULL_PTR;
    receive_started = FALSE;
    monitoring = FALSE;
    reception_before_main = FALSE;
    for (index = 0u; index < COM_MAX_PDUS; ++index) {
        values[index] = 0u;
        remaining[index] = 0u;
        reloaded_before_main[index] = FALSE;
    }
}
COM_CODE Com_StatusType Com_GetStatus(void) {
    return (configuration == NULL_PTR) ? COM_UNINIT : COM_INIT;
}
COM_CODE uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr) {
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    if (configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < configuration->pdu_count) && (SignalDataPtr != NULL_PTR) &&
            (configuration->pdus[index].receive == FALSE)) {
            (void)memcpy(&values[index], SignalDataPtr, sizeof(values[index]));
            result = E_OK;
        }
    }
    return result;
}
COM_CODE uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr) {
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    if (configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < configuration->pdu_count) && (SignalDataPtr != NULL_PTR) &&
            (configuration->pdus[index].receive == TRUE)) {
            (void)memcpy(SignalDataPtr, &values[index], sizeof(values[index]));
            result = (receive_started == TRUE) ? E_OK : COM_SERVICE_NOT_AVAILABLE;
        }
    }
    return result;
}
COM_CODE void Com_IpduGroupStart(Com_IpduGroupIdType IpduGroupId, boolean Initialize) {
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group) &&
        (receive_started == FALSE)) {
        uint16 index;
        receive_started = TRUE;
        monitoring = TRUE;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if (configuration->pdus[index].receive == TRUE) {
                if (Initialize == TRUE) {
                    values[index] = configuration->pdus[index].initial_value;
                }
                remaining[index] = 0u;
                reloaded_before_main[index] = FALSE;
            }
        }
    }
}
COM_CODE void Com_IpduGroupStop(Com_IpduGroupIdType IpduGroupId) {
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group)) {
        uint16 index;
        receive_started = FALSE;
        monitoring = FALSE;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if (configuration->pdus[index].receive == TRUE) {
                remaining[index] = 0u;
                reloaded_before_main[index] = FALSE;
            }
        }
    }
}
COM_CODE void Com_EnableReceptionDM(Com_IpduGroupIdType IpduGroupId) {
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group) &&
        (monitoring == FALSE)) {
        uint16 index;
        monitoring = TRUE;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if (configuration->pdus[index].receive == TRUE) {
                remaining[index] = 0u;
                reloaded_before_main[index] = FALSE;
            }
        }
    }
}
COM_CODE void Com_DisableReceptionDM(Com_IpduGroupIdType IpduGroupId) {
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group)) {
        monitoring = FALSE;
    }
}
COM_CODE void Ecu_ComReceptionBeforeMain(boolean pending) { reception_before_main = pending; }
COM_CODE void Com_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr) {
    if ((configuration != NULL_PTR) && (receive_started == TRUE) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) && (PduInfoPtr->SduLength >= 4u)) {
        const uint16 index = Com_FindPdu(RxPduId);
        if ((index < configuration->pdu_count) && (configuration->pdus[index].receive == TRUE)) {
            const uint8 *data = PduInfoPtr->SduDataPtr;
            values[index] = (uint32)data[0] | ((uint32)data[1] << 8u) | ((uint32)data[2] << 16u) |
                            ((uint32)data[3] << 24u);
            if (monitoring == TRUE) {
                remaining[index] = configuration->pdus[index].timeout_ticks;
                reloaded_before_main[index] = reception_before_main;
            }
            configuration->receive_notification(configuration->pdus[index].callback_handle);
        }
    }
}
COM_CODE Std_ReturnType Com_TriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr) {
    Std_ReturnType result = E_NOT_OK;
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) && (PduInfoPtr->SduLength >= 4u)) {
        const uint16 index = Com_FindPdu(TxPduId);
        if ((index < configuration->pdu_count) && (configuration->pdus[index].receive == FALSE)) {
            uint8 byte;
            for (byte = 0u; byte < 4u; ++byte) {
                PduInfoPtr->SduDataPtr[byte] =
                    (uint8)((values[index] >> ((uint32)byte * 8u)) & 0xffu);
            }
            PduInfoPtr->SduLength = 4u;
            result = E_OK;
        }
    }
    return result;
}
COM_CODE void Com_TxConfirmation(PduIdType TxPduId, Std_ReturnType result) {
    /* The selected periodic PDUs have no Tx ACK/NACK callback or Tx DM.
     * Lower acceptance/confirmation does not suppress the next periodic request.
     */
    (void)TxPduId;
    (void)result;
}
COM_CODE void Ecu_ComMainFunctionRx(void) {
    if ((configuration != NULL_PTR) && (receive_started == TRUE) && (monitoring == TRUE)) {
        uint16 index;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if ((configuration->pdus[index].receive == TRUE) && (remaining[index] != 0u)) {
                if (reloaded_before_main[index] == FALSE) {
                    --remaining[index];
                    if (remaining[index] == 0u) {
                        remaining[index] = configuration->pdus[index].timeout_ticks;
                        configuration->timeout_notification(
                            configuration->pdus[index].callback_handle);
                    }
                }
                reloaded_before_main[index] = FALSE;
            }
        }
    }
    reception_before_main = FALSE;
}
COM_CODE void Ecu_ComMainFunctionTx(void) {
    if (configuration != NULL_PTR) {
        uint16 index;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if (configuration->pdus[index].receive == FALSE) {
                uint8 bytes[4];
                PduInfoType info = {bytes, NULL_PTR, 4u};
                if (Com_TriggerTransmit(configuration->pdus[index].pdu, &info) == E_OK) {
                    (void)configuration->transmit(configuration->pdus[index].pdu, &info);
                }
            }
        }
    }
}
#define COM_STOP_SEC_CODE
#include "Com_MemMap.h"
