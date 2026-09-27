#include "Os.h"
#include "Can.h"
#include "Com.h"
#include "CanTp.h"
#include "Dcm.h"

static const EcuConfig *os_config;
static uint64_t current_ms;

void Os_Init(const EcuConfig *config) {
    os_config = config;
    current_ms = 0;
}

uint64_t Os_Now(void) { return current_ms; }

EcuStatus Os_Advance(uint64_t now_ms) {
    uint64_t previous_ms = current_ms;
    EcuStatus result = ECU_OK;
    if (now_ms < previous_ms) {
        result = ECU_ERR_TIME;
    } else {
        current_ms = now_ms;
        result = Com_AdvanceTime(now_ms);
        if (result == ECU_OK) {
            Dcm_AdvanceTime(now_ms);
            result = CanTp_AdvanceTime(now_ms);
        }
        if (result == ECU_OK) {
            size_t i;
            for (i = 0u; i < os_config->frame_count; ++i) {
                const EcuFrameConfig *frame = &os_config->frames[i];
                if ((frame->direction == 1u) &&
                    ((now_ms / frame->period_ms) > (previous_ms / frame->period_ms)) &&
                    (Can_GetMode() == CAN_STARTED)) {
                    result = Com_TriggerTransmit(i);
                    if (result != ECU_OK) {
                        break;
                    }
                }
            }
        }
    }
    return result;
}
