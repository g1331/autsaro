#include "CanIf.h"
#include "Can.h"
#include "Can_HostLock.h"
#include "LSduR.h"
#include "Ecu_Execution.h"

#define CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"

static const EcuConfig *canif_config;
static uint64_t host_rx_time_ms;
static uint8_t host_rx_time_active;
static EcuStatus host_rx_result;
static Can_ControllerStateType indicated_controller_mode;

#define CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"

#define CANIF_START_SEC_CODE
#include "CanIf_MemMap.h"

void CanIf_Init(const EcuConfig *config) {
    Can_Lock();
    canif_config = config;
    host_rx_time_active = 0u;
    host_rx_result = ECU_ERR_CONFIG;
    indicated_controller_mode = CAN_CS_STOPPED;
    Can_Unlock();
}

EcuStatus CanIf_Transmit(size_t frame_index, const uint8_t data[8]) {
    EcuStatus result = ECU_ERR_CONFIG;
    Can_Lock();
    if ((canif_config != NULL) && (frame_index < canif_config->frame_count)) {
        const EcuFrameConfig *frame = &canif_config->frames[frame_index];
        if (frame->direction != 1u) {
            result = ECU_ERR_DIRECTION;
        } else if (indicated_controller_mode != CAN_CS_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else {
            for (size_t index = 0u; index < Ecu_TransmitRouteCount; ++index) {
                const EcuTransmitRoute *route = &Ecu_TransmitRoutes[index];
                if ((route->consumer == ECU_ROUTE_COM) &&
                    ((size_t)route->upper_pdu == frame_index)) {
                    result = Can_TransmitPdu(route->canif_pdu, frame->id, frame->dlc, data);
                    break;
                }
            }
        }
    }
    Can_Unlock();
    return result;
}

EcuStatus CanIf_TransmitDiagnostic(uint8_t dlc, const uint8_t data[8]) {
    EcuStatus result = ECU_ERR_CONFIG;
    Can_Lock();
    if ((canif_config != NULL) && (canif_config->diagnostic != NULL)) {
        if (indicated_controller_mode != CAN_CS_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else {
            for (size_t index = 0u; index < Ecu_TransmitRouteCount; ++index) {
                const EcuTransmitRoute *route = &Ecu_TransmitRoutes[index];
                if ((route->consumer == ECU_ROUTE_CANTP) &&
                    (route->upper_pdu == canif_config->diagnostic->tx_pdu_id)) {
                    result = Can_TransmitPdu(route->canif_pdu,
                                             canif_config->diagnostic->response_can_id, dlc, data);
                    break;
                }
            }
        }
    }
    Can_Unlock();
    return result;
}

void CanIf_TxConfirmation(PduIdType can_tx_pdu_id) {
    Can_Lock();
    if (canif_config != NULL) {
        for (size_t index = 0u; index < Ecu_TransmitRouteCount; ++index) {
            const EcuTransmitRoute *route = &Ecu_TransmitRoutes[index];
            if (route->canif_pdu == can_tx_pdu_id) {
                if (((route->consumer == ECU_ROUTE_COM) &&
                     ((size_t)route->upper_pdu < canif_config->frame_count) &&
                     (canif_config->frames[route->upper_pdu].direction == 1u)) ||
                    ((route->consumer == ECU_ROUTE_CANTP) && (canif_config->diagnostic != NULL) &&
                     (route->upper_pdu == canif_config->diagnostic->tx_pdu_id))) {
                    LSduR_CanIfTxConfirmation(can_tx_pdu_id, E_OK);
                }
                break;
            }
        }
    }
    Can_Unlock();
}

void CanIf_ControllerModeIndication(uint8_t controller_id,
                                    Can_ControllerStateType controller_mode) {
    Can_Lock();
    if ((canif_config != NULL) && (controller_id == 0u) &&
        ((controller_mode == CAN_CS_STARTED) || (controller_mode == CAN_CS_STOPPED) ||
         (controller_mode == CAN_CS_SLEEP))) {
        indicated_controller_mode = controller_mode;
    }
    Can_Unlock();
}

void CanIf_ControllerBusOff(uint8_t controller_id) {
    Can_Lock();
    if ((canif_config != NULL) && (controller_id == 0u)) {
        indicated_controller_mode = CAN_CS_STOPPED;
    }
    Can_Unlock();
}

static EcuStatus RouteRx(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    for (size_t index = 0u; index < Ecu_ReceiveRouteCount; ++index) {
        const EcuReceiveRoute *route = &Ecu_ReceiveRoutes[index];
        if (route->can_id == id) {
            if (route->consumer == ECU_ROUTE_CANTP) {
                if ((canif_config->diagnostic != NULL) &&
                    (canif_config->diagnostic->request_can_id == id)) {
                    result = LSduR_CanTpRxIndication(dlc, data, now_ms);
                }
            } else if (((size_t)route->upper_pdu < canif_config->frame_count) &&
                       (canif_config->frames[route->upper_pdu].direction == 0u) &&
                       (canif_config->frames[route->upper_pdu].id == id)) {
                if (route->dlc != dlc) {
                    result = ECU_ERR_FRAME_DLC;
                } else {
                    result = LSduR_CanIfRxIndication(route->canif_pdu, data, now_ms);
                }
            }
            break;
        }
    }
    /* Other standard CAN IDs are discarded by the controller filter. */
    return result;
}

void CanIf_RxIndication(const Can_HwType *mailbox, const PduInfoType *pdu_info) {
    Can_Lock();
    host_rx_result = ECU_ERR_CONFIG;
    if ((canif_config != NULL) && (mailbox != NULL) && (pdu_info != NULL) &&
        (pdu_info->SduDataPtr != NULL)) {
        if ((mailbox->Hoh != 0u) || (mailbox->ControllerId != 0u)) {
            host_rx_result = ECU_ERR_CONFIG;
        } else if (mailbox->CanId > 0x7ffu) {
            host_rx_result = ECU_ERR_FRAME_ID;
        } else if ((pdu_info->SduLength < 1u) || (pdu_info->SduLength > 8u)) {
            host_rx_result = ECU_ERR_FRAME_DLC;
        } else {
            uint64_t now_ms = (host_rx_time_active != 0u) ? host_rx_time_ms : Ecu_ExecutionNow();
            host_rx_time_active = 0u;
            host_rx_result =
                RouteRx(mailbox->CanId, (uint8_t)pdu_info->SduLength, pdu_info->SduDataPtr, now_ms);
        }
    }
    Can_Unlock();
}

EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    Can_HwType mailbox = {id, 0u, 0u};
    uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    PduInfoType pdu_info = {payload, NULL, dlc};
    EcuStatus result;
    uint64_t previous_time_ms;
    uint8_t previous_time_active;
    if (data != NULL) {
        size_t i;
        for (i = 0u; (i < dlc) && (i < sizeof(payload)); ++i) {
            payload[i] = data[i];
        }
    } else {
        pdu_info.SduDataPtr = NULL;
    }
    Can_Lock();
    previous_time_ms = host_rx_time_ms;
    previous_time_active = host_rx_time_active;
    host_rx_time_ms = now_ms;
    host_rx_time_active = 1u;
    CanIf_RxIndication(&mailbox, &pdu_info);
    result = host_rx_result;
    host_rx_time_ms = previous_time_ms;
    host_rx_time_active = previous_time_active;
    Can_Unlock();
    return result;
}

#define CANIF_STOP_SEC_CODE
#include "CanIf_MemMap.h"
