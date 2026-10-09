#include "CanIf.h"
#include "Can.h"
#include "Det.h"
#include "LSduR_CanIf.h"
#include "SchM_CanIf.h"
#define CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"
static const CanIf_ConfigType *configuration CANIF_VAR_CLEARED;
static Can_ControllerStateType controller_mode CANIF_VAR_CLEARED;
static CanIf_PduModeType pdu_mode CANIF_VAR_CLEARED;
static uint32 outstanding[32] CANIF_VAR_CLEARED;
#define CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"
#define CANIF_START_SEC_CODE
#include "CanIf_MemMap.h"
static CANIF_CODE boolean CanIf_Valid(const CanIf_ConfigType *config) {
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->receive_count <= 32u) && (config->transmit_count <= 32u) &&
        ((config->receive_count == 0u) || (config->receive != NULL_PTR)) &&
        ((config->transmit_count == 0u) || (config->transmit != NULL_PTR)) &&
        (config->controller_mode != NULL_PTR) && (config->bus_off != NULL_PTR)) {
        valid = TRUE;
        for (i = 0u; i < config->receive_count; ++i) {
            uint16 j;
            const CanIf_RxPduConfigType *route = &config->receive[i];
            if ((route->can_id > 0x7ffu) || (route->minimum_length > 8u)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if ((route->can_id == config->receive[j].can_id) ||
                    (route->pdu == config->receive[j].pdu)) {
                    valid = FALSE;
                }
            }
        }
        for (i = 0u; i < config->transmit_count; ++i) {
            uint16 j;
            const CanIf_TxPduConfigType *route = &config->transmit[i];
            if ((route->can_id > 0x7ffu) || (route->maximum_length == 0u) ||
                (route->maximum_length > 8u)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if (route->pdu == config->transmit[j].pdu) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
CANIF_CODE void CanIf_Init(const CanIf_ConfigType *ConfigPtr) {
    SchM_Enter_CanIf_CANIF_STATE();
    if (CanIf_Valid(ConfigPtr) == TRUE) {
        uint16 i;
        configuration = ConfigPtr;
        controller_mode = CAN_CS_STOPPED;
        pdu_mode = CANIF_OFFLINE;
        for (i = 0u; i < 32u; ++i) {
            outstanding[i] = 0u;
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE void CanIf_DeInit(void) {
    uint16 i;
    SchM_Enter_CanIf_CANIF_STATE();
    configuration = NULL_PTR;
    controller_mode = CAN_CS_UNINIT;
    pdu_mode = CANIF_OFFLINE;
    for (i = 0u; i < 32u; ++i) {
        outstanding[i] = 0u;
    }
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE Std_ReturnType CanIf_SetControllerMode(uint8 ControllerId,
                                                  Can_ControllerStateType ControllerMode) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller)) {
        result = Can_SetControllerMode(configuration->driver_controller, ControllerMode);
        if (result == E_OK) {
            if (ControllerMode == CAN_CS_STOPPED) {
                pdu_mode = CANIF_TX_OFFLINE;
            } else if (ControllerMode == CAN_CS_SLEEP) {
                pdu_mode = CANIF_OFFLINE;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerMode(uint8 ControllerId,
                                                  Can_ControllerStateType *ControllerModePtr) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        (ControllerModePtr != NULL_PTR)) {
        result = Can_GetControllerMode(configuration->driver_controller, ControllerModePtr);
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerErrorState(uint8 ControllerId,
                                                        Can_ErrorStateType *ErrorStatePtr) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        (ErrorStatePtr != NULL_PTR)) {
        result = Can_GetControllerErrorState(configuration->driver_controller, ErrorStatePtr);
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest) {
    Std_ReturnType result = E_NOT_OK;
    Can_ControllerStateType actual_mode = CAN_CS_UNINIT;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        (Can_GetControllerMode(configuration->driver_controller, &actual_mode) == E_OK) &&
        (actual_mode == CAN_CS_STARTED) &&
        ((PduModeRequest == CANIF_OFFLINE) || (PduModeRequest == CANIF_TX_OFFLINE) ||
         (PduModeRequest == CANIF_ONLINE))) {
        pdu_mode = PduModeRequest;
        result = E_OK;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetPduMode(uint8 ControllerId, CanIf_PduModeType *PduModePtr) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        (PduModePtr != NULL_PTR)) {
        *PduModePtr = pdu_mode;
        result = E_OK;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            const CanIf_TxPduConfigType *route = &configuration->transmit[i];
            if (route->pdu == TxPduId) {
                if (pdu_mode != CANIF_ONLINE) {
                    (void)Det_ReportRuntimeError(60u, 0u, 0x49u, 70u);
                } else if (PduInfoPtr->SduLength > 8u) {
                    (void)Det_ReportRuntimeError(60u, 0u, 0x49u, 62u);
                    (void)Det_ReportRuntimeError(60u, 0u, 0x49u, 90u);
                } else if (PduInfoPtr->SduLength > route->maximum_length) {
                    (void)Det_ReportRuntimeError(60u, 0u, 0x49u, 90u);
                } else if ((controller_mode == CAN_CS_STARTED) && (outstanding[i] < UINT32_MAX)) {
                    Can_PduType pdu = {TxPduId, (uint8)PduInfoPtr->SduLength, route->can_id,
                                       PduInfoPtr->SduDataPtr};
                    if (Can_Write(configuration->transmit_hoh, &pdu) == E_OK) {
                        ++outstanding[i];
                        result = E_OK;
                    }
                }
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE void CanIf_TxConfirmation(PduIdType CanTxPduId) {
    SchM_Enter_CanIf_CANIF_STATE();
    if (configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            if ((configuration->transmit[i].pdu == CanTxPduId) && (outstanding[i] != 0u)) {
                if (pdu_mode == CANIF_ONLINE) {
                    --outstanding[i];
                    LSduR_CanIfTxConfirmation(CanTxPduId, E_OK);
                }
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
}
static CANIF_CODE void CanIf_CancelOutstanding(void) {
    uint16 i;
    for (i = 0u; i < configuration->transmit_count; ++i) {
        while (outstanding[i] != 0u) {
            const PduIdType id = configuration->transmit[i].pdu;
            --outstanding[i];
            LSduR_CanIfTxConfirmation(id, E_NOT_OK);
        }
    }
}
CANIF_CODE void CanIf_ControllerModeIndication(uint8 ControllerId,
                                               Can_ControllerStateType ControllerMode) {
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        ((ControllerMode == CAN_CS_STARTED) || (ControllerMode == CAN_CS_STOPPED) ||
         (ControllerMode == CAN_CS_SLEEP))) {
        controller_mode = ControllerMode;
        if (ControllerMode == CAN_CS_STOPPED) {
            pdu_mode = CANIF_TX_OFFLINE;
            CanIf_CancelOutstanding();
        } else if (ControllerMode == CAN_CS_SLEEP) {
            pdu_mode = CANIF_OFFLINE;
        }
        configuration->controller_mode(ControllerId, ControllerMode);
    }
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE void CanIf_ControllerBusOff(uint8 ControllerId) {
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller)) {
        Can_ControllerStateType actual_mode = CAN_CS_UNINIT;
        pdu_mode = CANIF_TX_OFFLINE;
        /* The host driver enters STOPPED on bus-off without a separate mode
         * indication. Converge only from its actual reported transition.
         */
        if ((Can_GetControllerMode(configuration->driver_controller, &actual_mode) == E_OK) &&
            (actual_mode == CAN_CS_STOPPED) && (controller_mode != CAN_CS_STOPPED)) {
            controller_mode = CAN_CS_STOPPED;
            CanIf_CancelOutstanding();
        }
        configuration->bus_off(ControllerId);
    }
    SchM_Exit_CanIf_CANIF_STATE();
}
static CANIF_CODE EcuStatus CanIf_Receive(const Can_HwType *Mailbox,
                                          const PduInfoType *PduInfoPtr) {
    EcuStatus result = ECU_ERR_FRAME_ID;
    if ((configuration != NULL_PTR) && (Mailbox != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) &&
        (Mailbox->ControllerId == configuration->controller) &&
        (Mailbox->Hoh == configuration->receive_hoh) && (Mailbox->CanId <= 0x7ffu) &&
        (controller_mode == CAN_CS_STARTED) &&
        ((pdu_mode == CANIF_ONLINE) || (pdu_mode == CANIF_TX_OFFLINE))) {
        uint16 i;
        for (i = 0u; i < configuration->receive_count; ++i) {
            const CanIf_RxPduConfigType *route = &configuration->receive[i];
            if (route->can_id == Mailbox->CanId) {
                if ((PduInfoPtr->SduLength < route->minimum_length) ||
                    (PduInfoPtr->SduLength > 8u)) {
                    (void)Det_ReportRuntimeError(60u, 0u, 20u, 61u);
                    result = ECU_ERR_FRAME_DLC;
                } else {
                    LSduR_CanIfRxIndication(route->pdu, PduInfoPtr);
                    result = ECU_OK;
                }
                break;
            }
        }
    }
    return result;
}
CANIF_CODE void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr) {
    SchM_Enter_CanIf_CANIF_STATE();
    (void)CanIf_Receive(Mailbox, PduInfoPtr);
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE EcuStatus CanIf_HostRxIndication(uint32 id, uint8 dlc, const uint8 data[8],
                                            uint64 now_ms) {
    uint8 bytes[8];
    uint8 i;
    PduInfoType info = {bytes, NULL_PTR, dlc};
    EcuStatus result = ECU_ERR_FRAME_DLC;
    SchM_Enter_CanIf_CANIF_STATE();
    (void)now_ms;
    if ((configuration != NULL_PTR) && (data != NULL_PTR) && (dlc <= 8u)) {
        const Can_HwType mailbox = {id, configuration->receive_hoh, configuration->controller};
        for (i = 0u; i < dlc; ++i) {
            bytes[i] = data[i];
        }
        result = CanIf_Receive(&mailbox, &info);
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
#define CANIF_STOP_SEC_CODE
#include "CanIf_MemMap.h"
