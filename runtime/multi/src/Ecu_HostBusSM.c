#include "Ecu_HostBusSM.h"
#include "Can.h"
#include "Can_HostLock.h"
#define ECU_HOSTBUSSM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
static const Ecu_HostBusSM_ConfigType *configuration ECU_HOSTBUSSM_VAR_CLEARED;
static ComM_ModeType requested_mode ECU_HOSTBUSSM_VAR_CLEARED;
static boolean transition_pending ECU_HOSTBUSSM_VAR_CLEARED;
#define ECU_HOSTBUSSM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
#define ECU_HOSTBUSSM_START_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Publish(ComM_ModeType mode) {
    /* ComM filters unchanged indications. Repeated factual publication also
     * permits a restarted ComM instance to obtain the actual completed mode.
     */
    ComM_BusSM_ModeIndication(configuration->channel, mode);
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr) {
    Can_Lock();
    if ((ConfigPtr != NULL_PTR) && (ConfigPtr->controller == 0u)) {
        configuration = ConfigPtr;
        requested_mode = COMM_NO_COMMUNICATION;
        transition_pending = FALSE;
    }
    Can_Unlock();
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_DeInit(void) {
    Can_Lock();
    configuration = NULL_PTR;
    transition_pending = FALSE;
    Can_Unlock();
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel,
                                                                  ComM_ModeType *ComMode) {
    Std_ReturnType result = E_NOT_OK;
    Can_ControllerStateType controller = CAN_CS_UNINIT;
    CanIf_PduModeType pdu = CANIF_OFFLINE;
    Can_Lock();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel) &&
        (ComMode != NULL_PTR)) {
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(configuration->controller, &controller);
        const Std_ReturnType pdu_result = CanIf_GetPduMode(configuration->controller, &pdu);
        if ((controller_result == E_OK) && (pdu_result == E_OK)) {
            if ((controller != CAN_CS_STARTED) || (pdu == CANIF_OFFLINE) ||
                (transition_pending == TRUE)) {
                *ComMode = COMM_NO_COMMUNICATION;
            } else if (pdu == CANIF_TX_OFFLINE) {
                *ComMode = COMM_SILENT_COMMUNICATION;
            } else {
                *ComMode = COMM_FULL_COMMUNICATION;
            }
            result = E_OK;
        }
    }
    Can_Unlock();
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel,
                                                          ComM_ModeType ComMode) {
    Std_ReturnType result = E_NOT_OK;
    Can_ControllerStateType controller = CAN_CS_UNINIT;
    Can_Lock();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel) &&
        (ComMode <= COMM_FULL_COMMUNICATION)) {
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(configuration->controller, &controller);
        if (controller_result == E_OK) {
            if (controller == CAN_CS_STARTED) {
                const CanIf_PduModeType pdu =
                    (ComMode == COMM_FULL_COMMUNICATION)
                        ? CANIF_ONLINE
                        : ((ComMode == COMM_SILENT_COMMUNICATION) ? CANIF_TX_OFFLINE
                                                                  : CANIF_OFFLINE);
                result = CanIf_SetPduMode(configuration->controller, pdu);
            } else if (ComMode == COMM_NO_COMMUNICATION) {
                /* STOPPED/SLEEP already reject RX/TX; never fabricate ONLINE. */
                result = E_OK;
            } else {
                /* A direction action cannot start a stopped controller. */
            }
        }
    }
    Can_Unlock();
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel,
                                                               ComM_ModeType ComMode) {
    Std_ReturnType result = E_NOT_OK;
    Can_ControllerStateType controller = CAN_CS_UNINIT;
    Can_ErrorStateType error = CAN_ERRORSTATE_BUSOFF;
    Can_Lock();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel) &&
        (ComMode <= COMM_FULL_COMMUNICATION)) {
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(configuration->controller, &controller);
        const Std_ReturnType error_result =
            Can_GetControllerErrorState(configuration->controller, &error);
        if ((controller_result == E_OK) && (error_result == E_OK) &&
            ((ComMode == COMM_NO_COMMUNICATION) || (error != CAN_ERRORSTATE_BUSOFF))) {
            if ((transition_pending == TRUE) && (requested_mode == ComMode)) {
                result = E_OK; /* Accepted transition still awaits the actual driver poll. */
            } else if (((ComMode == COMM_NO_COMMUNICATION) && (controller != CAN_CS_STARTED)) ||
                       ((ComMode != COMM_NO_COMMUNICATION) && (controller == CAN_CS_STARTED) &&
                        (transition_pending == FALSE))) {
                result = Ecu_HostBusSM_ApplyMode(Channel, ComMode);
                if (result == E_OK) {
                    requested_mode = ComMode;
                    Ecu_HostBusSM_Publish(ComMode);
                }
            } else {
                const Can_ControllerStateType next =
                    (ComMode == COMM_NO_COMMUNICATION) ? CAN_CS_STOPPED : CAN_CS_STARTED;
                result = CanIf_SetControllerMode(configuration->controller, next);
                if (result == E_OK) {
                    requested_mode = ComMode;
                    transition_pending = TRUE;
                }
            }
        }
    }
    Can_Unlock();
    return result;
}
ECU_HOSTBUSSM_CODE void
Ecu_HostBusSM_ControllerModeIndication(uint8 ControllerId, Can_ControllerStateType ControllerMode) {
    Can_Lock();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller)) {
        Std_ReturnType result = E_NOT_OK;
        transition_pending = FALSE;
        if (ControllerMode == CAN_CS_STARTED) {
            result = Ecu_HostBusSM_ApplyMode(configuration->channel, requested_mode);
            if (result == E_OK) {
                Ecu_HostBusSM_Publish(requested_mode);
            }
        } else if ((ControllerMode == CAN_CS_STOPPED) || (ControllerMode == CAN_CS_SLEEP)) {
            Ecu_HostBusSM_Publish(COMM_NO_COMMUNICATION);
        } else {
            /* No completed communication mode for UNINIT. */
        }
    }
    Can_Unlock();
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId) {
    Can_Lock();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller)) {
        Can_ControllerStateType actual = CAN_CS_UNINIT;
        const Std_ReturnType result = CanIf_GetControllerMode(ControllerId, &actual);
        if ((result == E_OK) && (actual == CAN_CS_STOPPED)) {
            transition_pending = FALSE;
            Ecu_HostBusSM_Publish(COMM_NO_COMMUNICATION);
        }
    }
    Can_Unlock();
}
#define ECU_HOSTBUSSM_STOP_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
