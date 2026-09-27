#include "Com.h"
#include "Dem.h"
#include "PduR.h"

static const EcuConfig *com_config;
static uint32_t signal_values[ECU_MAX_SIGNALS];
static uint8_t signal_valid[ECU_MAX_SIGNALS];
static uint64_t rx_at_ms[ECU_MAX_FRAMES];
static uint8_t rx_seen[ECU_MAX_FRAMES];

static size_t FindSignal(uint16_t id) {
    size_t i;
    size_t result = com_config->signal_count;
    for (i = 0u; i < com_config->signal_count; ++i) {
        if (com_config->signals[i].id == id) {
            result = i;
            break;
        }
    }
    return result;
}

static size_t FrameForSignal(size_t signal_index) {
    size_t i;
    size_t result = com_config->frame_count;
    for (i = 0u; i < com_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &com_config->frames[i];
        if ((signal_index >= frame->first_signal) &&
            ((signal_index - frame->first_signal) < frame->signal_count)) {
            result = i;
            break;
        }
    }
    return result;
}

void Com_Init(const EcuConfig *config) {
    size_t i;
    com_config = config;
    for (i = 0; i < config->signal_count; ++i) {
        signal_values[i] = config->signals[i].initial_value;
        signal_valid[i] = (uint8_t)((config->frames[FrameForSignal(i)].direction == 1u) ? 1u : 0u);
    }
    for (i = 0; i < config->frame_count; ++i) {
        rx_at_ms[i] = 0;
        rx_seen[i] = 0;
    }
}

EcuStatus Com_SetSignal(uint16_t id, uint32_t value) {
    size_t i = FindSignal(id);
    EcuStatus result = ECU_ERR_SIGNAL_ID;
    if (i < com_config->signal_count) {
        const EcuSignalConfig *signal = &com_config->signals[i];
        if (com_config->frames[FrameForSignal(i)].direction != 1u) {
            result = ECU_ERR_DIRECTION;
        } else if ((signal->bit_length < 32u) && (value >= (UINT32_C(1) << signal->bit_length))) {
            result = ECU_ERR_SIGNAL_VALUE;
        } else {
            signal_values[i] = value;
            result = ECU_OK;
        }
    }
    return result;
}

EcuStatus Com_GetSignal(uint16_t id, uint32_t *value, uint8_t *valid) {
    size_t i = FindSignal(id);
    EcuStatus result = ECU_ERR_SIGNAL_ID;
    if (i < com_config->signal_count) {
        *value = signal_values[i];
        *valid = signal_valid[i];
        result = ECU_OK;
    }
    return result;
}

EcuStatus Com_TriggerTransmit(size_t frame_index) {
    const EcuFrameConfig *frame = &com_config->frames[frame_index];
    uint8_t data[8] = {0};
    size_t i;
    for (i = frame->first_signal; i < (size_t)frame->first_signal + frame->signal_count; ++i) {
        const EcuSignalConfig *signal = &com_config->signals[i];
        unsigned bit;
        for (bit = 0; bit < signal->bit_length; ++bit) {
            unsigned position = (unsigned)signal->start_bit + bit;
            data[position / 8u] |= (uint8_t)(((signal_values[i] >> bit) & 1u) << (position % 8u));
        }
    }
    return PduR_Transmit(frame_index, data);
}

EcuStatus Com_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms) {
    const EcuFrameConfig *frame = &com_config->frames[frame_index];
    size_t i;
    EcuStatus result = Dem_ReportPassed((uint16_t)frame_index);
    if (result == ECU_OK) {
        for (i = frame->first_signal; i < (size_t)frame->first_signal + frame->signal_count; ++i) {
            const EcuSignalConfig *signal = &com_config->signals[i];
            uint32_t value = 0u;
            unsigned bit;
            for (bit = 0u; bit < signal->bit_length; ++bit) {
                unsigned position = (unsigned)signal->start_bit + bit;
                value |= (uint32_t)((data[position / 8u] >> (position % 8u)) & 1u) << bit;
            }
            signal_values[i] = value;
            signal_valid[i] = 1u;
        }
        rx_at_ms[frame_index] = now_ms;
        rx_seen[frame_index] = 1u;
    }
    return result;
}

EcuStatus Com_AdvanceTime(uint64_t now_ms) {
    size_t i;
    EcuStatus result = ECU_OK;
    for (i = 0u; i < com_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &com_config->frames[i];
        if ((frame->direction == 0u) && (rx_seen[i] != 0u) &&
            (signal_valid[frame->first_signal] != 0u) &&
            ((now_ms - rx_at_ms[i]) >= frame->timeout_ms)) {
            size_t j;
            result = Dem_ReportFailed((uint16_t)i);
            if (result != ECU_OK) {
                break;
            }
            for (j = frame->first_signal; j < (size_t)frame->first_signal + frame->signal_count;
                 ++j) {
                signal_valid[j] = 0u;
            }
        }
    }
    return result;
}
