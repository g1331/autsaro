#include "BswM.h"
#include "BswM_ComM.h"
#include "SchM_BswM.h"
#define BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
static const BswM_ConfigType *configuration BSWM_VAR_CLEARED;
static ComM_ModeType input_mode BSWM_VAR_CLEARED;
static ComM_ModeType pending_mode BSWM_VAR_CLEARED;
static boolean processing BSWM_VAR_CLEARED;
static boolean pending BSWM_VAR_CLEARED;
#define BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
#define BSWM_START_SEC_CODE
#include "BswM_MemMap.h"
BSWM_CODE void BswM_Init(const BswM_ConfigType *ConfigPtr) {
    SchM_Enter_BswM_BSWM_STATE();
    if ((ConfigPtr != NULL_PTR) && (ConfigPtr->initial_mode <= COMM_FULL_COMMUNICATION) &&
        (ConfigPtr->action != NULL_PTR)) {
        configuration = ConfigPtr;
        input_mode = ConfigPtr->initial_mode;
        processing = FALSE;
        pending = FALSE;
        pending_mode = COMM_NO_COMMUNICATION;
    }
    SchM_Exit_BswM_BSWM_STATE();
}
BSWM_CODE void BswM_Deinit(void) {
    SchM_Enter_BswM_BSWM_STATE();
    configuration = NULL_PTR;
    processing = FALSE;
    pending = FALSE;
    SchM_Exit_BswM_BSWM_STATE();
}
BSWM_CODE void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType RequestedMode) {
    SchM_Enter_BswM_BSWM_STATE();
    if ((configuration != NULL_PTR) && (Network == configuration->channel) &&
        (RequestedMode <= COMM_FULL_COMMUNICATION)) {
        pending_mode = RequestedMode;
        pending = TRUE;
        if (processing == FALSE) {
            processing = TRUE;
            while ((pending == TRUE) && (configuration != NULL_PTR)) {
                Std_ReturnType result;
                pending = FALSE;
                /* Update immediately before arbitration, including postponed requests. */
                input_mode = pending_mode;
                result = configuration->action(Network, input_mode);
                if (result != E_OK) {
                    /* Single-action list stops here. Optional configured failure
                     * runtime reporting is not selected; lower state stays factual.
                     */
                }
            }
            processing = FALSE;
        }
    }
    SchM_Exit_BswM_BSWM_STATE();
}
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
