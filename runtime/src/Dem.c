#include "Dem.h"
#include "NvM.h"

#define DTC_AVAILABILITY UINT8_C(0x7f)

static const EcuDtcConfig *active_dtc;
static uint8_t event_status;
static uint8_t dtc_setting_enabled;

static EcuStatus SetStatus(uint8_t updated)
{
    EcuStatus result;
    if (updated == event_status) {
        return ECU_OK;
    }
    result = NvM_Write(updated);
    if (result == ECU_OK) {
        event_status = updated;
    }
    return result;
}

EcuStatus Dem_Init(const EcuConfig *config, const char *nvm_path)
{
    uint8_t saved;
    EcuStatus result;
    active_dtc = NULL;
    event_status = 0x50u;
    dtc_setting_enabled = 1u;
    if (config->diagnostic == NULL || config->diagnostic->dtc == NULL) {
        return ECU_OK;
    }
    if (nvm_path == NULL || nvm_path[0] == '\0') {
        return ECU_ERR_CONFIG;
    }
    result = NvM_Init(config, nvm_path, &saved);
    if (result != ECU_OK) {
        return result;
    }
    active_dtc = config->diagnostic->dtc;
    event_status = saved;
    /* A pending DTC survives only when the previous cycle saw a failure. */
    return SetStatus((uint8_t)((saved & (uint8_t)~UINT8_C(0x06)) |
                               ((saved & 0x02u) != 0u ? UINT8_C(0x04) : 0u) |
                               UINT8_C(0x40)));
}

EcuStatus Dem_DisableDTCSetting(void)
{
    if (active_dtc == NULL) {
        return ECU_ERR_CONFIG;
    }
    dtc_setting_enabled = 0u;
    return ECU_OK;
}

EcuStatus Dem_EnableDTCSetting(void)
{
    if (active_dtc == NULL) {
        return ECU_ERR_CONFIG;
    }
    dtc_setting_enabled = 1u;
    return ECU_OK;
}

EcuStatus Dem_ReportPassed(uint16_t frame_index)
{
    if (active_dtc == NULL || active_dtc->monitor_frame_index != frame_index || dtc_setting_enabled == 0u) {
        return ECU_OK;
    }
    return SetStatus((uint8_t)(event_status & (uint8_t)~UINT8_C(0x51)));
}

EcuStatus Dem_ReportFailed(uint16_t frame_index)
{
    if (active_dtc == NULL || active_dtc->monitor_frame_index != frame_index || dtc_setting_enabled == 0u) {
        return ECU_OK;
    }
    return SetStatus((uint8_t)((event_status & (uint8_t)~UINT8_C(0x50)) | UINT8_C(0x2f)));
}

uint8_t Dem_FilterDtc(uint8_t mask, uint32_t *code, uint8_t *status)
{
    if (active_dtc == NULL || (event_status & mask & DTC_AVAILABILITY) == 0u) {
        return 0u;
    }
    return Dem_GetSupportedDtc(code, status);
}

uint8_t Dem_GetSupportedDtc(uint32_t *code, uint8_t *status)
{
    if (active_dtc == NULL) {
        return 0u;
    }
    *code = active_dtc->code;
    *status = event_status;
    return 1u;
}

EcuStatus Dem_ClearAll(void)
{
    if (active_dtc == NULL) {
        return ECU_ERR_CONFIG;
    }
    return SetStatus(0x50u);
}
