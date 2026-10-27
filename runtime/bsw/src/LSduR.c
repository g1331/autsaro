/** @file R24-11 LSduR routing PDUs one-to-one over immutable configured routes. */
#include "LSduR.h"
#include "SchM_LSduR.h"
#include "LSduR_PduR.h"
#include "LSduR_CanTp.h"
#include "LSduR_CanIf.h"
#include "CanIf.h"
#include "Det.h"
#define LSDUR_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "LSduR_MemMap.h"
/* LSDUR_STATE protects pointer publication and reads. The accepted objects are immutable. Init
 * publishes only while NULL and never resets the published pointer. */
static const LSduR_PBConfigType *lsdur_configuration
    LSDUR_VAR_CLEARED; /**< First accepted route set; Init never replaces it. */
#define LSDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "LSduR_MemMap.h"
#define LSDUR_START_SEC_CODE
#include "LSduR_MemMap.h"
static LSDUR_CODE boolean LSduR_Valid(const LSduR_PBConfigType *config)
{
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->receive_count <= 32u) && (config->transmit_count <= 32u) &&
        ((config->receive_count == 0u) || (config->receive != NULL_PTR)) &&
        ((config->transmit_count == 0u) || (config->transmit != NULL_PTR))) {
        valid = TRUE;
        for (i = 0u; i < config->receive_count; ++i) {
            uint16 j;
            if (config->receive[i].indication == NULL_PTR) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if (config->receive[i].lower == config->receive[j].lower) {
                    valid = FALSE;
                }
            }
        }
        for (i = 0u; i < config->transmit_count; ++i) {
            uint16 j;
            const LSduR_TxRouteType *route = &config->transmit[i];
            if (((route->module != LSDUR_UP_PDUR) && (route->module != LSDUR_UP_CANTP)) ||
                (route->confirmation == NULL_PTR)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if ((route->lower == config->transmit[j].lower) ||
                    ((route->module == config->transmit[j].module) &&
                     (route->upper == config->transmit[j].upper))) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
/* The area guards only the published pointer. The configuration it points to is
 * immutable and stays published, so routing reads it and calls other modules outside. */
static LSDUR_CODE const LSduR_PBConfigType *LSduR_Configuration(void)
{
    const LSduR_PBConfigType *config;
    SchM_Enter_LSduR_LSDUR_STATE();
    config = lsdur_configuration;
    SchM_Exit_LSduR_LSDUR_STATE();
    return config;
}
/* Only LSDUR_UNINIT accepts a configuration: the state machine has no LSDUR_ONLINE ->
 * LSDUR_ONLINE transition, so routes in flight never see another configuration. */
LSDUR_CODE void LSduR_Init(const LSduR_PBConfigType *ConfigPtr)
{
    if (LSduR_Valid(ConfigPtr) == TRUE) {
        SchM_Enter_LSduR_LSDUR_STATE();
        if (lsdur_configuration == NULL_PTR) {
            lsdur_configuration = ConfigPtr;
        }
        SchM_Exit_LSduR_LSDUR_STATE();
    }
}
LSDUR_CODE LSduR_PBConfigIdType LSduR_GetConfigurationId(void)
{
    const LSduR_PBConfigType *config = LSduR_Configuration();
    return (config == NULL_PTR) ? 0u : config->id;
}
static LSDUR_CODE Std_ReturnType LSduR_Transmit(LSduR_UpperType module,
                                                PduIdType id,
                                                const PduInfoType *info)
{
    const LSduR_PBConfigType *config = LSduR_Configuration();
    Std_ReturnType result = E_NOT_OK;
    if ((config != NULL_PTR) && (info != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const LSduR_TxRouteType *route = &config->transmit[i];
            if ((route->module == module) && (route->upper == id)) {
                result = CanIf_Transmit(route->lower, info);
                break;
            }
        }
    }
    return result;
}
LSDUR_CODE Std_ReturnType LSduR_PduRTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    return LSduR_Transmit(LSDUR_UP_PDUR, TxPduId, PduInfoPtr);
}
LSDUR_CODE Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    return LSduR_Transmit(LSDUR_UP_CANTP, TxPduId, PduInfoPtr);
}
LSDUR_CODE void LSduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    const LSduR_PBConfigType *config = LSduR_Configuration();
    if ((config != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->receive_count; ++i) {
            const LSduR_RxRouteType *route = &config->receive[i];
            if (route->lower == RxPduId) {
                route->indication(route->upper, PduInfoPtr);
                break;
            }
        }
    }
}
LSDUR_CODE void LSduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    const LSduR_PBConfigType *config = LSduR_Configuration();
    if (config != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const LSduR_TxRouteType *route = &config->transmit[i];
            if (route->lower == TxPduId) {
                route->confirmation(route->upper, result);
                break;
            }
        }
    }
}
LSDUR_CODE Std_ReturnType LSduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr)
{
    const LSduR_PBConfigType *config = LSduR_Configuration();
    Std_ReturnType result = E_NOT_OK;
    if ((config != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const LSduR_TxRouteType *route = &config->transmit[i];
            if ((route->lower == TxPduId) && (route->trigger != NULL_PTR)) {
                result = route->trigger(route->upper, PduInfoPtr);
                break;
            }
        }
    }
    return result;
}
#define LSDUR_STOP_SEC_CODE
#include "LSduR_MemMap.h"
