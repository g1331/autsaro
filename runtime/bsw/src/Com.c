/** @file Immediate UINT32 COM with real Rx group and PDU-level reception DM.
 * Com enters only its own COM_STATE area and calls PduR and its users outside it (ADR 0018):
 * a transmission is packed in the area and sent after leaving it; notifications are
 * recorded in the area and published in order by one context at a time.
 */
#include "Com.h"
#include "SchM_Com.h"
#include "Com_Internal.h"
#include <stddef.h>
#include <string.h>

#define COM_MAX_PDUS  32u
#define COM_PDU_BYTES 4u
#define COM_MAX_OWED  0xffffffffu
#define COM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"
static const Com_ConfigType *configuration COM_VAR_CLEARED;
static uint32 values[COM_MAX_PDUS] COM_VAR_CLEARED;
static uint32 remaining[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean reloaded_before_main[COM_MAX_PDUS] COM_VAR_CLEARED;
/* Notifications owed per Rx PDU: every reception is acknowledged (SWS_Com_00883), repeated
 * timeouts coalesce, and the event recorded last is also delivered last, so a stale timeout
 * never follows a fresh reception. */
static uint32 receptions_owed[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean timeout_owed[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean timeout_last[COM_MAX_PDUS] COM_VAR_CLEARED;
static boolean receive_started COM_VAR_CLEARED;
/* Reception DM of the Rx group; independent of start and stop (SWS_Com_00772). */
static boolean monitoring COM_VAR_CLEARED;
static boolean reception_before_main COM_VAR_CLEARED;
/* A context is delivering a notification; never reset, so it outlives a reinitialization. */
static boolean publishing COM_VAR_CLEARED;
#define COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"

#define COM_START_SEC_CODE
#include "Com_MemMap.h"
/* The helpers below run inside COM_STATE with an initialized configuration. */
static COM_CODE uint16 Com_FindPdu(PduIdType id)
{
    uint16 index = 0u;
    while ((index < configuration->pdu_count) && (configuration->pdus[index].pdu != id)) {
        ++index;
    }
    return index;
}
static COM_CODE uint16 Com_FindSignal(Com_SignalIdType id)
{
    uint16 index = 0u;
    while ((index < configuration->pdu_count) && (configuration->pdus[index].signal != id)) {
        ++index;
    }
    return index;
}
static COM_CODE void Com_Pack(uint16 index, uint8 *bytes)
{
    uint8 byte;
    for (byte = 0u; byte < COM_PDU_BYTES; ++byte) {
        bytes[byte] = (uint8)((values[index] >> ((uint32)byte * 8u)) & 0xffu);
    }
}
static COM_CODE void Com_RestartRx(boolean initialize)
{
    uint16 index;
    for (index = 0u; index < configuration->pdu_count; ++index) {
        if (configuration->pdus[index].receive == TRUE) {
            if (initialize == TRUE) {
                values[index] = configuration->pdus[index].initial_value;
            }
            /* First timeout zero: nothing is monitored until a reception (SWS_Com_00716). */
            remaining[index] = 0u;
            reloaded_before_main[index] = FALSE;
        }
    }
}
static COM_CODE void Com_Reset(void)
{
    uint16 index;
    configuration = NULL_PTR;
    receive_started = FALSE;
    monitoring = FALSE;
    reception_before_main = FALSE;
    for (index = 0u; index < COM_MAX_PDUS; ++index) {
        values[index] = 0u;
        remaining[index] = 0u;
        reloaded_before_main[index] = FALSE;
        receptions_owed[index] = 0u;
        timeout_owed[index] = FALSE;
        timeout_last[index] = FALSE;
    }
}
/* Takes the next owed notification; FALSE when nothing is owed. */
static COM_CODE boolean Com_TakeOwed(Com_NotificationType *notify, CbkHandleIdType *handle)
{
    boolean taken = FALSE;
    uint16 index = 0u;
    while ((taken == FALSE) && (index < configuration->pdu_count)) {
        if ((timeout_owed[index] == TRUE) &&
            ((timeout_last[index] == FALSE) || (receptions_owed[index] == 0u))) {
            timeout_owed[index] = FALSE;
            *notify = configuration->timeout_notification;
            taken = TRUE;
        } else if (receptions_owed[index] != 0u) {
            --receptions_owed[index];
            *notify = configuration->receive_notification;
            taken = TRUE;
        } else {
            ++index;
        }
    }
    if (taken == TRUE) {
        *handle = configuration->pdus[index].callback_handle;
    }
    return taken;
}
/* Single-flight ordered publication (ADR 0018): a context that finds another one delivering
 * only records; the delivering context keeps going until nothing is owed. */
static COM_CODE void Com_Publish(void)
{
    boolean more = TRUE;
    while (more == TRUE) {
        Com_NotificationType notify = NULL_PTR;
        CbkHandleIdType handle = 0u;
        more = FALSE;
        SchM_Enter_Com_COM_STATE();
        if ((publishing == FALSE) && (configuration != NULL_PTR)) {
            more = Com_TakeOwed(&notify, &handle);
            publishing = more;
        }
        SchM_Exit_Com_COM_STATE();
        if (more == TRUE) {
            notify(handle);
            SchM_Enter_Com_COM_STATE();
            publishing = FALSE;
            SchM_Exit_Com_COM_STATE();
        }
    }
}
static COM_CODE boolean Com_ValidConfig(const Com_ConfigType *config)
{
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
COM_CODE void Com_Init(const Com_ConfigType *config)
{
    const boolean valid = Com_ValidConfig(config);
    uint16 index;
    SchM_Enter_Com_COM_STATE();
    Com_Reset();
    if (valid == TRUE) {
        configuration = config;
        for (index = 0u; index < config->pdu_count; ++index) {
            values[index] = config->pdus[index].initial_value;
        }
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_DeInit(void)
{
    SchM_Enter_Com_COM_STATE();
    Com_Reset();
    SchM_Exit_Com_COM_STATE();
}
COM_CODE Com_StatusType Com_GetStatus(void)
{
    Com_StatusType status;
    SchM_Enter_Com_COM_STATE();
    status = (configuration == NULL_PTR) ? COM_UNINIT : COM_INIT;
    SchM_Exit_Com_COM_STATE();
    return status;
}
COM_CODE uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr)
{
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    SchM_Enter_Com_COM_STATE();
    if (configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < configuration->pdu_count) && (SignalDataPtr != NULL_PTR) &&
            (configuration->pdus[index].receive == FALSE)) {
            (void)memcpy(&values[index], SignalDataPtr, sizeof(values[index]));
            result = E_OK;
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
COM_CODE uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr)
{
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    SchM_Enter_Com_COM_STATE();
    if (configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < configuration->pdu_count) && (SignalDataPtr != NULL_PTR) &&
            (configuration->pdus[index].receive == TRUE)) {
            (void)memcpy(SignalDataPtr, &values[index], sizeof(values[index]));
            result = (receive_started == TRUE) ? E_OK : COM_SERVICE_NOT_AVAILABLE;
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
COM_CODE void Com_IpduGroupStart(Com_IpduGroupIdType IpduGroupId, boolean Initialize)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group) &&
        (receive_started == FALSE)) {
        receive_started = TRUE;
        Com_RestartRx(Initialize);
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_IpduGroupStop(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group)) {
        receive_started = FALSE;
        Com_RestartRx(FALSE);
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_EnableReceptionDM(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group) &&
        (monitoring == FALSE)) {
        monitoring = TRUE;
        Com_RestartRx(FALSE);
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_DisableReceptionDM(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (IpduGroupId == configuration->receive_group)) {
        monitoring = FALSE;
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Ecu_ComReceptionBeforeMain(boolean pending)
{
    SchM_Enter_Com_COM_STATE();
    reception_before_main = pending;
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (receive_started == TRUE) && (PduInfoPtr != NULL_PTR) &&
        ((PduInfoPtr->SduDataPtr != NULL_PTR) || (PduInfoPtr->SduLength == 0u))) {
        const uint16 index = Com_FindPdu(RxPduId);
        if ((index < configuration->pdu_count) && (configuration->pdus[index].receive == TRUE)) {
            /* I-PDU based DM is reset first, also for a short PDU (SWS_Com_00396). */
            if (monitoring == TRUE) {
                remaining[index] = configuration->pdus[index].timeout_ticks;
                reloaded_before_main[index] = reception_before_main;
            }
            /* Only a completely received signal is unpacked and notified (SWS_Com_00574). */
            if (PduInfoPtr->SduLength >= COM_PDU_BYTES) {
                const uint8 *data = PduInfoPtr->SduDataPtr;
                values[index] = (uint32)data[0] | ((uint32)data[1] << 8u) |
                                ((uint32)data[2] << 16u) | ((uint32)data[3] << 24u);
                if (receptions_owed[index] != COM_MAX_OWED) {
                    ++receptions_owed[index];
                }
                timeout_last[index] = FALSE;
            }
        }
    }
    SchM_Exit_Com_COM_STATE();
    Com_Publish();
}
COM_CODE Std_ReturnType Com_TriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) && (PduInfoPtr->SduLength >= COM_PDU_BYTES)) {
        const uint16 index = Com_FindPdu(TxPduId);
        if ((index < configuration->pdu_count) && (configuration->pdus[index].receive == FALSE)) {
            Com_Pack(index, PduInfoPtr->SduDataPtr);
            PduInfoPtr->SduLength = COM_PDU_BYTES;
            result = E_OK;
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
COM_CODE Std_ReturnType Com_TriggerIPDUSend(PduIdType PduId)
{
    uint8 bytes[COM_PDU_BYTES];
    const PduInfoType info = {bytes, NULL_PTR, COM_PDU_BYTES};
    Com_TransmitType transmit = NULL_PTR;
    SchM_Enter_Com_COM_STATE();
    if (configuration != NULL_PTR) {
        const uint16 index = Com_FindPdu(PduId);
        if ((index < configuration->pdu_count) && (configuration->pdus[index].receive == FALSE)) {
            transmit = configuration->transmit;
            Com_Pack(index, bytes);
        }
    }
    SchM_Exit_Com_COM_STATE();
    /* Started I-PDUs are triggered; without retry a lower rejection is not a stop (00348). */
    if (transmit != NULL_PTR) {
        (void)transmit(PduId, &info);
    }
    return (transmit != NULL_PTR) ? E_OK : E_NOT_OK;
}
COM_CODE void Com_TxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    /* The selected periodic PDUs have no Tx ACK/NACK callback or Tx DM.
     * Lower acceptance/confirmation does not suppress the next periodic request.
     */
    (void)TxPduId;
    (void)result;
}
COM_CODE void Ecu_ComMainFunctionRx(void)
{
    SchM_Enter_Com_COM_STATE();
    if ((configuration != NULL_PTR) && (receive_started == TRUE) && (monitoring == TRUE)) {
        uint16 index;
        for (index = 0u; index < configuration->pdu_count; ++index) {
            if ((configuration->pdus[index].receive == TRUE) && (remaining[index] != 0u)) {
                if (reloaded_before_main[index] == FALSE) {
                    --remaining[index];
                    if (remaining[index] == 0u) {
                        /* Restarted on expiry (SWS_Com_00889). */
                        remaining[index] = configuration->pdus[index].timeout_ticks;
                        timeout_owed[index] = TRUE;
                        timeout_last[index] = TRUE;
                    }
                }
                reloaded_before_main[index] = FALSE;
            }
        }
    }
    reception_before_main = FALSE;
    SchM_Exit_Com_COM_STATE();
    Com_Publish();
}
COM_CODE void Ecu_ComMainFunctionTx(void)
{
    uint16 index = 0u;
    boolean more = TRUE;
    while (more == TRUE) {
        uint8 bytes[COM_PDU_BYTES];
        const PduInfoType info = {bytes, NULL_PTR, COM_PDU_BYTES};
        PduIdType id = 0u;
        Com_TransmitType transmit = NULL_PTR;
        SchM_Enter_Com_COM_STATE();
        more = ((configuration != NULL_PTR) && (index < configuration->pdu_count)) ? TRUE : FALSE;
        if ((more == TRUE) && (configuration->pdus[index].receive == FALSE)) {
            id = configuration->pdus[index].pdu;
            transmit = configuration->transmit;
            Com_Pack(index, bytes);
        }
        SchM_Exit_Com_COM_STATE();
        /* Without ComRetryFailedTransmitRequests a rejected PERIODIC request is not retried. */
        if (transmit != NULL_PTR) {
            (void)transmit(id, &info);
        }
        ++index;
    }
}
#define COM_STOP_SEC_CODE
#include "Com_MemMap.h"
