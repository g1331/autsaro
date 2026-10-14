/** @file R24-11 reporting behavior for the selected no-storage DET. */
#include "Det.h"
#include "Det_Host.h"
#include "SchM_Det.h"
#include <stddef.h>

#define DET_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"
static const Det_ConfigType *configuration DET_VAR_CLEARED;
static boolean initialized DET_VAR_CLEARED;
#define DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"

#define DET_START_SEC_CODE
#include "Det_MemMap.h"
static DET_CODE boolean Det_ValidHooks(const Det_ErrorHookType *hooks, uint16_t count) {
    boolean valid = TRUE;
    uint16_t index;
    if ((count != 0u) && (hooks == NULL)) {
        valid = FALSE;
    } else {
        for (index = 0u; index < count; ++index) {
            if (hooks[index] == NULL) {
                valid = FALSE;
            }
        }
    }
    return valid;
}

DET_CODE void Det_Init(const Det_ConfigType *ConfigPtr) {
    SchM_Enter_Det_DET_STATE();
    initialized = FALSE;
    configuration = NULL;
    SchM_Exit_Det_DET_STATE();
    if (ConfigPtr != NULL) {
        if ((Det_ValidHooks(ConfigPtr->error_hooks, ConfigPtr->error_hook_count) == FALSE) ||
            (Det_ValidHooks(ConfigPtr->runtime_callouts, ConfigPtr->runtime_callout_count) ==
             FALSE)) {
            Ecu_DetHalt();
        }
    }
    SchM_Enter_Det_DET_STATE();
    configuration = ConfigPtr;
    initialized = TRUE;
    SchM_Exit_Det_DET_STATE();
}

DET_CODE void Det_Start(void) {
    /* SWS_Det_00025 permits no action when startup storage is not selected. */
}

DET_CODE Std_ReturnType Det_ReportRuntimeError(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                                               uint8_t ErrorId) {
    const Det_ConfigType *config = NULL;
    SchM_Enter_Det_DET_STATE();
    if (initialized == TRUE) {
        config = configuration;
    }
    SchM_Exit_Det_DET_STATE();
    /* Callouts are caller code: they run outside the area, and may report again. */
    if (config != NULL) {
        uint16_t index;
        for (index = 0u; index < config->runtime_callout_count; ++index) {
            (void)config->runtime_callouts[index](ModuleId, InstanceId, ApiId, ErrorId);
        }
    }
    return E_OK;
}

DET_CODE Std_ReturnType Det_ReportError(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                                        uint8_t ErrorId) {
    const Det_ConfigType *config = NULL;
    SchM_Enter_Det_DET_STATE();
    if (initialized == TRUE) {
        config = configuration;
    }
    SchM_Exit_Det_DET_STATE();
    if ((config != NULL) && (Ecu_DetEnterDevelopmentReport() == TRUE)) {
        uint16_t index;
        for (index = 0u; index < config->error_hook_count; ++index) {
            (void)config->error_hooks[index](ModuleId, InstanceId, ApiId, ErrorId);
        }
    }
    Ecu_DetHalt();
    /* The host halt never returns. Retain the standard signature without a
     * fabricated return status even if an invalid adapter were linked.
     */
    for (;;) {
    }
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"
