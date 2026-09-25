#include "Os.h"
#include "Can.h"
#include "Com.h"

static const EcuConfig *active_config;
static uint64_t current_ms;

void Os_Init(const EcuConfig *config)
{
    active_config = config;
    current_ms = 0;
}

uint64_t Os_Now(void)
{
    return current_ms;
}

EcuStatus Os_Advance(uint64_t now_ms)
{
    size_t i;
    if (now_ms < current_ms) {
        return ECU_ERR_TIME;
    }
    Com_AdvanceTime(now_ms);
    for (i = 0; i < active_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &active_config->frames[i];
        if (frame->direction == 1u &&
            now_ms / frame->period_ms > current_ms / frame->period_ms &&
            Can_GetMode() == CAN_STARTED) {
            EcuStatus result = Com_TriggerTransmit(i);
            if (result != ECU_OK) {
                return result;
            }
        }
    }
    current_ms = now_ms;
    return ECU_OK;
}
