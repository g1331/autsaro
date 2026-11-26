/** @file BswM with IMMEDIATE ComM mode inputs and generated actions, run in caller context.
 * BSWM_STATE protects module data only; the action runs outside it.
 */
#include "BswM.h"
#include "BswM_ComM.h"
#include "SchM_BswM.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "BswM.c -> SchM_BswM.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "BswM.c -> SchM_BswM.h: incompatible AUTOSAR release"
#endif
#include "Ecu_ProfileConfig.h"
#define BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
/* All BSWM_STATE. Postponed inputs imply processing: an input received while an action
 * runs is postponed to that processor, never run recursively, and each is arbitrated
 * after it (SWS_BswM_00069). The port takes its value just before arbitration
 * (SWS_BswM_00281), so every postponed input arbitrates the latest value. Init and
 * Deinit keep processing, so no second processor overtakes an action in flight. Every
 * input checks configuration and Deinit drops postponed inputs, so they also imply a
 * configuration. */
static const BswM_ConfigType *bswm_configuration BSWM_VAR_CLEARED; /**< NULL when uninitialized. */
typedef struct {
    ComM_ModeType mode; /**< Latest indication for this port. */
    uint32 postponed;   /**< Inputs awaiting arbitration. */
} BswM_PortType;
static BswM_PortType
    bswm_ports[COMM_CHANNEL_CAPACITY] BSWM_VAR_CLEARED; /**< Port states in configuration order;
                                                           cleared by Init or Deinit. */
static boolean processing BSWM_VAR_CLEARED;             /**< An arbitration is in flight. */
#define BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "BswM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "BswM.c -> BswM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "BswM.c -> BswM_MemMap.h: incompatible AUTOSAR release"
#endif
#define BSWM_START_SEC_CODE
#include "BswM_MemMap.h"
static BSWM_CODE void BswM_ResetPorts(void)
{
    for (uint16 i = 0u; i < COMM_CHANNEL_CAPACITY; ++i) {
        bswm_ports[i].mode = COMM_NO_COMMUNICATION;
        bswm_ports[i].postponed = 0u;
    }
}
static BSWM_CODE boolean BswM_Valid(const BswM_ConfigType *config)
{
    boolean valid =
        (config != NULL_PTR) && (config->channels != NULL_PTR) && (config->channel_count != 0u) &&
        (config->channel_count <= COMM_CHANNEL_CAPACITY) && (config->action != NULL_PTR);
    if (valid) {
        for (uint16 i = 0u; (i < config->channel_count) && valid; ++i) {
            for (uint16 j = 0u; (j < i) && valid; ++j) {
                valid = config->channels[i] != config->channels[j];
            }
        }
    }
    return valid;
}
BSWM_CODE void BswM_Init(const BswM_ConfigType *ConfigPtr)
{
    if (BswM_Valid(ConfigPtr)) {
        SchM_Enter_BswM_BSWM_STATE();
        bswm_configuration = ConfigPtr;
        BswM_ResetPorts();
        SchM_Exit_BswM_BSWM_STATE();
    }
}
BSWM_CODE void BswM_Deinit(void)
{
    SchM_Enter_BswM_BSWM_STATE();
    bswm_configuration = NULL_PTR;
    BswM_ResetPorts();
    SchM_Exit_BswM_BSWM_STATE();
}
/* MR12 RULE 8.7 VIOLATION: standard main function (SWS_BswM_00053) that the selected
 * all-IMMEDIATE profile does not schedule; see the BswM deviation record. */
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
    while (owner) {
        BswM_ModeActionType action = NULL_PTR;
        NetworkHandleType network = 0u;
        ComM_ModeType mode = COMM_NO_COMMUNICATION;
        SchM_Enter_BswM_BSWM_STATE();
        if (bswm_configuration != NULL_PTR) {
            for (uint16 i = 0u; i < bswm_configuration->channel_count; ++i) {
                BswM_PortType *port = &bswm_ports[i];
                if (port->postponed != 0u) {
                    --port->postponed;
                    action = bswm_configuration->action;
                    network = bswm_configuration->channels[i];
                    mode = port->mode;
                    break;
                }
            }
        }
        if (action == NULL_PTR) {
            processing = FALSE;
            owner = FALSE;
        }
        SchM_Exit_BswM_BSWM_STATE();
        if (owner) {
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
    if ((bswm_configuration != NULL_PTR) && (RequestedMode <= COMM_FULL_COMMUNICATION)) {
        for (uint16 i = 0u; i < bswm_configuration->channel_count; ++i) {
            if (Network == bswm_configuration->channels[i]) {
                bswm_ports[i].mode = RequestedMode;
                ++bswm_ports[i].postponed;
                if (!processing) {
                    processing = TRUE;
                    owner = TRUE;
                }
                break;
            }
        }
    }
    SchM_Exit_BswM_BSWM_STATE();
    if (owner) {
        BswM_Process();
    }
}
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
