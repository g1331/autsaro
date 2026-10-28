/** @file R24-11 reporting behavior for the selected no-storage DET. */
#include "Det.h"
#include "Det_Host.h"
#include "SchM_Det.h"
#include <stddef.h>

#define DET_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"
static const Det_ConfigType *det_configuration DET_VAR_CLEARED; /**< DET_STATE; reset by Init. */
static boolean initialized DET_VAR_CLEARED; /**< DET_STATE; Init publishes configuration first. */
#define DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Det_MemMap.h"

#define DET_START_SEC_CODE
#include "Det_MemMap.h"
static DET_CODE boolean Det_ValidHooks(const Det_ErrorHookType *hooks, uint16 count)
{
    boolean valid = TRUE;
    if ((count != 0u) && (hooks == NULL)) {
        valid = FALSE;
    } else {
        for (uint16 index = 0u; index < count; ++index) {
            if (hooks[index] == NULL_PTR) {
                valid = FALSE;
            }
        }
    }
    return valid;
}

DET_CODE void Det_Init(const Det_ConfigType *ConfigPtr)
{
    SchM_Enter_Det_DET_STATE();
    initialized = FALSE;
    det_configuration = NULL_PTR;
    SchM_Exit_Det_DET_STATE();
    if (ConfigPtr != NULL_PTR) {
        if ((Det_ValidHooks(ConfigPtr->error_hooks, ConfigPtr->error_hook_count) == FALSE) ||
            (Det_ValidHooks(ConfigPtr->runtime_callouts, ConfigPtr->runtime_callout_count) ==
             FALSE)) {
            Det_HostHalt();
        }
    }
    SchM_Enter_Det_DET_STATE();
    det_configuration = ConfigPtr;
    initialized = TRUE;
    SchM_Exit_Det_DET_STATE();
}

DET_CODE void Det_Start(void)
{
    /* SWS_Det_00025 permits no action when startup storage is not selected. */
}

DET_CODE Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId,
                                               uint8 InstanceId,
                                               uint8 ApiId,
                                               uint8 ErrorId)
{
    const Det_ConfigType *config = NULL_PTR;
    SchM_Enter_Det_DET_STATE();
    if (initialized == TRUE) {
        config = det_configuration;
    }
    SchM_Exit_Det_DET_STATE();
    /* Callouts are caller code: they run outside the area, and may report again. */
    if (config != NULL_PTR) {
        uint16 index;
        for (index = 0u; index < config->runtime_callout_count; ++index) {
            (void)config->runtime_callouts[index](ModuleId, InstanceId, ApiId, ErrorId);
        }
    }
    return E_OK;
}

DET_CODE Std_ReturnType Det_ReportError(uint16 ModuleId,
                                        uint8 InstanceId,
                                        uint8 ApiId,
                                        uint8 ErrorId)
{
    const Det_ConfigType *config = NULL_PTR;
    SchM_Enter_Det_DET_STATE();
    if (initialized == TRUE) {
        config = det_configuration;
    }
    SchM_Exit_Det_DET_STATE();
    if (config != NULL_PTR) {
        if (Det_HostEnterDevelopmentReport() == TRUE) {
            uint16 index;
            for (index = 0u; index < config->error_hook_count; ++index) {
                (void)config->error_hooks[index](ModuleId, InstanceId, ApiId, ErrorId);
            }
        }
    }
    Det_HostHalt();
    /* The host halt never returns. Retain the standard signature without a
     * fabricated return status even if an invalid adapter were linked.
     */
    for (;;) {
    }
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"
