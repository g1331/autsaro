#include "Dem.h"
#include "NvM.h"

#define DTC_AVAILABILITY 0x7fu

static const EcuDtcConfig *active_dtc;
static uint8_t event_status;
static uint8_t dtc_setting_enabled;

static EcuStatus SetStatus(uint8_t updated) {
    EcuStatus result = ECU_OK;
    if (updated != event_status) {
        result = NvM_Write(updated);
        if (result == ECU_OK) {
            event_status = updated;
        }
    }
    return result;
}

EcuStatus Dem_Init(const EcuConfig *config, const char *nvm_path) {
    uint8_t saved;
    EcuStatus result = ECU_OK;
    active_dtc = NULL;
    event_status = 0x50u;
    dtc_setting_enabled = 1u;
    if ((config->diagnostic != NULL) && (config->diagnostic->dtc != NULL)) {
        if ((nvm_path == NULL) || (nvm_path[0] == '\0')) {
            result = ECU_ERR_CONFIG;
        } else {
            result = NvM_Init(config, nvm_path, &saved);
            if (result == ECU_OK) {
                active_dtc = config->diagnostic->dtc;
                event_status = saved;
                /* A pending DTC survives only when the previous cycle saw a failure. */
                result = SetStatus(
                    (uint8_t)((saved & 0xf9u) | (((saved & 0x02u) != 0u) ? 0x04u : 0u) | 0x40u));
            }
        }
    }
    return result;
}

EcuStatus Dem_DisableDTCSetting(void) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (active_dtc != NULL) {
        dtc_setting_enabled = 0u;
        result = ECU_OK;
    }
    return result;
}

EcuStatus Dem_EnableDTCSetting(void) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (active_dtc != NULL) {
        dtc_setting_enabled = 1u;
        result = ECU_OK;
    }
    return result;
}

EcuStatus Dem_ReportPassed(uint16_t frame_index) {
    EcuStatus result = ECU_OK;
    if ((active_dtc != NULL) && (active_dtc->monitor_frame_index == frame_index) &&
        (dtc_setting_enabled != 0u)) {
        result = SetStatus((uint8_t)(event_status & 0xaeu));
    }
    return result;
}

EcuStatus Dem_ReportFailed(uint16_t frame_index) {
    EcuStatus result = ECU_OK;
    if ((active_dtc != NULL) && (active_dtc->monitor_frame_index == frame_index) &&
        (dtc_setting_enabled != 0u)) {
        result = SetStatus((uint8_t)((event_status & 0xafu) | 0x2fu));
    }
    return result;
}

uint8_t Dem_FilterDtc(uint8_t mask, uint32_t *code, uint8_t *status) {
    uint8_t result = 0u;
    if ((active_dtc != NULL) && (((event_status & mask) & DTC_AVAILABILITY) != 0u)) {
        result = Dem_GetSupportedDtc(code, status);
    }
    return result;
}

uint8_t Dem_GetSupportedDtc(uint32_t *code, uint8_t *status) {
    uint8_t result = 0u;
    if (active_dtc != NULL) {
        *code = active_dtc->code;
        *status = event_status;
        result = 1u;
    }
    return result;
}

EcuStatus Dem_ClearAll(void) {
    EcuStatus result = ECU_ERR_CONFIG;
    if (active_dtc != NULL) {
        result = SetStatus(0x50u);
    }
    return result;
}
