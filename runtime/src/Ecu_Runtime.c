#include "Ecu_Runtime.h"
#include "CanIf.h"
#include "Com.h"
#include "Os.h"

static EcuStatus ValidateConfig(const EcuConfig *config)
{
    uint8_t assigned[ECU_MAX_SIGNALS] = {0};
    size_t i;
    size_t j;

    if (config == NULL || config->name == NULL || config->name[0] == '\0' ||
        config->frames == NULL || config->signals == NULL ||
        config->frame_count == 0 || config->frame_count > ECU_MAX_FRAMES ||
        config->signal_count == 0 || config->signal_count > ECU_MAX_SIGNALS) {
        return ECU_ERR_CONFIG;
    }
    for (i = 0; i < config->signal_count; ++i) {
        for (j = 0; j < i; ++j) {
            if (config->signals[i].id == config->signals[j].id) {
                return ECU_ERR_CONFIG;
            }
        }
    }
    for (i = 0; i < config->frame_count; ++i) {
        const EcuFrameConfig *frame = &config->frames[i];
        uint64_t occupied = 0;
        if (frame->id > 0x7ffu || frame->dlc < 1u || frame->dlc > 8u ||
            frame->direction > 1u || frame->signal_count == 0u ||
            frame->first_signal > config->signal_count ||
            frame->signal_count > config->signal_count - frame->first_signal ||
            (frame->direction == 1u && (frame->period_ms == 0u || frame->timeout_ms != 0u)) ||
            (frame->direction == 0u && (frame->timeout_ms == 0u || frame->period_ms != 0u))) {
            return ECU_ERR_CONFIG;
        }
        for (j = 0; j < i; ++j) {
            if (frame->id == config->frames[j].id) {
                return ECU_ERR_CONFIG;
            }
        }
        for (j = frame->first_signal; j < (size_t)frame->first_signal + frame->signal_count; ++j) {
            const EcuSignalConfig *signal = &config->signals[j];
            unsigned bit;
            if (assigned[j] || signal->bit_length == 0u || signal->bit_length > 32u ||
                (unsigned)signal->start_bit + signal->bit_length > (unsigned)frame->dlc * 8u ||
                (signal->bit_length < 32u && signal->initial_value >= (UINT32_C(1) << signal->bit_length))) {
                return ECU_ERR_CONFIG;
            }
            assigned[j] = 1u;
            for (bit = signal->start_bit; bit < (unsigned)signal->start_bit + signal->bit_length; ++bit) {
                uint64_t mask = UINT64_C(1) << bit;
                if ((occupied & mask) != 0u) {
                    return ECU_ERR_CONFIG;
                }
                occupied |= mask;
            }
        }
    }
    for (i = 0; i < config->signal_count; ++i) {
        if (!assigned[i]) {
            return ECU_ERR_CONFIG;
        }
    }
    return ECU_OK;
}

EcuStatus Ecu_Init(const EcuConfig *config, CanTxSink sink)
{
    EcuStatus result = ValidateConfig(config);
    if (result != ECU_OK || sink == NULL) {
        return ECU_ERR_CONFIG;
    }
    Com_Init(config);
    CanIf_Init(config);
    Can_Init(sink);
    Os_Init(config);
    return ECU_OK;
}
