/** @file Explicit CDD bus state manager for controlled-host CAN channels.
 * Its own strict area protects module data only: CanIf and ComM are called
 * outside it. A controller transition is registered before CanIf is asked for it, so
 * a completion indicated from within that call finds it.
 */
#include "Ecu_HostBusSM.h"
#include "Can_HostLock.h"
#include "Ecu_ProfileConfig.h"
#define ECU_HOSTBUSSM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
/* All CAN_HOST_AREA_BUSSM. Init keeps an in-flight publisher so that no second one
 * overtakes it. */
/** The configuration, or NULL when uninitialized. */
static const Ecu_HostBusSM_ConfigType *bussm_configuration
    ECU_HOSTBUSSM_VAR_CLEARED; /**< BUSSM area; Init replaces, DeInit clears borrowed config. */
typedef struct {
    ComM_ModeType requested; /**< Last accepted request. */
    boolean pending;         /**< Controller transition. */
    uint32 revision;         /**< Transition events; never reset. */
    boolean due;             /**< A completion is unpublished. */
    boolean publishing;      /**< A publisher is running. */
} Ecu_HostBusSM_StateType;
static Ecu_HostBusSM_StateType
    bussm_channels[COMM_CHANNEL_CAPACITY] ECU_HOSTBUSSM_VAR_CLEARED; /**< BUSSM area;
                                                                        configuration-order channel
                                                                        states. */
#define ECU_HOSTBUSSM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ecu_HostBusSM_MemMap.h"
#define ECU_HOSTBUSSM_START_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Enter(void) { Can_HostAreaEnter(CAN_HOST_AREA_BUSSM); }
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Exit(void) { Can_HostAreaExit(CAN_HOST_AREA_BUSSM); }
/* A completion queues a fresh observation for this channel. A concurrent
 * or reentrant request is processed by the publisher that already owns it. */
static ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Publish(Ecu_HostBusSM_StateType *state,
                                                     NetworkHandleType channel,
                                                     boolean completed)
{
    boolean owner = FALSE;
    Ecu_HostBusSM_Enter();
    if (bussm_configuration != NULL_PTR) {
        if (completed) {
            state->due = TRUE;
        }
        if (state->due && !state->publishing) {
            state->publishing = TRUE;
            owner = TRUE;
        }
    }
    Ecu_HostBusSM_Exit();
    while (owner) {
        ComM_ModeType next = COMM_NO_COMMUNICATION;
        Ecu_HostBusSM_Enter();
        if ((bussm_configuration != NULL_PTR) && state->due) {
            state->due = FALSE;
        } else {
            state->publishing = FALSE;
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
/* A call borrows its immutable mapping and the corresponding preallocated state. */
typedef struct {
    const Ecu_HostBusSM_ChannelConfigType *config; /**< NULL when no channel matches. */
    Ecu_HostBusSM_StateType *state;                /**< Mutable state under CAN_HOST_AREA_BUSSM. */
} Ecu_HostBusSM_BindingType;

/* Caller holds the BusSM area. */
static ECU_HOSTBUSSM_CODE Ecu_HostBusSM_BindingType Ecu_HostBusSM_Binding(uint8 id,
                                                                          boolean controller)
{
    Ecu_HostBusSM_BindingType binding = {NULL_PTR, NULL_PTR};
    if (bussm_configuration != NULL_PTR) {
        for (uint16 i = 0u; (i < COMM_CHANNEL_CAPACITY) && (i < bussm_configuration->channel_count);
             ++i) {
            const Ecu_HostBusSM_ChannelConfigType *config = &bussm_configuration->channels[i];
            if ((controller ? config->controller : config->channel) == id) {
                binding.config = config;
                binding.state = &bussm_channels[i];
                break;
            }
        }
    }
    return binding;
}
static ECU_HOSTBUSSM_CODE Ecu_HostBusSM_BindingType Ecu_HostBusSM_Channel(NetworkHandleType channel)
{
    Ecu_HostBusSM_Enter();
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Binding(channel, FALSE);
    Ecu_HostBusSM_Exit();
    return binding;
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_QueueModePublication(NetworkHandleType Channel)
{
    Ecu_HostBusSM_Enter();
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Binding(Channel, FALSE);
    if (binding.state != NULL_PTR) {
        binding.state->due = TRUE;
    }
    Ecu_HostBusSM_Exit();
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_PublishPendingMode(void)
{
    for (uint16 index = 0u; index < COMM_CHANNEL_CAPACITY; ++index) {
        NetworkHandleType channel = 0u;
        Ecu_HostBusSM_Enter();
        const boolean configured =
            (bussm_configuration != NULL_PTR) && (index < bussm_configuration->channel_count);
        if (configured) {
            channel = bussm_configuration->channels[index].channel;
        }
        Ecu_HostBusSM_Exit();
        if (configured) {
            Ecu_HostBusSM_Publish(&bussm_channels[index], channel, FALSE);
        }
    }
}
static ECU_HOSTBUSSM_CODE boolean Ecu_HostBusSM_Valid(const Ecu_HostBusSM_ConfigType *config)
{
    boolean valid = (config != NULL_PTR) && (config->channels != NULL_PTR) &&
                    (config->channel_count != 0u) &&
                    (config->channel_count <= COMM_CHANNEL_CAPACITY);
    if (valid) {
        for (uint16 i = 0u; (i < config->channel_count) && valid; ++i) {
            CanIf_PduModeType mode = CANIF_OFFLINE;
            const Ecu_HostBusSM_ChannelConfigType *channel = &config->channels[i];
            valid = CanIf_GetPduMode(channel->controller, &mode) == E_OK;
            for (uint16 j = 0u; (j < i) && valid; ++j) {
                valid = (channel->channel != config->channels[j].channel) &&
                        (channel->controller != config->channels[j].controller);
            }
        }
    }
    return valid;
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr)
{
    if (Ecu_HostBusSM_Valid(ConfigPtr)) {
        Ecu_HostBusSM_Enter();
        bussm_configuration = ConfigPtr;
        for (uint16 i = 0u; (i < COMM_CHANNEL_CAPACITY) && (i < ConfigPtr->channel_count); ++i) {
            bussm_channels[i].requested = COMM_NO_COMMUNICATION;
            bussm_channels[i].pending = FALSE;
            ++bussm_channels[i].revision;
        }
        Ecu_HostBusSM_Exit();
    }
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_DeInit(void)
{
    Ecu_HostBusSM_Enter();
    bussm_configuration = NULL_PTR;
    for (uint16 i = 0u; i < COMM_CHANNEL_CAPACITY; ++i) {
        bussm_channels[i].pending = FALSE;
        bussm_channels[i].due = FALSE;
        ++bussm_channels[i].revision;
    }
    Ecu_HostBusSM_Exit();
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel,
                                                                  ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Channel(Channel);
    const Ecu_HostBusSM_ChannelConfigType *config = binding.config;
    if ((config != NULL_PTR) && (ComMode != NULL_PTR)) {
        Can_ControllerStateType controller = CAN_CS_UNINIT;
        CanIf_PduModeType pdu = CANIF_OFFLINE;
        boolean pending;
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(config->controller, &controller);
        const Std_ReturnType pdu_result = CanIf_GetPduMode(config->controller, &pdu);
        Ecu_HostBusSM_Enter();
        pending = binding.state->pending;
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
static ECU_HOSTBUSSM_CODE Std_ReturnType
Ecu_HostBusSM_Apply(const Ecu_HostBusSM_ChannelConfigType *config, ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
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
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel,
                                                          ComM_ModeType ComMode)
{
    return Ecu_HostBusSM_Apply(Ecu_HostBusSM_Channel(Channel).config, ComMode);
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ConfirmMode(NetworkHandleType Channel,
                                                            ComM_ModeType ComMode)
{
    ComM_ModeType actual = COMM_NO_COMMUNICATION;
    const Std_ReturnType observed = Ecu_HostBusSM_GetCurrentComMode(Channel, &actual);
    return ((observed == E_OK) && (actual == ComMode)) ? E_OK : E_NOT_OK;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ConfirmControllerMode(NetworkHandleType Channel,
                                                                      uint8 ControllerId,
                                                                      ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_ChannelConfigType *config = Ecu_HostBusSM_Channel(Channel).config;
    if ((config != NULL_PTR) && (config->controller == ControllerId)) {
        result = Ecu_HostBusSM_ConfirmMode(Channel, ComMode);
    }
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_HostOffline(uint8 ControllerId)
{
    Ecu_HostBusSM_Enter();
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Binding(ControllerId, TRUE);
    Ecu_HostBusSM_Exit();
    Std_ReturnType result = E_NOT_OK;
    if (binding.config != NULL_PTR) {
        result = CanIf_SetPduMode(ControllerId, CANIF_OFFLINE);
        if (result == E_OK) {
            Ecu_HostBusSM_Publish(binding.state, binding.config->channel, TRUE);
        }
    }
    return result;
}
/* Register a controller transition, then ask CanIf. A refusal restores whether an older
 * transition is pending, unless a newer registration or a completion came meanwhile;
 * the refused mode stays requested, and ComM requests it again. */
static ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_Transition(Ecu_HostBusSM_BindingType binding,
                                                                  ComM_ModeType ComMode)
{
    Std_ReturnType result;
    uint32 registration;
    boolean previous_pending;
    Ecu_HostBusSM_Enter();
    previous_pending = binding.state->pending;
    binding.state->requested = ComMode;
    binding.state->pending = TRUE;
    ++binding.state->revision;
    registration = binding.state->revision;
    Ecu_HostBusSM_Exit();
    result = CanIf_SetControllerMode(binding.config->controller,
                                     (ComMode == COMM_NO_COMMUNICATION) ? CAN_CS_STOPPED
                                                                        : CAN_CS_STARTED);
    if (result != E_OK) {
        Ecu_HostBusSM_Enter();
        if (registration == binding.state->revision) {
            binding.state->pending = previous_pending;
        }
        Ecu_HostBusSM_Exit();
    }
    return result;
}
/* Decide on the actual controller state: apply directions to a controller already in
 * the right state, otherwise start a controller transition. */
static ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_Request(Ecu_HostBusSM_BindingType binding,
                                                               ComM_ModeType ComMode,
                                                               Can_ControllerStateType controller)
{
    Std_ReturnType result;
    boolean pending;
    boolean repeated;
    Ecu_HostBusSM_Enter();
    pending = binding.state->pending;
    repeated = pending && (binding.state->requested == ComMode);
    Ecu_HostBusSM_Exit();
    if (repeated) {
        result = E_OK; /* Accepted transition still awaits the actual driver poll. */
    } else if (((ComMode == COMM_NO_COMMUNICATION) && (controller != CAN_CS_STARTED)) ||
               ((ComMode != COMM_NO_COMMUNICATION) && (controller == CAN_CS_STARTED) && !pending)) {
        result = Ecu_HostBusSM_ApplyMode(binding.config->channel, ComMode);
        if (result == E_OK) {
            Ecu_HostBusSM_Enter();
            binding.state->requested = ComMode;
            Ecu_HostBusSM_Exit();
            Ecu_HostBusSM_Publish(binding.state, binding.config->channel, TRUE);
        }
    } else {
        result = Ecu_HostBusSM_Transition(binding, ComMode);
    }
    return result;
}
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel,
                                                               ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Channel(Channel);
    const Ecu_HostBusSM_ChannelConfigType *config = binding.config;
    if ((config != NULL_PTR) && (ComMode <= COMM_FULL_COMMUNICATION)) {
        Can_ControllerStateType controller = CAN_CS_UNINIT;
        Can_ErrorStateType error = CAN_ERRORSTATE_BUSOFF;
        const Std_ReturnType controller_result =
            CanIf_GetControllerMode(config->controller, &controller);
        const Std_ReturnType error_result =
            CanIf_GetControllerErrorState(config->controller, &error);
        if ((controller_result == E_OK) && (error_result == E_OK) &&
            ((ComMode == COMM_NO_COMMUNICATION) || (error != CAN_ERRORSTATE_BUSOFF))) {
            result = Ecu_HostBusSM_Request(binding, ComMode, controller);
        }
    }
    return result;
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerModeIndication(
    uint8 ControllerId, Can_ControllerStateType ControllerMode)
{
    ComM_ModeType requested = COMM_NO_COMMUNICATION;
    Ecu_HostBusSM_Enter();
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Binding(ControllerId, TRUE);
    const Ecu_HostBusSM_ChannelConfigType *config = binding.config;
    if (config != NULL_PTR) {
        binding.state->pending = FALSE;
        ++binding.state->revision;
        requested = binding.state->requested;
    }
    Ecu_HostBusSM_Exit();
    if (config != NULL_PTR) {
        if (ControllerMode == CAN_CS_STARTED) {
            if (Ecu_HostBusSM_ApplyMode(config->channel, requested) == E_OK) {
                Ecu_HostBusSM_Publish(binding.state, binding.config->channel, TRUE);
            }
        } else if ((ControllerMode == CAN_CS_STOPPED) || (ControllerMode == CAN_CS_SLEEP)) {
            Ecu_HostBusSM_Publish(binding.state, binding.config->channel, TRUE);
        } else {
            /* No completed communication mode for UNINIT. */
        }
    }
}
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId)
{
    Can_ControllerStateType actual = CAN_CS_UNINIT;
    Std_ReturnType result = E_NOT_OK;
    Ecu_HostBusSM_Enter();
    const Ecu_HostBusSM_BindingType binding = Ecu_HostBusSM_Binding(ControllerId, TRUE);
    const Ecu_HostBusSM_ChannelConfigType *config = binding.config;
    Ecu_HostBusSM_Exit();
    if (config != NULL_PTR) {
        result = CanIf_GetControllerMode(ControllerId, &actual);
    }
    if (result == E_OK) {
        if (actual == CAN_CS_STOPPED) {
            Ecu_HostBusSM_Enter();
            binding.state->pending = FALSE;
            ++binding.state->revision;
            Ecu_HostBusSM_Exit();
            Ecu_HostBusSM_Publish(binding.state, binding.config->channel, TRUE);
        } else if (actual == CAN_CS_STARTED) {
            ComM_ModeType requested;
            Ecu_HostBusSM_Enter();
            requested = binding.state->requested;
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
