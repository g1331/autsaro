#include "CanIf.h"
#include "Can.h"
#include "Can_HostLock.h"
#include "LSduR.h"
#include "Os.h"

static const EcuConfig *canif_config;
static uint64_t host_rx_time_ms;
static uint8_t host_rx_time_active;
static EcuStatus host_rx_result;

void CanIf_Init(const EcuConfig *config) {
    Can_Lock();
    canif_config = config;
    host_rx_time_active = 0u;
    host_rx_result = ECU_ERR_CONFIG;
    Can_Unlock();
}

EcuStatus CanIf_Transmit(size_t frame_index, const uint8_t data[8]) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (frame_index < canif_config->frame_count) {
        const EcuFrameConfig *frame = &canif_config->frames[frame_index];
        if (frame->direction != 1u) {
            result = ECU_ERR_DIRECTION;
        } else {
            result = Can_Transmit(frame->id, frame->dlc, data);
        }
    }
    return result;
}

static EcuStatus RouteRx(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    size_t i;
    EcuStatus result = ECU_OK;
    if ((canif_config->diagnostic != NULL) && (id == canif_config->diagnostic->request_can_id)) {
        result = LSduR_CanTpRxIndication(dlc, data, now_ms);
    } else {
        for (i = 0u; i < canif_config->frame_count; ++i) {
            const EcuFrameConfig *frame = &canif_config->frames[i];
            if ((frame->id == id) && (frame->direction == 0u)) {
                if (frame->dlc != dlc) {
                    result = ECU_ERR_FRAME_DLC;
                } else {
                    result = LSduR_CanIfRxIndication(i, data, now_ms);
                }
                break;
            }
        }
    }
    /* 其他标准 CAN ID 在控制器过滤器处被丢弃。 */
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
            uint64_t now_ms = (host_rx_time_active != 0u) ? host_rx_time_ms : Os_Now();
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
    size_t i;
    if (data != NULL) {
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
