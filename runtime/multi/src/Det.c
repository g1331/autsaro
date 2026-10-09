/** @file R24-11 reporting behavior for the selected no-storage DET. */
#include "Det.h"
#include "Det_Host.h"
#include <stddef.h>

#define DET_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"
static const Det_ConfigType *configuration DET_VAR_CLEARED;
static boolean initialized DET_VAR_CLEARED;
#define DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"

#define DET_START_SEC_CODE
#include "Det_MemMap.h"
static DET_CODE boolean Det_ValidHooks(const Det_ErrorHookType *hooks, uint16 count) {
    boolean valid = TRUE;
    uint16 index;
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
    initialized = FALSE;
    configuration = NULL;
    if (ConfigPtr != NULL) {
        if ((Det_ValidHooks(ConfigPtr->error_hooks, ConfigPtr->error_hook_count) == FALSE) ||
            (Det_ValidHooks(ConfigPtr->runtime_callouts,
                            ConfigPtr->runtime_callout_count) == FALSE)) {
            Ecu_DetHalt();
        }
    }
    configuration = ConfigPtr;
    initialized = TRUE;
}

DET_CODE void Det_Start(void) {
    /* SWS_Det_00025 permits no action when startup storage is not selected. */
}

DET_CODE Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId, uint8 InstanceId,
                                              uint8 ApiId, uint8 ErrorId) {
    uint16 index;
    if ((initialized == TRUE) && (configuration != NULL)) {
        for (index = 0u; index < configuration->runtime_callout_count; ++index) {
            (void)configuration->runtime_callouts[index](ModuleId, InstanceId, ApiId, ErrorId);
        }
    }
    return E_OK;
}

DET_CODE Std_ReturnType Det_ReportError(uint16 ModuleId, uint8 InstanceId,
                                       uint8 ApiId, uint8 ErrorId) {
    uint16 index;
    if ((initialized == TRUE) && (configuration != NULL) &&
        (Ecu_DetEnterDevelopmentReport() == TRUE)) {
        for (index = 0u; index < configuration->error_hook_count; ++index) {
            (void)configuration->error_hooks[index](ModuleId, InstanceId, ApiId, ErrorId);
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
