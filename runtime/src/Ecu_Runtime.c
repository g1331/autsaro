#include "Ecu_Runtime.h"
#include "Dem.h"
#include "CanIf.h"
#include "Com.h"
#include "Os.h"
#include "CanTp.h"
#include "Dcm.h"
#include "LSduR.h"
#include "PduR.h"

static uint8_t HasReadableDiagnosticSignal(const EcuConfig *config, uint16_t id)
{
    size_t frame_index;
    for (frame_index = 0u; frame_index < config->frame_count; ++frame_index) {
        const EcuFrameConfig *frame = &config->frames[frame_index];
        size_t signal_index;
        if (frame->direction != 1u) {
            continue;
        }
        for (signal_index = frame->first_signal;
             signal_index < (size_t)frame->first_signal + frame->signal_count; ++signal_index) {
            if (config->signals[signal_index].id == id &&
                config->signals[signal_index].bit_length == 32u) {
                return 1u;
            }
        }
    }
    return 0u;
}

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
    if (config->diagnostic != NULL) {
        const EcuDiagnosticConfig *diagnostic = config->diagnostic;
        if (diagnostic->request_can_id > 0x7ffu || diagnostic->response_can_id > 0x7ffu ||
            diagnostic->request_can_id == diagnostic->response_can_id ||
            diagnostic->s3_ms == 0u || diagnostic->n_bs_ms == 0u || diagnostic->n_cr_ms == 0u ||
            diagnostic->did == 0u || diagnostic->did == 0xf186u ||
            diagnostic->did_signal_ids == NULL || diagnostic->did_signal_count == 0u ||
            diagnostic->did_signal_count > ECU_DIAG_MAX_DID_SIGNALS) {
            return ECU_ERR_CONFIG;
        }
        if (diagnostic->reset_routine != NULL &&
            (diagnostic->did_writers == NULL || diagnostic->reset_routine->start == NULL)) {
            return ECU_ERR_CONFIG;
        }
        for (i = 0; i < config->frame_count; ++i) {
            if (config->frames[i].id == diagnostic->request_can_id ||
                config->frames[i].id == diagnostic->response_can_id) {
                return ECU_ERR_CONFIG;
            }
        }
        for (i = 0; i < diagnostic->did_signal_count; ++i) {
            if (!HasReadableDiagnosticSignal(config, diagnostic->did_signal_ids[i])) {
                return ECU_ERR_CONFIG;
            }
            if (diagnostic->did_writers != NULL && diagnostic->did_writers[i] == NULL) {
                return ECU_ERR_CONFIG;
            }
            for (j = 0; j < i; ++j) {
                if (diagnostic->did_signal_ids[j] == diagnostic->did_signal_ids[i]) {
                    return ECU_ERR_CONFIG;
                }
            }
        }
        if (diagnostic->dtc != NULL) {
            const EcuDtcConfig *dtc = diagnostic->dtc;
            if (dtc->code < 0x100u || dtc->code > 0xfffffeu ||
                dtc->monitor_frame_index >= config->frame_count ||
                config->frames[dtc->monitor_frame_index].direction != 0u ||
                config->frames[dtc->monitor_frame_index].signal_count == 0u ||
                config->frames[dtc->monitor_frame_index].timeout_ms == 0u) {
                return ECU_ERR_CONFIG;
            }
        }
    }
    return ECU_OK;
}

EcuStatus Ecu_Init(const EcuConfig *config, CanTxSink sink, const char *nvm_path)
{
    EcuStatus result = ValidateConfig(config);
    if (result != ECU_OK || sink == NULL) {
        return ECU_ERR_CONFIG;
    }
    result = Dem_Init(config, nvm_path);
    if (result != ECU_OK) {
        return result;
    }
    Com_Init(config);
    CanIf_Init(config);
    Can_Init(sink);
    Os_Init(config);
    PduR_Init(config);
    LSduR_Init(config->diagnostic);
    CanTp_Init(config->diagnostic);
    Dcm_Init(config->diagnostic);
    return ECU_OK;
}
