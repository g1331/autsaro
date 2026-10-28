/** @file Explicit CDD bus state manager for one controlled-host CAN channel.
 * Its own strict area protects module data only: CanIf and ComM are called
 * outside it. A controller transition is registered before CanIf is asked for it, so
 * a completion indicated from within that call finds it.
 */
#include "Ecu_HostBusSM.h"
#include "Can_HostLock.h"
#define ECU_HOSTBUSSM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
/* All CAN_HOST_AREA_BUSSM. Init keeps an in-flight publisher so that no second one
 * overtakes it. */
/** The configuration, or NULL when uninitialized. */
static const Ecu_HostBusSM_ConfigType *bussm_configuration
    ECU_HOSTBUSSM_VAR_CLEARED; /**< BUSSM area; Init replaces, DeInit clears borrowed config. */
static ComM_ModeType requested_mode ECU_HOSTBUSSM_VAR_CLEARED; /**< Last accepted request. */
static boolean transition_pending ECU_HOSTBUSSM_VAR_CLEARED;   /**< Controller transition. */
static uint32 revision ECU_HOSTBUSSM_VAR_CLEARED; /**< Transition events; never reset. */
/* Completions awaiting Ecu_HostBusSM_Publish; Init keeps a running publisher. */
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
static struct {
    boolean due;     /**< A completion is unpublished. */
    boolean running; /**< A publisher is running. */
} bussm_publication
    ECU_HOSTBUSSM_VAR_CLEARED; /**< Area-protected publication; Init keeps a running publisher. */
#define ECU_HOSTBUSSM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
#define ECU_HOSTBUSSM_START_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Enter(void) { Can_HostAreaEnter(CAN_HOST_AREA_BUSSM); }
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Exit(void) { Can_HostAreaExit(CAN_HOST_AREA_BUSSM); }
/* Publish the actual mode after a completion; a concurrent or reentrant completion is
 * published by the running publisher. Each round reads the actual mode, so a completion
 * that raced a later one cannot leave ComM with a stale mode. Repeated factual
 * publication lets a restarted ComM obtain the actual mode; ComM filters repeats. */
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Publish(void)
{
    boolean owner = FALSE;
    Ecu_HostBusSM_Enter();
    if (bussm_configuration != NULL_PTR) {
        bussm_publication.due = TRUE;
        if (!bussm_publication.running) {
            bussm_publication.running = TRUE;
            owner = TRUE;
        }
    }
    Ecu_HostBusSM_Exit();
    while (owner) {
        NetworkHandleType channel = 0u;
        ComM_ModeType next = COMM_NO_COMMUNICATION;
        Ecu_HostBusSM_Enter();
        if ((bussm_configuration != NULL_PTR) && bussm_publication.due) {
            bussm_publication.due = FALSE;
            channel = bussm_configuration->channel;
        } else {
            bussm_publication.running = FALSE;
            owner = FALSE;
        }
        Ecu_HostBusSM_Exit();
        /* An unreadable controller publishes nothing; its next completion does. */
        if (owner) {
            if (Ecu_HostBusSM_GetCurrentComMode(channel, &next) == E_OK) {
                ComM_BusSM_ModeIndication(channel, next);
            }
        }
    }
}
/* The configuration for Channel, or NULL; read under the area, used outside it. */
static ECU_HOSTBUSSM_CODE const Ecu_HostBusSM_ConfigType *Ecu_HostBusSM_Channel(
    NetworkHandleType Channel)
{
    const Ecu_HostBusSM_ConfigType *config;
    Ecu_HostBusSM_Enter();
    config = ((bussm_configuration != NULL_PTR) && (Channel == bussm_configuration->channel))
                 ? bussm_configuration
                 : NULL_PTR;
    Ecu_HostBusSM_Exit();
    return config;
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr)
{
    CanIf_PduModeType mode = CANIF_OFFLINE;
    Std_ReturnType known = E_NOT_OK;
    if (ConfigPtr != NULL_PTR) {
        known = CanIf_GetPduMode(ConfigPtr->controller, &mode); /* CanIf's controller. */
    }
    if (known == E_OK) {
        Ecu_HostBusSM_Enter();
        bussm_configuration = ConfigPtr;
        requested_mode = COMM_NO_COMMUNICATION;
        transition_pending = FALSE;
        Ecu_HostBusSM_Exit();
    }
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_DeInit(void)
{
    Ecu_HostBusSM_Enter();
    bussm_configuration = NULL_PTR;
    transition_pending = FALSE;
    Ecu_HostBusSM_Exit();
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel,
                                                                  ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_ConfigType *config = Ecu_HostBusSM_Channel(Channel);
    if ((config != NULL_PTR) && (ComMode != NULL_PTR)) {
        Can_ControllerStateType controller = CAN_CS_UNINIT;
        CanIf_PduModeType pdu = CANIF_OFFLINE;
        boolean pending;
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(config->controller, &controller);
        const Std_ReturnType pdu_result = CanIf_GetPduMode(config->controller, &pdu);
        Ecu_HostBusSM_Enter();
        pending = transition_pending;
        Ecu_HostBusSM_Exit();
        if ((controller_result == E_OK) && (pdu_result == E_OK)) {
            if ((controller != CAN_CS_STARTED) || (pdu == CANIF_OFFLINE) || pending) {
                *ComMode = COMM_NO_COMMUNICATION;
            } else if (pdu == CANIF_TX_OFFLINE) {
                *ComMode = COMM_SILENT_COMMUNICATION;
            } else {
                *ComMode = COMM_FULL_COMMUNICATION;
            }
            result = E_OK;
        }
    }
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel,
                                                          ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_ConfigType *config = Ecu_HostBusSM_Channel(Channel);
    Can_ControllerStateType controller = CAN_CS_UNINIT;
    Std_ReturnType controller_result = E_NOT_OK;
    if ((config != NULL_PTR) && (ComMode <= COMM_FULL_COMMUNICATION)) {
        controller_result = CanIf_GetControllerMode(config->controller, &controller);
    }
    if (controller_result == E_OK) {
        if (controller == CAN_CS_STARTED) {
            const CanIf_PduModeType pdu =
                (ComMode == COMM_FULL_COMMUNICATION)
                    ? CANIF_ONLINE
                    : ((ComMode == COMM_SILENT_COMMUNICATION) ? CANIF_TX_OFFLINE : CANIF_OFFLINE);
            result = CanIf_SetPduMode(config->controller, pdu);
        } else if (ComMode == COMM_NO_COMMUNICATION) {
            /* STOPPED/SLEEP already reject RX/TX; never fabricate ONLINE. */
            result = E_OK;
        } else {
            /* A direction action cannot start a stopped controller. */
        }
    }
    return result;
}
/* Register a controller transition, then ask CanIf. A refusal restores whether an older
 * transition is pending, unless a newer registration or a completion came meanwhile;
 * the refused mode stays requested, and ComM requests it again. */
static ECU_HOSTBUSSM_CODE Std_ReturnType
Ecu_HostBusSM_Transition(const Ecu_HostBusSM_ConfigType *config, ComM_ModeType ComMode)
{
    Std_ReturnType result;
    uint32 registration;
    boolean previous_pending;
    Ecu_HostBusSM_Enter();
    previous_pending = transition_pending;
    requested_mode = ComMode;
    transition_pending = TRUE;
    ++revision;
    registration = revision;
    Ecu_HostBusSM_Exit();
    result = CanIf_SetControllerMode(
        config->controller, (ComMode == COMM_NO_COMMUNICATION) ? CAN_CS_STOPPED : CAN_CS_STARTED);
    if (result != E_OK) {
        Ecu_HostBusSM_Enter();
        if (registration == revision) {
            transition_pending = previous_pending;
        }
        Ecu_HostBusSM_Exit();
    }
    return result;
}
/* Decide on the actual controller state: apply directions to a controller already in
 * the right state, otherwise start a controller transition. */
static ECU_HOSTBUSSM_CODE Std_ReturnType
Ecu_HostBusSM_Request(const Ecu_HostBusSM_ConfigType *config,
                      ComM_ModeType ComMode,
                      Can_ControllerStateType controller)
{
    Std_ReturnType result;
    boolean pending;
    boolean repeated;
    Ecu_HostBusSM_Enter();
    pending = transition_pending;
    repeated = transition_pending && (requested_mode == ComMode);
    Ecu_HostBusSM_Exit();
    if (repeated) {
        result = E_OK; /* Accepted transition still awaits the actual driver poll. */
    } else if (((ComMode == COMM_NO_COMMUNICATION) && (controller != CAN_CS_STARTED)) ||
               ((ComMode != COMM_NO_COMMUNICATION) && (controller == CAN_CS_STARTED) && !pending)) {
        result = Ecu_HostBusSM_ApplyMode(config->channel, ComMode);
        if (result == E_OK) {
            Ecu_HostBusSM_Enter();
            requested_mode = ComMode;
            Ecu_HostBusSM_Exit();
            Ecu_HostBusSM_Publish();
        }
    } else {
        result = Ecu_HostBusSM_Transition(config, ComMode);
    }
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel,
                                                               ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_ConfigType *config = Ecu_HostBusSM_Channel(Channel);
    if ((config != NULL_PTR) && (ComMode <= COMM_FULL_COMMUNICATION)) {
        Can_ControllerStateType controller = CAN_CS_UNINIT;
        Can_ErrorStateType error = CAN_ERRORSTATE_BUSOFF;
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(config->controller, &controller);
        const Std_ReturnType error_result =
            CanIf_GetControllerErrorState(config->controller, &error);
        if ((controller_result == E_OK) && (error_result == E_OK) &&
            ((ComMode == COMM_NO_COMMUNICATION) || (error != CAN_ERRORSTATE_BUSOFF))) {
            result = Ecu_HostBusSM_Request(config, ComMode, controller);
        }
    }
    return result;
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerModeIndication(
    uint8 ControllerId, Can_ControllerStateType ControllerMode)
{
    const Ecu_HostBusSM_ConfigType *config = NULL_PTR;
    ComM_ModeType requested = COMM_NO_COMMUNICATION;
    Ecu_HostBusSM_Enter();
    if ((bussm_configuration != NULL_PTR) && (ControllerId == bussm_configuration->controller)) {
        config = bussm_configuration;
        transition_pending = FALSE;
        ++revision;
        requested = requested_mode;
    }
    Ecu_HostBusSM_Exit();
    if (config != NULL_PTR) {
        if (ControllerMode == CAN_CS_STARTED) {
            if (Ecu_HostBusSM_ApplyMode(config->channel, requested) == E_OK) {
                Ecu_HostBusSM_Publish();
            }
        } else if ((ControllerMode == CAN_CS_STOPPED) || (ControllerMode == CAN_CS_SLEEP)) {
            Ecu_HostBusSM_Publish();
        } else {
            /* No completed communication mode for UNINIT. */
        }
    }
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId)
{
    const Ecu_HostBusSM_ConfigType *config = NULL_PTR;
    Can_ControllerStateType actual = CAN_CS_UNINIT;
    Std_ReturnType result = E_NOT_OK;
    Ecu_HostBusSM_Enter();
    if ((bussm_configuration != NULL_PTR) && (ControllerId == bussm_configuration->controller)) {
        config = bussm_configuration;
    }
    Ecu_HostBusSM_Exit();
    if (config != NULL_PTR) {
        result = CanIf_GetControllerMode(ControllerId, &actual);
    }
    if (result == E_OK) {
        if (actual == CAN_CS_STOPPED) {
            Ecu_HostBusSM_Enter();
            transition_pending = FALSE;
            ++revision;
            Ecu_HostBusSM_Exit();
            Ecu_HostBusSM_Publish();
        } else if (actual == CAN_CS_STARTED) {
            ComM_ModeType requested;
            Ecu_HostBusSM_Enter();
            requested = requested_mode;
            Ecu_HostBusSM_Exit();
            /* Delivered after a restart: CanIf already went TX_OFFLINE, so
             * restore the requested mode as a bus-off recovery would. A pending
             * transition applies the same mode again on its completion. */
            (void)Ecu_HostBusSM_ApplyMode(config->channel, requested);
        } else {
            /* No controller state to restore. */
        }
    }
}
#define ECU_HOSTBUSSM_STOP_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
