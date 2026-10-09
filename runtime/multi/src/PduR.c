#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_Dcm.h"
#include "PduR_CanIf.h"
#define PDUR_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "PduR_MemMap.h"
static const PduR_PBConfigType *configuration PDUR_VAR_CLEARED;
#define PDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "PduR_MemMap.h"
#define PDUR_START_SEC_CODE
#include "PduR_MemMap.h"
static PDUR_CODE boolean PduR_Valid(const PduR_PBConfigType *config) {
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
            const PduR_TxRouteType *route = &config->transmit[i];
            if (((route->module != PDUR_UP_COM) && (route->module != PDUR_UP_DCM)) ||
                ((route->confirmation == NULL_PTR) || (route->transmit == NULL_PTR))) {
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
PDUR_CODE void PduR_Init(const PduR_PBConfigType *ConfigPtr) {
    if (PduR_Valid(ConfigPtr) == TRUE) {
        configuration = ConfigPtr;
    }
}
PDUR_CODE PduR_PBConfigIdType PduR_GetConfigurationId(void) {
    return (configuration == NULL_PTR) ? 0u : configuration->id;
}
static PDUR_CODE Std_ReturnType PduR_Transmit(PduR_UpperType module, PduIdType id,
                                              const PduInfoType *info) {
    Std_ReturnType result = E_NOT_OK;
    if ((configuration != NULL_PTR) && (info != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            const PduR_TxRouteType *route = &configuration->transmit[i];
            if ((route->module == module) && (route->upper == id)) {
                result = route->transmit(route->lower, info);
                break;
            }
        }
    }
    return result;
}
PDUR_CODE Std_ReturnType PduR_ComTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    return PduR_Transmit(PDUR_UP_COM, TxPduId, PduInfoPtr);
}
PDUR_CODE Std_ReturnType PduR_DcmTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    return PduR_Transmit(PDUR_UP_DCM, TxPduId, PduInfoPtr);
}
PDUR_CODE void PduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr) {
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < configuration->receive_count; ++i) {
            const PduR_RxRouteType *route = &configuration->receive[i];
            if (route->lower == RxPduId) {
                route->indication(route->upper, PduInfoPtr);
                break;
            }
        }
    }
}
PDUR_CODE void PduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result) {
    if (configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            const PduR_TxRouteType *route = &configuration->transmit[i];
            if (route->lower == TxPduId) {
                route->confirmation(route->upper, result);
                break;
            }
        }
    }
}
PDUR_CODE Std_ReturnType PduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr) {
    Std_ReturnType result = E_NOT_OK;
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            const PduR_TxRouteType *route = &configuration->transmit[i];
            if ((route->lower == TxPduId) && (route->trigger != NULL_PTR)) {
                result = route->trigger(route->upper, PduInfoPtr);
                break;
            }
        }
    }
    return result;
}
#define PDUR_STOP_SEC_CODE
#include "PduR_MemMap.h"
