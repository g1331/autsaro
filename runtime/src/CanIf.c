#include "CanIf.h"
#include "Can.h"
#include "LSduR.h"

static const EcuConfig *canif_config;

void CanIf_Init(const EcuConfig *config) { canif_config = config; }

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

EcuStatus CanIf_RxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
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
