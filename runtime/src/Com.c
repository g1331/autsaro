#include "Com.h"
#include "PduR.h"

static const EcuConfig *active_config;
static uint32_t signal_values[ECU_MAX_SIGNALS];
static uint8_t signal_valid[ECU_MAX_SIGNALS];
static uint64_t rx_at_ms[ECU_MAX_FRAMES];
static uint8_t rx_seen[ECU_MAX_FRAMES];

static size_t FindSignal(uint16_t id)
{
    size_t i;
    for (i = 0; i < active_config->signal_count; ++i) {
        if (active_config->signals[i].id == id) {
            return i;
        }
    }
    return active_config->signal_count;
}

static size_t FrameForSignal(size_t signal_index)
{
    size_t i;
    for (i = 0; i < active_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &active_config->frames[i];
        if (signal_index >= frame->first_signal &&
            signal_index - frame->first_signal < frame->signal_count) {
            return i;
        }
    }
    return active_config->frame_count;
}

void Com_Init(const EcuConfig *config)
{
    size_t i;
    active_config = config;
    for (i = 0; i < config->signal_count; ++i) {
        signal_values[i] = config->signals[i].initial_value;
        signal_valid[i] = (uint8_t)(config->frames[FrameForSignal(i)].direction == 1u);
    }
    for (i = 0; i < config->frame_count; ++i) {
        rx_at_ms[i] = 0;
        rx_seen[i] = 0;
    }
}

EcuStatus Com_SetSignal(uint16_t id, uint32_t value)
{
    size_t i = FindSignal(id);
    const EcuSignalConfig *signal;
    if (i == active_config->signal_count) {
        return ECU_ERR_SIGNAL_ID;
    }
    if (active_config->frames[FrameForSignal(i)].direction != 1u) {
        return ECU_ERR_DIRECTION;
    }
    signal = &active_config->signals[i];
    if (signal->bit_length < 32u && value >= (UINT32_C(1) << signal->bit_length)) {
        return ECU_ERR_SIGNAL_VALUE;
    }
    signal_values[i] = value;
    return ECU_OK;
}

EcuStatus Com_GetSignal(uint16_t id, uint32_t *value, uint8_t *valid)
{
    size_t i = FindSignal(id);
    if (i == active_config->signal_count) {
        return ECU_ERR_SIGNAL_ID;
    }
    *value = signal_values[i];
    *valid = signal_valid[i];
    return ECU_OK;
}

EcuStatus Com_TriggerTransmit(size_t frame_index)
{
    const EcuFrameConfig *frame = &active_config->frames[frame_index];
    uint8_t data[8] = {0};
    size_t i;
    for (i = frame->first_signal; i < (size_t)frame->first_signal + frame->signal_count; ++i) {
        const EcuSignalConfig *signal = &active_config->signals[i];
        unsigned bit;
        for (bit = 0; bit < signal->bit_length; ++bit) {
            unsigned position = (unsigned)signal->start_bit + bit;
            data[position / 8u] |= (uint8_t)(((signal_values[i] >> bit) & 1u) << (position % 8u));
        }
    }
    return PduR_Transmit(frame_index, data);
}

EcuStatus Com_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms)
{
    const EcuFrameConfig *frame = &active_config->frames[frame_index];
    size_t i;
    for (i = frame->first_signal; i < (size_t)frame->first_signal + frame->signal_count; ++i) {
        const EcuSignalConfig *signal = &active_config->signals[i];
        uint32_t value = 0;
        unsigned bit;
        for (bit = 0; bit < signal->bit_length; ++bit) {
            unsigned position = (unsigned)signal->start_bit + bit;
            value |= (uint32_t)((data[position / 8u] >> (position % 8u)) & 1u) << bit;
        }
        signal_values[i] = value;
        signal_valid[i] = 1u;
    }
    rx_at_ms[frame_index] = now_ms;
    rx_seen[frame_index] = 1u;
    return ECU_OK;
}

void Com_AdvanceTime(uint64_t now_ms)
{
    size_t i;
    for (i = 0; i < active_config->frame_count; ++i) {
        const EcuFrameConfig *frame = &active_config->frames[i];
        if (frame->direction == 0u && rx_seen[i] &&
            now_ms - rx_at_ms[i] >= frame->timeout_ms) {
            size_t j;
            for (j = frame->first_signal; j < (size_t)frame->first_signal + frame->signal_count; ++j) {
                signal_valid[j] = 0u;
            }
        }
    }
}
