#include "Os.h"
#include "Can.h"
#include "Com.h"
#include "CanTp.h"
#include "Dcm.h"

static const EcuConfig *active_config;
static uint64_t current_ms;

void Os_Init(const EcuConfig *config) {
    active_config = config;
    current_ms = 0;
}

uint64_t Os_Now(void) { return current_ms; }

EcuStatus Os_Advance(uint64_t now_ms) {
    size_t i;
    uint64_t previous_ms = current_ms;
    EcuStatus transport_status;
    if (now_ms < previous_ms) {
        return ECU_ERR_TIME;
    }
    current_ms = now_ms;
    {
        EcuStatus result = Com_AdvanceTime(now_ms);
        if (result != ECU_OK) {
            return result;
        }
    }
    Dcm_AdvanceTime(now_ms);
    transport_status = CanTp_AdvanceTime(now_ms);
    if (transport_status != ECU_OK) {
        return transport_status;
    }
    for (i = 0; i < active_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &active_config->frames[i];
        if (frame->direction == 1u && now_ms / frame->period_ms > previous_ms / frame->period_ms &&
            Can_GetMode() == CAN_STARTED) {
            EcuStatus result = Com_TriggerTransmit(i);
            if (result != ECU_OK) {
                return result;
            }
        }
    }
    return ECU_OK;
}
