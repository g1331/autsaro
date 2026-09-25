#include "CanIf.h"
#include "Can.h"
#include "PduR.h"

static const EcuConfig *active_config;

void CanIf_Init(const EcuConfig *config)
{
    active_config = config;
}

EcuStatus CanIf_Transmit(size_t frame_index, const uint8_t data[8])
{
    const EcuFrameConfig *frame;
    if (frame_index >= active_config->frame_count) {
        return ECU_ERR_CONFIG;
    }
    frame = &active_config->frames[frame_index];
    if (frame->direction != 1u) {
        return ECU_ERR_DIRECTION;
    }
    return Can_Transmit(frame->id, frame->dlc, data);
}

EcuStatus CanIf_RxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    size_t i;
    for (i = 0; i < active_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &active_config->frames[i];
        if (frame->id == id && frame->direction == 0u) {
            if (frame->dlc != dlc) {
                return ECU_ERR_FRAME_DLC;
            }
            return PduR_RxIndication(i, data, now_ms);
        }
    }
    /* 其他标准 CAN ID 在控制器过滤器处被丢弃。 */
    return ECU_OK;
}
