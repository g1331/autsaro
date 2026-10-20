/** @file Standard PduR with immutable one-to-one routes between Com, Dcm and lower layers. */
#include "PduR.h"
#include "SchM_PduR.h"
#include "PduR_Com.h"
#include "PduR_Dcm.h"
#include "PduR_CanIf.h"
#include "PduR_CanTp.h"
#include "Det.h"
#define PDUR_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "PduR_MemMap.h"
/* PDUR_STATE protects pointer publication and reads. The accepted objects are immutable. Init
 * publishes only while NULL and never resets the published pointer. */
static const PduR_PBConfigType *configuration
    PDUR_VAR_CLEARED; /**< First accepted route set; Init never replaces it. */
#define PDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "PduR_MemMap.h"
#define PDUR_START_SEC_CODE
#include "PduR_MemMap.h"
static PDUR_CODE boolean PduR_Valid(const PduR_PBConfigType *config)
{
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->receive_count <= 32u) && (config->transmit_count <= 32u) &&
        ((config->receive_count == 0u) || (config->receive != NULL_PTR)) &&
        ((config->transmit_count == 0u) || (config->transmit != NULL_PTR)) &&
        (config->transport_count <= 32u) &&
        ((config->transport_count == 0u) || (config->transport != NULL_PTR))) {
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
            const PduR_TxRouteType *route = &config->transmit[i];
            /* CanTp confirms through the transport routes; IF routes confirm here. */
            if (((route->module != PDUR_UP_COM) && (route->module != PDUR_UP_DCM)) ||
                ((route->lower_module != PDUR_LO_LSDUR) &&
                 (route->lower_module != PDUR_LO_CANTP)) ||
                ((route->lower_module == PDUR_LO_LSDUR) && (route->confirmation == NULL_PTR)) ||
                (route->transmit == NULL_PTR)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if (((route->lower_module == config->transmit[j].lower_module) &&
                     (route->lower == config->transmit[j].lower)) ||
                    ((route->module == config->transmit[j].module) &&
                     (route->upper == config->transmit[j].upper))) {
                    valid = FALSE;
                }
            }
        }
        for (i = 0u; i < config->transport_count; ++i) {
            uint16 j;
            const PduR_TpRouteType *route = &config->transport[i];
            if ((route->start == NULL_PTR) || (route->copy_rx == NULL_PTR) ||
                (route->receive == NULL_PTR) || (route->copy_tx == NULL_PTR) ||
                (route->confirmation == NULL_PTR)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if ((route->lower_rx == config->transport[j].lower_rx) ||
                    (route->lower_tx == config->transport[j].lower_tx) ||
                    (route->upper_rx == config->transport[j].upper_rx) ||
                    (route->upper_tx == config->transport[j].upper_tx)) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
/* The area guards only the published pointer. The configuration it points to is
 * immutable and stays published, so routing reads it and calls other modules outside. */
static PDUR_CODE const PduR_PBConfigType *PduR_Configuration(void)
{
    const PduR_PBConfigType *config;
    SchM_Enter_PduR_PDUR_STATE();
    config = configuration;
    SchM_Exit_PduR_PDUR_STATE();
    return config;
}
static PDUR_CODE const PduR_TpRouteType *PduR_Transport(PduIdType id, boolean receive)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    const PduR_TpRouteType *result = NULL_PTR;
    if (config != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < config->transport_count; ++i) {
            const PduR_TpRouteType *route = &config->transport[i];
            if (((receive == TRUE) && (route->lower_rx == id)) ||
                ((receive == FALSE) && (route->lower_tx == id))) {
                result = route;
                break;
            }
        }
    }
    return result;
}
PDUR_CODE BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                                       const PduInfoType *info,
                                                       PduLengthType length,
                                                       PduLengthType *available)
{
    const PduR_TpRouteType *route = PduR_Transport(id, TRUE);
    return (route == NULL_PTR) ? BUFREQ_E_NOT_OK
                               : route->start(route->upper_rx, info, length, available);
}
PDUR_CODE BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                                 const PduInfoType *info,
                                                 PduLengthType *available)
{
    const PduR_TpRouteType *route = PduR_Transport(id, TRUE);
    return (route == NULL_PTR) ? BUFREQ_E_NOT_OK : route->copy_rx(route->upper_rx, info, available);
}
PDUR_CODE void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    const PduR_TpRouteType *route = PduR_Transport(id, TRUE);
    if (route != NULL_PTR) {
        route->receive(route->upper_rx, result);
    }
}
PDUR_CODE BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                                 const PduInfoType *info,
                                                 const RetryInfoType *retry,
                                                 PduLengthType *available)
{
    const PduR_TpRouteType *route = PduR_Transport(id, FALSE);
    return (route == NULL_PTR) ? BUFREQ_E_NOT_OK
                               : route->copy_tx(route->upper_tx, info, retry, available);
}
PDUR_CODE void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    const PduR_TpRouteType *route = PduR_Transport(id, FALSE);
    if (route != NULL_PTR) {
        route->confirmation(route->upper_tx, result);
    }
}
/* Only PDUR_UNINIT accepts a configuration: there is no PDUR_ONLINE -> PDUR_ONLINE
 * transition (SWS_PduR_00100), so routes in flight never see another configuration. */
PDUR_CODE void PduR_Init(const PduR_PBConfigType *ConfigPtr)
{
    if (PduR_Valid(ConfigPtr) == TRUE) {
        SchM_Enter_PduR_PDUR_STATE();
        if (configuration == NULL_PTR) {
            configuration = ConfigPtr;
        }
        SchM_Exit_PduR_PDUR_STATE();
    }
}
PDUR_CODE PduR_PBConfigIdType PduR_GetConfigurationId(void)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    return (config == NULL_PTR) ? 0u : config->id;
}
/* The selected configuration has no PduRRoutingPathGroup: every id is unknown and the
 * call returns with no action (SWS_PduR_00647). */
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_PduR_00615) that no
 * module of the selected program calls; see the PduR deviation record. */
PDUR_CODE void PduR_EnableRouting(PduR_RoutingPathGroupIdType id) { (void)id; }
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_PduR_00617) that no
 * module of the selected program calls; see the PduR deviation record. */
PDUR_CODE void PduR_DisableRouting(PduR_RoutingPathGroupIdType id, boolean initialize)
{
    (void)id;
    (void)initialize;
}
static PDUR_CODE Std_ReturnType PduR_Transmit(PduR_UpperType module,
                                              PduIdType id,
                                              const PduInfoType *info)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    Std_ReturnType result = E_NOT_OK;
    if ((config != NULL_PTR) && (info != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const PduR_TxRouteType *route = &config->transmit[i];
            if ((route->module == module) && (route->upper == id)) {
                result = route->transmit(route->lower, info);
                break;
            }
        }
    }
    return result;
}
PDUR_CODE Std_ReturnType PduR_ComTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    return PduR_Transmit(PDUR_UP_COM, TxPduId, PduInfoPtr);
}
PDUR_CODE Std_ReturnType PduR_DcmTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    return PduR_Transmit(PDUR_UP_DCM, TxPduId, PduInfoPtr);
}
PDUR_CODE void PduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    if ((config != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->receive_count; ++i) {
            const PduR_RxRouteType *route = &config->receive[i];
            if (route->lower == RxPduId) {
                route->indication(route->upper, PduInfoPtr);
                break;
            }
        }
    }
}
PDUR_CODE void PduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    if (config != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const PduR_TxRouteType *route = &config->transmit[i];
            if ((route->lower_module == PDUR_LO_LSDUR) && (route->lower == TxPduId)) {
                route->confirmation(route->upper, result);
                break;
            }
        }
    }
}
PDUR_CODE Std_ReturnType PduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr)
{
    const PduR_PBConfigType *config = PduR_Configuration();
    Std_ReturnType result = E_NOT_OK;
    if ((config != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < config->transmit_count; ++i) {
            const PduR_TxRouteType *route = &config->transmit[i];
            if ((route->lower_module == PDUR_LO_LSDUR) && (route->lower == TxPduId) &&
                (route->trigger != NULL_PTR)) {
                result = route->trigger(route->upper, PduInfoPtr);
                break;
            }
        }
    }
    return result;
}
#define PDUR_STOP_SEC_CODE
#include "PduR_MemMap.h"
