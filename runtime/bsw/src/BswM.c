/** @file BswM with one IMMEDIATE ComM mode input and a single action, run in caller context.
 * BSWM_STATE protects module data only (ADR 0018); the action runs outside it.
 */
#include "BswM.h"
#include "BswM_ComM.h"
#include "SchM_BswM.h"
#define BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
/* All BSWM_STATE. pending implies processing: an input received while an action runs
 * is postponed to that processor, never run recursively (SWS_BswM_00069). The port
 * takes its value just before arbitration (SWS_BswM_00281), so several postponed
 * inputs arbitrate once with the latest. Init and Deinit keep processing, so no
 * second processor overtakes an action in flight. Every input checks configuration and
 * Deinit clears pending, so pending also implies a configuration. */
static const BswM_ConfigType *configuration BSWM_VAR_CLEARED; /**< NULL when uninitialized. */
static ComM_ModeType pending_mode BSWM_VAR_CLEARED;           /**< Latest postponed input. */
static boolean pending BSWM_VAR_CLEARED;    /**< pending_mode awaits arbitration. */
static boolean processing BSWM_VAR_CLEARED; /**< An arbitration is in flight. */
#define BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
#define BSWM_START_SEC_CODE
#include "BswM_MemMap.h"
BSWM_CODE void BswM_Init(const BswM_ConfigType *ConfigPtr)
{
    if ((ConfigPtr != NULL_PTR) && (ConfigPtr->action != NULL_PTR)) {
        SchM_Enter_BswM_BSWM_STATE();
        configuration = ConfigPtr;
        pending = FALSE;
        SchM_Exit_BswM_BSWM_STATE();
    }
}
BSWM_CODE void BswM_Deinit(void)
{
    SchM_Enter_BswM_BSWM_STATE();
    configuration = NULL_PTR;
    pending = FALSE;
    SchM_Exit_BswM_BSWM_STATE();
}
BSWM_CODE void BswM_MainFunction(void)
{
    /* SWS_BswM_00075 evaluates rules with DEFERRED inputs here. The selected
     * configuration contains only IMMEDIATE inputs, evaluated in their actual
     * callbacks. There is no deferred work and no added periodic arbitration.
     */
}
/* Called by the claimed processor only; arbitrates postponed inputs until none is left. */
static BSWM_CODE void BswM_Process(void)
{
    boolean owner = TRUE;
    while (owner == TRUE) {
        BswM_ModeActionType action = NULL_PTR;
        NetworkHandleType network = 0u;
        ComM_ModeType mode = COMM_NO_COMMUNICATION;
        SchM_Enter_BswM_BSWM_STATE();
        if (pending == TRUE) {
            pending = FALSE;
            action = configuration->action;
            network = configuration->channel;
            mode = pending_mode;
        } else {
            processing = FALSE;
            owner = FALSE;
        }
        SchM_Exit_BswM_BSWM_STATE();
        if (owner == TRUE) {
            /* The single-item action list ends either way; runtime failure
             * reporting (BswMReportFailRuntimeErrorId) is not selected. */
            (void)action(network, mode);
        }
    }
}
BSWM_CODE void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType RequestedMode)
{
    boolean owner = FALSE;
    SchM_Enter_BswM_BSWM_STATE();
    if ((configuration != NULL_PTR) && (Network == configuration->channel) &&
        (RequestedMode <= COMM_FULL_COMMUNICATION)) {
        pending_mode = RequestedMode;
        pending = TRUE;
        if (processing == FALSE) {
            processing = TRUE;
            owner = TRUE;
        }
    }
    SchM_Exit_BswM_BSWM_STATE();
    if (owner == TRUE) {
        BswM_Process();
    }
}
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
