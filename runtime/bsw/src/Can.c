/** @file Standard Can driver for configured CAN controllers on the controlled host. */
#include "Can_Internal.h"
#include "Ecu_ProfileConfig.h"
#include "Det.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(DET_AR_RELEASE_MAJOR_VERSION) || !defined(DET_AR_RELEASE_MINOR_VERSION)
#error "Can.c -> Det.h: missing AUTOSAR release"
#elif (DET_AR_RELEASE_MAJOR_VERSION != 4U) || (DET_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can.c -> Det.h: incompatible AUTOSAR release"
#endif
#include "CanIf.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CANIF_AR_RELEASE_MAJOR_VERSION) || !defined(CANIF_AR_RELEASE_MINOR_VERSION)
#error "Can.c -> CanIf.h: missing AUTOSAR release"
#elif (CANIF_AR_RELEASE_MAJOR_VERSION != 4U) || (CANIF_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can.c -> CanIf.h: incompatible AUTOSAR release"
#endif
#include "SchM_Can.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "Can.c -> SchM_Can.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can.c -> SchM_Can.h: incompatible AUTOSAR release"
#endif
#include "Ecu_Execution.h"

/* Static controller options are supplied by the validated integrated configuration. */
/* Driver IDs and callback IDs are distinct configured namespaces. */
#ifndef CAN_CONTROLLER_CAPACITY
#define CAN_CONTROLLER_CAPACITY 1u
#endif
#ifndef CAN_HOH_CAPACITY
#define CAN_HOH_CAPACITY 2u
#endif
#ifndef CAN_CONTROLLER_ID
#define CAN_CONTROLLER_ID 0u
#endif
#ifndef CAN_CANIF_CONTROLLER_ID
#define CAN_CANIF_CONTROLLER_ID 0u
#endif
#ifndef CAN_TX_HOH
#define CAN_TX_HOH 0u
#endif
#ifndef CAN_RX_ID_TYPE
#define CAN_RX_ID_TYPE CAN_HOH_ID_STANDARD
#endif
#ifndef CAN_TX_ID_TYPE
#define CAN_TX_ID_TYPE CAN_HOH_ID_STANDARD
#endif
#ifndef CAN_RX_HOH
#define CAN_RX_HOH 0u
#endif
#ifndef CAN_RX_POLLING
#define CAN_RX_POLLING STD_OFF
#endif
#ifndef CAN_HOST_QUEUED_COMPLETION
#define CAN_HOST_QUEUED_COMPLETION STD_OFF
#endif
#ifndef CAN_TX_POLLING
#define CAN_TX_POLLING STD_OFF
#endif
#ifndef CAN_BUSOFF_POLLING
#define CAN_BUSOFF_POLLING STD_OFF
#endif

#define CAN_INDICATION_MODE   0u
#define CAN_INDICATION_BUSOFF 1u

#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

/* Storage belongs to CAN_STATE, including configuration and every controller slot. */
static uint8 can_initialized CAN_VAR_CLEARED;  /**< Init installs; DeInit clears. */
static uint64 driver_lifetime CAN_VAR_CLEARED; /**< Successful Init count. */
static const Can_ConfigType *can_configuration CAN_VAR_CLEARED; /**< Borrowed immutable setup. */

typedef struct {
    uint8 controller;                    /**< Driver identifier; installed by Init. */
    uint8 can_if;                        /**< Callback identifier; installed by Init. */
    uint32 fd_tx_bitrate;                /**< FD Tx bitrate; zero disables FD; set by Init. */
    boolean fd_tx_brs;                   /**< FD bitrate switch; set by Init. */
    CanMode can_controller_mode;         /**< Controller state; Init: STOPPED. */
    uint8 can_bus_off;                   /**< In bus-off; Init resets. */
    uint8 busoff_notification_pending;   /**< Bus-off for CanIf; Init resets. */
    uint8 tx_pending;                    /**< Admitted, not yet output. */
    uint8 tx_in_flight;                  /**< Host output running. */
    uint8 tx_cancelled;                  /**< Output gets no confirmation. */
    uint8 tx_confirmation_pending;       /**< Confirmation for CanIf. */
    uint8 tx_confirming;                 /**< CanIf_TxConfirmation running. */
    uint32 tx_id;                        /**< Encoded identifier. */
    uint8 tx_length;                     /**< Wire length, 0 to 64 bytes. */
    uint8 tx_payload[CAN_FD_MAX_LENGTH]; /**< tx_length bytes valid. */
    PduIdType tx_handle;                 /**< swPduHandle of the frame. */
    PduIdType tx_confirmation_handle;    /**< swPduHandle to confirm. */
    uint64 host_tx_sequence;             /**< Token of the last admitted frame. */
    uint64 host_tx_cancelled_through;    /**< Tokens up to here never complete. */
    uint64 host_tx_completed_through;    /**< Tokens up to here have completed. */
    uint32 interrupt_disable_count;      /**< Disable nesting; Init resets. */
    Can_ControllerStateType pending_controller_mode; /**< Mode to indicate. */
    uint8 mode_notification_pending;                 /**< Mode not yet sent. */
    uint8 indication_processing[2];                  /**< Kind in delivery. */
    uint8 rx_pending;                                /**< Frame buffered; Init resets. */
    uint8 rx_processing;                             /**< CanIf_RxIndication running. */
    uint32 rx_id;                                    /**< Encoded identifier. */
    uint8 rx_length;                                 /**< Wire length, 0 to 64 bytes. */
    uint8 rx_payload[CAN_FD_MAX_LENGTH];             /**< rx_length bytes valid. */
    uint8 rx_overrun_pending;                        /**< CAN_E_DATALOST to report; Init resets. */
} Can_ControllerRuntimeType;

/* In-flight flags are cleared only by the context that set them. Callback return
 * checks the captured lifetime before it changes any newly initialized state.
 * Queued sequence numbers survive initialization and are compared per controller. */
static Can_ControllerRuntimeType
    controllers[CAN_CONTROLLER_CAPACITY] CAN_VAR_CLEARED; /**< One reception buffer, transmit slot
                                                             and mode machine per controller. */
#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Can.c -> Can_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can.c -> Can_MemMap.h: incompatible AUTOSAR release"
#endif

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

static CAN_CODE void Can_CancelHostTransmit(Can_ControllerRuntimeType *state);
static CAN_CODE EcuStatus Can_AdmitTransmit(Can_ControllerRuntimeType *state,
                                            const Can_HohConfigType *hoh,
                                            const Can_PduType *pdu,
                                            const uint8 *data);
static CAN_CODE void Can_ClearPolledRx(Can_ControllerRuntimeType *state);
static CAN_CODE EcuStatus Can_DeliverImmediateRx(Can_ControllerRuntimeType *state);
static CAN_CODE void Can_ReadImmediateRx(Can_ControllerRuntimeType *state);
static CAN_CODE void Can_ConfirmImmediateTx(Can_ControllerRuntimeType *state);
static CAN_CODE void Can_RecordBusOff(Can_ControllerRuntimeType *state);
static CAN_CODE void Can_PublishImmediateBusOff(Can_ControllerRuntimeType *state);
static CAN_CODE EcuStatus Can_DeliverRx(Can_ControllerRuntimeType *state);
/* Scheduled-function bodies, also run on direct (non-polled) events (SWS_BSW_00133). */
static CAN_CODE void Can_ConfirmTx(Can_ControllerRuntimeType *state);
static CAN_CODE void Can_ReadRx(Can_ControllerRuntimeType *state);

static CAN_CODE uint16 Can_ControllerCount(const Can_ConfigType *config)
{
    return (config->controller_count == 0u) ? 1u : config->controller_count;
}

/* Called under CAN_STATE. Controller identifiers come from the generated table. */
static CAN_CODE Can_ControllerRuntimeType *Can_Controller(uint8 controller)
{
    Can_ControllerRuntimeType *state = NULL_PTR;
    if (can_initialized != 0u) {
        for (uint16 i = 0u;
             (i < CAN_CONTROLLER_CAPACITY) && (i < Can_ControllerCount(can_configuration));
             ++i) {
            if (controllers[i].controller == controller) {
                state = &controllers[i];
                break;
            }
        }
    }
    return state;
}

static CAN_CODE void Can_ResetController(Can_ControllerRuntimeType *state)
{
    Can_CancelHostTransmit(state);
    state->can_controller_mode = CAN_STOPPED;
    state->can_bus_off = 0u;
    state->busoff_notification_pending = 0u;
    state->tx_pending = 0u;
    state->tx_cancelled = 1u;
    state->tx_confirmation_pending = 0u;
    state->rx_pending = 0u;
    state->rx_overrun_pending = 0u;
    state->interrupt_disable_count = 0u;
    state->mode_notification_pending = 0u;
}

static CAN_CODE void Can_InstallController(Can_ControllerRuntimeType *state,
                                           const Can_ControllerConfigType *config)
{
    Can_ResetController(state);
    state->controller = config->controller;
    state->can_if = config->can_if;
    state->fd_tx_brs = (config->data_bitrate != 0u) && config->brs;
    state->fd_tx_bitrate = ((config->data_bitrate != 0u) && (!state->fd_tx_brs))
                               ? config->nominal_bitrate
                               : config->data_bitrate;
}

static CAN_CODE boolean Can_ValidConfiguration(const Can_ConfigType *config)
{
    boolean valid = (config != NULL_PTR) &&
                    ((Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) || (config->sink != NULL_PTR));
    if (valid) {
        if (config->controller_count == 0u) {
            valid = (config->data_bitrate == 0u) || (config->nominal_bitrate != 0u);
        } else {
            valid = (config->controller_count <= CAN_CONTROLLER_CAPACITY) &&
                    (config->controllers != NULL_PTR) && (config->hardware != NULL_PTR) &&
                    (config->hardware_count <= CAN_HOH_CAPACITY);
        }
    }
    return valid;
}

CAN_CODE void Can_Init(const Can_ConfigType *Config)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized == 0u) && (driver_lifetime != UINT64_MAX) &&
        Can_ValidConfiguration(Config)) {
        if (Config->controller_count == 0u) {
            const Can_ControllerConfigType standalone = {CAN_CONTROLLER_ID,
                                                         CAN_CANIF_CONTROLLER_ID,
                                                         Config->nominal_bitrate,
                                                         Config->data_bitrate,
                                                         Config->brs};
            Can_InstallController(&controllers[0], &standalone);
        } else {
            for (uint16 i = 0u; (i < CAN_CONTROLLER_CAPACITY) && (i < Config->controller_count);
                 ++i) {
                Can_InstallController(&controllers[i], &Config->controllers[i]);
            }
        }
        ++driver_lifetime;
        can_configuration = Config;
        can_initialized = 1u;
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_DeInit(void)
{
    SchM_Enter_Can_CAN_STATE();
    if (can_initialized != 0u) {
        boolean all_stopped = TRUE;
        const uint16 count = Can_ControllerCount(can_configuration);
        for (uint16 i = 0u; (i < CAN_CONTROLLER_CAPACITY) && (i < count); ++i) {
            all_stopped = all_stopped && (controllers[i].can_controller_mode != CAN_STARTED);
        }
        if (all_stopped) {
            for (uint16 i = 0u; (i < CAN_CONTROLLER_CAPACITY) && (i < count); ++i) {
                Can_ResetController(&controllers[i]);
            }
            can_initialized = 0u;
            can_configuration = NULL_PTR;
        }
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE Std_ReturnType Can_SetControllerMode(uint8 Controller, Can_ControllerStateType Transition)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(Controller);
    if ((state != NULL_PTR) && (state->mode_notification_pending == 0u) &&
        (state->busoff_notification_pending == 0u)) {
        if ((Transition == CAN_CS_STARTED) && (state->can_controller_mode == CAN_STOPPED)) {
            state->can_controller_mode = CAN_STARTED;
            state->can_bus_off = 0u;
            state->pending_controller_mode = CAN_CS_STARTED;
            state->mode_notification_pending = 1u;
            result = E_OK;
        } else if (Transition == CAN_CS_STOPPED) {
            /* The state machine also loops STOPPED to STOPPED; SLEEP to STOPPED leaves the
             * logical sleep without touching the already stopped controller (SWS_Can_00267). */
            (void)Can_CancelHostTransmit(state);
            state->can_controller_mode = CAN_STOPPED;
            (void)Can_ClearPolledRx(state);
            state->tx_pending = 0u;
            state->tx_cancelled = 1u;
            state->tx_confirmation_pending = 0u;
            state->pending_controller_mode = CAN_CS_STOPPED;
            state->mode_notification_pending = 1u;
            result = E_OK;
        } else if ((Transition == CAN_CS_SLEEP) && ((state->can_controller_mode == CAN_STOPPED) ||
                                                    (state->can_controller_mode == CAN_SLEEP))) {
            /* Logical sleep: the virtual controller stays stopped (SWS_Can_00258, 00404). */
            state->can_controller_mode = CAN_SLEEP;
            state->pending_controller_mode = CAN_CS_SLEEP;
            state->mode_notification_pending = 1u;
            result = E_OK;
        } else {
            /* Invalid transition (SWS_Can_00409, 00411) leaves the state unchanged. */
        }
    }
    SchM_Exit_Can_CAN_STATE();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerMode(uint8 Controller,
                                              Can_ControllerStateType *ControllerModePtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(Controller);
    if ((state != NULL_PTR) && (ControllerModePtr != NULL_PTR)) {
        if (state->can_controller_mode == CAN_STARTED) {
            *ControllerModePtr = CAN_CS_STARTED;
        } else if (state->can_controller_mode == CAN_SLEEP) {
            *ControllerModePtr = CAN_CS_SLEEP;
        } else {
            *ControllerModePtr = CAN_CS_STOPPED;
        }
        result = E_OK;
    }
    SchM_Exit_Can_CAN_STATE();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerErrorState(uint8 ControllerId,
                                                    Can_ErrorStateType *ErrorStatePtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    const Can_ControllerRuntimeType *state = Can_Controller(ControllerId);
    if ((state != NULL_PTR) && (ErrorStatePtr != NULL_PTR)) {
        *ErrorStatePtr = (state->can_bus_off != 0u) ? CAN_ERRORSTATE_BUSOFF : CAN_ERRORSTATE_ACTIVE;
        result = E_OK;
    }
    SchM_Exit_Can_CAN_STATE();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerRxErrorCounter(uint8 ControllerId,
                                                        uint8 *RxErrorCounterPtr)
{
    (void)ControllerId;
    (void)RxErrorCounterPtr;
    return E_NOT_OK;
}

CAN_CODE Std_ReturnType Can_GetControllerTxErrorCounter(uint8 ControllerId,
                                                        uint8 *TxErrorCounterPtr)
{
    (void)ControllerId;
    (void)TxErrorCounterPtr;
    return E_NOT_OK;
}

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
CAN_CODE void Can_DisableControllerInterrupts(uint8 Controller)
{
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(Controller);
    if ((state != NULL_PTR) && (state->interrupt_disable_count < UINT32_MAX)) {
        ++state->interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
CAN_CODE void Can_EnableControllerInterrupts(uint8 Controller)
{
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(Controller);
    if ((state != NULL_PTR) && (state->interrupt_disable_count != 0u)) {
        --state->interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

static CAN_CODE EcuStatus Can_StoreTransmit(Can_ControllerRuntimeType *state,
                                            const Can_HohConfigType *hoh,
                                            const Can_PduType *pdu,
                                            const uint8 *data)
{
    uint8 i;
    /* SWS_Can_00486: a non-FD controller ignores the FD bit. */
    state->tx_id = (state->fd_tx_bitrate != 0u) ? pdu->id : (pdu->id & ~CAN_ID_FD_FLAG);
    state->tx_handle = pdu->swPduHandle;
    state->tx_length = pdu->length;
    if ((state->tx_id & CAN_ID_FD_FLAG) != 0u) {
        state->tx_length = Can_FdDlcToLength(Can_FdLengthToDlc(pdu->length));
    }
    for (i = 0u; i < pdu->length; ++i) {
        state->tx_payload[i] = data[i];
    }
    /* SWS_Can_00502: the driver owns the bytes added to reach the wire DLC. */
    for (i = pdu->length; i < state->tx_length; ++i) {
        state->tx_payload[i] = hoh->fd_padding;
    }
    state->tx_pending = 1u;
    return ECU_OK;
}

static CAN_CODE boolean Can_HohMatchesId(uint8 format, Can_IdType id)
{
    return (format == CAN_HOH_ID_MIXED) ||
           ((format == CAN_HOH_ID_EXTENDED) == ((id & CAN_ID_EXTENDED_FLAG) != 0u));
}

/* Array positions are independent of the configured identifiers. */
static CAN_CODE boolean Can_TransmitHoh(Can_HwHandleType handle, Can_HohConfigType *hoh)
{
    boolean found = FALSE;
    if (can_initialized != 0u) {
        if (can_configuration->controller_count == 0u) {
            if (handle == CAN_TX_HOH) {
                const Can_HohConfigType standalone = {CAN_TX_HOH,
                                                      CAN_CONTROLLER_ID,
                                                      CAN_TX_ID_TYPE,
                                                      FALSE,
                                                      can_configuration->fd_padding};
                *hoh = standalone;
                found = TRUE;
            }
        } else {
            for (uint32 i = 0u; (i < CAN_HOH_CAPACITY) && (i < can_configuration->hardware_count);
                 ++i) {
                const Can_HohConfigType *entry = &can_configuration->hardware[i];
                if ((entry->handle == handle) && (!entry->receive)) {
                    *hoh = *entry;
                    found = TRUE;
                    break;
                }
            }
        }
    }
    return found;
}

static CAN_CODE EcuStatus Can_WriteHost(Can_HwHandleType hth,
                                        const Can_PduType *pdu,
                                        const uint8 *data)
{
    EcuStatus result = ECU_ERR_CONFIG;
    /* The area guards data only, so Can_Write stays non-blocking (SWS_Can_00275). */
    SchM_Enter_Can_CAN_STATE();
    Can_HohConfigType hoh;
    if ((pdu != NULL_PTR) && (data != NULL_PTR) && Can_TransmitHoh(hth, &hoh)) {
        Can_ControllerRuntimeType *state = Can_Controller(hoh.controller);
        if ((!Can_IsValidId(pdu->id)) || (!Can_HohMatchesId(hoh.format, pdu->id))) {
            result = ECU_ERR_FRAME_ID;
        } else if (state == NULL_PTR) {
            /* No installed controller owns this object. */
        } else if ((pdu->length > CAN_FD_MAX_LENGTH) ||
                   ((pdu->length > CAN_CLASSICAL_MAX_LENGTH) &&
                    ((state->fd_tx_bitrate == 0u) || ((pdu->id & CAN_ID_FD_FLAG) == 0u)))) {
            result = ECU_ERR_FRAME_DLC;
        } else if (state->can_controller_mode != CAN_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else if ((state->tx_pending != 0u) || (state->tx_in_flight != 0u) ||
                   (state->tx_confirmation_pending != 0u) || (state->tx_confirming != 0u)) {
            result = ECU_ERR_CAN_BUSY;
        } else {
            result = Can_AdmitTransmit(state, &hoh, pdu, data);
        }
    }
    SchM_Exit_Can_CAN_STATE();
    return result;
}

CAN_CODE Std_ReturnType Can_Write(Can_HwHandleType Hth, const Can_PduType *PduInfo)
{
    Std_ReturnType result = E_NOT_OK;
    EcuStatus host_result =
        Can_WriteHost(Hth, PduInfo, (PduInfo != NULL_PTR) ? PduInfo->sdu : NULL_PTR);
    if (host_result == ECU_OK) {
        result = E_OK;
    } else if (host_result == ECU_ERR_CAN_BUSY) {
        result = CAN_BUSY;
    } else {
        /* Invalid request retains E_NOT_OK. */
    }
    return result;
}

static CAN_CODE EcuStatus Can_Flush(Can_ControllerRuntimeType *state)
{
    EcuStatus result = ECU_OK;
    SchM_Enter_Can_CAN_STATE();
    if (can_initialized == 0u) {
        result = ECU_ERR_CONFIG;
    } else if ((state->tx_pending != 0u) && (state->tx_in_flight == 0u)) {
        const uint32 id = state->tx_id;
        const uint8 length = state->tx_length;
        const boolean fd = (id & CAN_ID_FD_FLAG) != 0u;
        const boolean brs = fd && state->fd_tx_brs;
        const uint32 data_phase_bitrate = fd ? state->fd_tx_bitrate : 0u;
        const PduIdType handle = state->tx_handle;
        const Can_TxSinkType sink = can_configuration->sink;
        const uint8 controller = state->controller;
        const uint64 lifetime = driver_lifetime;
        uint8 payload[CAN_FD_MAX_LENGTH] = {0};
        uint8 i;
        for (i = 0u; i < length; ++i) {
            payload[i] = state->tx_payload[i];
        }
        state->tx_pending = 0u;
        state->tx_in_flight = 1u;
        state->tx_cancelled = 0u;
        SchM_Exit_Can_CAN_STATE();
        result = Ecu_ExecutionTransmit(
            sink, controller, handle, id, length, payload, brs, data_phase_bitrate);
        SchM_Enter_Can_CAN_STATE();
        state->tx_in_flight = 0u;
        if ((can_initialized != 0u) && (lifetime == driver_lifetime) &&
            (state->can_controller_mode == CAN_STARTED) && (state->tx_cancelled == 0u) &&
            (result == ECU_OK) && (Ecu_Policy.tx_confirmation == ECU_TX_SYNCHRONOUS)) {
            state->tx_confirmation_handle = handle;
            state->tx_confirmation_pending = 1u;
        }
    } else {
        /* No admitted output is ready. */
    }
    SchM_Exit_Can_CAN_STATE();
    (void)Can_ConfirmImmediateTx(state);
    return result;
}

CAN_CODE EcuStatus Can_InternalHostFlush(void)
{
    EcuStatus result = ECU_OK;
    for (uint16 i = 0u; (i < CAN_CONTROLLER_CAPACITY) && (result == ECU_OK); ++i) {
        result = Can_Flush(&controllers[i]);
    }
    return result;
}

/* Mode and bus-off indications leave the driver one context at a time, in the order
 * they were recorded: a context that finds one in delivery only records,
 * and the delivering context (or the next main function) publishes it. `drain` keeps
 * delivering what is recorded during a callback; polled publication stops after one. */
static CAN_CODE void Can_PublishMode(Can_ControllerRuntimeType *state, uint8 drain)
{
    uint8 more = 1u;
    while (more != 0u) {
        Can_ControllerStateType mode = CAN_CS_STOPPED;
        uint8 controller = 0u;
        more = 0u;
        SchM_Enter_Can_CAN_STATE();
        if ((can_initialized != 0u) && (state->mode_notification_pending != 0u) &&
            (state->indication_processing[CAN_INDICATION_MODE] == 0u)) {
            mode = state->pending_controller_mode;
            state->mode_notification_pending = 0u;
            state->indication_processing[CAN_INDICATION_MODE] = 1u;
            controller = state->can_if;
            more = 1u;
        }
        SchM_Exit_Can_CAN_STATE();
        if (more != 0u) {
            CanIf_ControllerModeIndication(controller, mode);
            SchM_Enter_Can_CAN_STATE();
            state->indication_processing[CAN_INDICATION_MODE] = 0u;
            SchM_Exit_Can_CAN_STATE();
            more = drain;
        }
    }
}

static CAN_CODE void Can_PublishBusOff(Can_ControllerRuntimeType *state, uint8 drain)
{
    uint8 more = 1u;
    while (more != 0u) {
        uint8 controller = 0u;
        more = 0u;
        SchM_Enter_Can_CAN_STATE();
        if ((can_initialized != 0u) && (state->busoff_notification_pending != 0u) &&
            (state->indication_processing[CAN_INDICATION_BUSOFF] == 0u)) {
            state->busoff_notification_pending = 0u;
            state->indication_processing[CAN_INDICATION_BUSOFF] = 1u;
            controller = state->can_if;
            more = 1u;
        }
        SchM_Exit_Can_CAN_STATE();
        if (more != 0u) {
            CanIf_ControllerBusOff(controller);
            SchM_Enter_Can_CAN_STATE();
            state->indication_processing[CAN_INDICATION_BUSOFF] = 0u;
            SchM_Exit_Can_CAN_STATE();
            more = drain;
        }
    }
}

CAN_CODE void Can_HostSetControllerMode(uint8 controller, CanMode mode)
{
    uint8 notify_mode = 0u;
    uint8 notify_busoff = 0u;
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(controller);
    if (state == NULL_PTR) {
        /* Host mutation requires Can_Init. */
    } else if ((mode != CAN_STARTED) && (mode != CAN_STOPPED) && (mode != CAN_SLEEP) &&
               (mode != CAN_BUS_OFF)) {
        /* Ignore unsupported host mode values without changing a pending indication. */
    } else if ((state->can_controller_mode == CAN_SLEEP) && (mode != CAN_STOPPED)) {
        /* Logical sleep can only exit via STOPPED. */
    } else if ((state->busoff_notification_pending != 0u) && (mode != CAN_BUS_OFF)) {
        /* Publish the observed bus-off before an explicit host recovery. */
    } else if (mode == CAN_BUS_OFF) {
        (void)Can_RecordBusOff(state);
        state->mode_notification_pending = 0u;
        (void)Can_CancelHostTransmit(state);
        state->can_controller_mode = CAN_STOPPED;
        (void)Can_ClearPolledRx(state);
        state->can_bus_off = 1u;
        state->tx_pending = 0u;
        state->tx_cancelled = 1u;
        state->tx_confirmation_pending = 0u;
        notify_busoff = 1u;
    } else {
        state->mode_notification_pending = 0u;
        state->can_controller_mode = mode;
        state->can_bus_off = 0u;
        if (mode != CAN_STARTED) {
            (void)Can_ClearPolledRx(state);
            (void)Can_CancelHostTransmit(state);
            state->tx_pending = 0u;
            state->tx_cancelled = 1u;
            state->tx_confirmation_pending = 0u;
        }
        if (mode == CAN_STARTED) {
            state->pending_controller_mode = CAN_CS_STARTED;
            notify_mode = 1u;
        } else if (mode == CAN_STOPPED) {
            state->pending_controller_mode = CAN_CS_STOPPED;
            notify_mode = 1u;
        } else if (mode == CAN_SLEEP) {
            state->pending_controller_mode = CAN_CS_SLEEP;
            notify_mode = 1u;
        } else {
            /* Bus-off was handled above; other values have no CanIf mode. */
        }
        state->mode_notification_pending = notify_mode;
    }
    SchM_Exit_Can_CAN_STATE();
    /* The bus-off and mode branches above are exclusive; a polled bus-off is
     * published by Can_MainFunction_BusOff instead. */
    if (notify_busoff != 0u) {
        (void)Can_PublishImmediateBusOff(state);
    }
    if (notify_mode != 0u) {
        Can_PublishMode(state, 1u);
    }
}

CAN_CODE void Can_InternalSetMode(CanMode mode)
{
    Can_HostSetControllerMode(CAN_CONTROLLER_ID, mode);
}

CAN_CODE void Can_MainFunction_BusOff(void)
{
    for (uint16 i = 0u; i < CAN_CONTROLLER_CAPACITY; ++i) {
        Can_PublishBusOff(&controllers[i], 0u);
    }
}

CAN_CODE void Can_MainFunction_Mode(void)
{
    for (uint16 i = 0u; i < CAN_CONTROLLER_CAPACITY; ++i) {
        Can_PublishMode(&controllers[i], 0u);
    }
}

CAN_CODE void Can_MainFunction_Wakeup(void) { Can_MainFunction_Mode(); }

CAN_CODE CanMode Can_InternalGetMode(void)
{
    CanMode mode = CAN_STOPPED;
    SchM_Enter_Can_CAN_STATE();
    const Can_ControllerRuntimeType *state = Can_Controller(CAN_CONTROLLER_ID);
    if (state != NULL_PTR) {
        mode = (state->can_bus_off != 0u) ? CAN_BUS_OFF : state->can_controller_mode;
    }
    SchM_Exit_Can_CAN_STATE();
    return mode;
}

CAN_CODE EcuStatus Can_InternalTransmitPdu(PduIdType pdu_id,
                                           uint32 id,
                                           uint8 dlc,
                                           const uint8 *data)
{
    const Can_PduType pdu = {pdu_id, dlc, id, NULL_PTR};
    EcuStatus result = Can_WriteHost(CAN_TX_HOH, &pdu, data);
    if (result == ECU_OK) {
        result = Can_InternalHostFlush();
    }
    return result;
}

static CAN_CODE void Can_ConfirmTx(Can_ControllerRuntimeType *state)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (state->tx_confirmation_pending != 0u) &&
        (state->tx_confirming == 0u)) {
        const PduIdType handle = state->tx_confirmation_handle;
        state->tx_confirmation_pending = 0u;
        state->tx_confirming = 1u;
        SchM_Exit_Can_CAN_STATE();
        CanIf_TxConfirmation(handle);
        SchM_Enter_Can_CAN_STATE();
        state->tx_confirming = 0u;
    }
    SchM_Exit_Can_CAN_STATE();
}

static CAN_CODE boolean Can_HasReceiveHoh(uint8 controller, uint32 id)
{
    boolean result = FALSE;
    const Can_ConfigType *config = can_configuration;
    if (config->controller_count == 0u) {
        result = Can_HohMatchesId(CAN_RX_ID_TYPE, id);
    } else {
        for (uint32 i = 0u; (i < CAN_HOH_CAPACITY) && (i < config->hardware_count) && !result;
             ++i) {
            const Can_HohConfigType *hoh = &config->hardware[i];
            result = hoh->receive && (hoh->controller == controller) &&
                     Can_HohMatchesId(hoh->format, id);
        }
    }
    return result;
}

static CAN_CODE EcuStatus
Can_ReceiveHost(uint8 controller, uint32 id, uint8 dlc, const uint8 *data, uint8 accepted)
{
    EcuStatus result = ECU_OK;
    uint8 lost = 0u;
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(controller);
    if (!Can_IsValidId(id)) {
        result = ECU_ERR_FRAME_ID;
    } else if (state == NULL_PTR) {
        result = ECU_ERR_CONTROLLER;
    } else if ((!Can_IsWireLength(id, dlc)) ||
               (((id & CAN_ID_FD_FLAG) != 0u) && (state->fd_tx_bitrate == 0u))) {
        result = ECU_ERR_FRAME_DLC;
    } else if ((can_initialized == 0u) || (state->can_controller_mode != CAN_STARTED)) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL_PTR) {
        result = ECU_ERR_CONFIG;
    } else if (!Can_HasReceiveHoh(controller, id)) {
        /* No receive hardware object on this controller accepts the identifier width. */
    } else if ((state->rx_processing != 0u) || (state->rx_pending != 0u)) {
        result = ECU_ERR_CAN_BUSY;
        lost = accepted;
        if (accepted != 0u) {
            state->rx_overrun_pending = 1u;
        }
    } else {
        uint8 i;
        state->rx_id = id;
        state->rx_length = dlc;
        for (i = 0u; i < dlc; ++i) {
            state->rx_payload[i] = data[i];
        }
        state->rx_pending = 1u;
        result = Can_DeliverImmediateRx(state);
    }
    SchM_Exit_Can_CAN_STATE();
    if (lost != 0u) {
        (void)Can_ReadImmediateRx(state);
    }
    return result;
}

CAN_CODE EcuStatus Can_InternalInject(uint32 id, uint8 dlc, const uint8 data[8], uint64 now_ms)
{
    (void)now_ms; /* CanIf_RxIndication carries no reception time. */
    return Can_ReceiveHost(CAN_CONTROLLER_ID, id, dlc, data, 0u);
}

CAN_CODE EcuStatus Can_HostInjectChannel(uint8 controller, uint32 id, uint8 dlc, const uint8 *data)
{
    return Can_ReceiveHost(controller, id, dlc, data, 0u);
}

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
CAN_CODE EcuStatus Can_InternalRxIndication(uint32 id,
                                            uint8 dlc,
                                            const uint8 data[8],
                                            uint64 now_ms)
{
    /* Already received by a platform adapter: occupied shadow storage is a real overrun.
     * Host admission uses Can_InternalInject and remains retryable without a lost-frame report. */
    (void)now_ms;
    return Can_ReceiveHost(CAN_CONTROLLER_ID, id, dlc, data, 1u);
}

/* Called with the driver lock held; returns with it held. The result belongs
 * to this snapshot, including when the callback cancels/reinitializes the driver. */
static CAN_CODE EcuStatus Can_DeliverRx(Can_ControllerRuntimeType *state)
{
    EcuStatus result = ECU_OK;
    if ((can_initialized != 0u) && (state->rx_processing == 0u) && (state->rx_pending != 0u)) {
        uint32 id = state->rx_id;
        uint8 length = state->rx_length;
        uint8 payload[CAN_FD_MAX_LENGTH] = {0};
        const uint64 lifetime = driver_lifetime;
        const Can_ConfigType *config = can_configuration;
        const uint8 controller = state->controller;
        const uint8 can_if = state->can_if;
        PduInfoType info = {payload, NULL_PTR, length};
        uint8 i;
        for (i = 0u; i < length; ++i) {
            payload[i] = state->rx_payload[i];
        }
        state->rx_pending = 0u;
        state->rx_processing = 1u;
        if (config->controller_count == 0u) {
            const Can_HwType mailbox = {id, CAN_RX_HOH, can_if};
            SchM_Exit_Can_CAN_STATE();
            CanIf_RxIndication(&mailbox, &info);
            SchM_Enter_Can_CAN_STATE();
        } else {
            for (uint32 h = 0u; (h < CAN_HOH_CAPACITY) && (h < config->hardware_count); ++h) {
                const Can_HohConfigType *hoh = &config->hardware[h];
                if (hoh->receive && (hoh->controller == controller) &&
                    Can_HohMatchesId(hoh->format, id)) {
                    const Can_HwType mailbox = {id, hoh->handle, can_if};
                    SchM_Exit_Can_CAN_STATE();
                    CanIf_RxIndication(&mailbox, &info);
                    SchM_Enter_Can_CAN_STATE();
                    if ((can_initialized == 0u) || (driver_lifetime != lifetime) ||
                        (state->can_controller_mode != CAN_STARTED)) {
                        break;
                    }
                }
            }
        }
        if ((can_initialized == 0u) || (driver_lifetime != lifetime)) {
            result = ECU_ERR_CONTROLLER;
        }
        state->rx_processing = 0u;
    }
    return result;
}

static CAN_CODE void Can_ReadRx(Can_ControllerRuntimeType *state)
{
    uint8 lost;
    SchM_Enter_Can_CAN_STATE();
    lost = state->rx_overrun_pending;
    state->rx_overrun_pending = 0u;
    if (lost != 0u) {
        SchM_Exit_Can_CAN_STATE();
        /* SWS_Can_00226: report before Rx can reentrantly replace consumers. */
        (void)Det_ReportRuntimeError(CAN_MODULE_ID, 0u, CAN_SID_MAIN_FUNCTION_READ, CAN_E_DATALOST);
        SchM_Enter_Can_CAN_STATE();
    }
    (void)Can_DeliverRx(state);
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_MainFunction_Write(void)
{
    for (uint16 i = 0u; i < CAN_CONTROLLER_CAPACITY; ++i) {
        Can_ConfirmTx(&controllers[i]);
    }
}

CAN_CODE void Can_MainFunction_Read(void)
{
    for (uint16 i = 0u; i < CAN_CONTROLLER_CAPACITY; ++i) {
        Can_ReadRx(&controllers[i]);
    }
}

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"

/* Optional actions are selected around complete mapped definitions. */

#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_CancelHostTransmit(Can_ControllerRuntimeType *state)
{
    state->host_tx_cancelled_through = state->host_tx_sequence;
}
static CAN_CODE EcuStatus Can_AdmitTransmit(Can_ControllerRuntimeType *state,
                                            const Can_HohConfigType *hoh,
                                            const Can_PduType *pdu,
                                            const uint8 *data)
{
    EcuStatus result = ECU_ERR_CAN_BUSY;
    if (state->host_tx_sequence != UINT64_MAX) {
        ++state->host_tx_sequence;
        result = Can_StoreTransmit(state, hoh, pdu, data);
    }
    return result;
}
CAN_CODE uint64 Can_HostTransmitTokenFor(uint8 controller)
{
    uint64 token = UINT64_C(0);
    SchM_Enter_Can_CAN_STATE();
    const Can_ControllerRuntimeType *state = Can_Controller(controller);
    if ((state != NULL_PTR) && (state->tx_in_flight != 0u) &&
        (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED)) {
        token = state->host_tx_sequence;
    }
    SchM_Exit_Can_CAN_STATE();
    return token;
}

CAN_CODE Std_ReturnType Can_HostCompleteTransmitFor(uint8 controller,
                                                    PduIdType handle,
                                                    uint64 token)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    Can_ControllerRuntimeType *state = Can_Controller(controller);
    if ((state != NULL_PTR) && (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) &&
        (token > state->host_tx_cancelled_through) && (token > state->host_tx_completed_through) &&
        (token <= state->host_tx_sequence)) {
        if ((state->tx_confirmation_pending != 0u) || (state->tx_confirming != 0u)) {
            result = CAN_BUSY;
        } else {
            state->tx_confirmation_handle = handle;
            state->tx_confirmation_pending = 1u;
            state->host_tx_completed_through = token;
            result = E_OK;
        }
    }
    SchM_Exit_Can_CAN_STATE();
    return result;
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_CancelHostTransmit(Can_ControllerRuntimeType *state)
{
    (void)state; /* This configuration has no queued host token to cancel. */
}
static CAN_CODE EcuStatus Can_AdmitTransmit(Can_ControllerRuntimeType *state,
                                            const Can_HohConfigType *hoh,
                                            const Can_PduType *pdu,
                                            const uint8 *data)
{
    return Can_StoreTransmit(state, hoh, pdu, data);
}
CAN_CODE uint64 Can_HostTransmitTokenFor(uint8 controller)
{
    (void)controller;
    return UINT64_C(0);
}
CAN_CODE Std_ReturnType Can_HostCompleteTransmitFor(uint8 controller,
                                                    PduIdType handle,
                                                    uint64 token)
{
    Std_ReturnType result = E_NOT_OK;
    (void)controller;
    (void)handle;
    (void)token;
    return result;
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
CAN_CODE uint64 Can_InternalHostTransmitToken(void)
{
    return Can_HostTransmitTokenFor(CAN_CONTROLLER_ID);
}

CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64 token)
{
    return Can_HostCompleteTransmitFor(CAN_CONTROLLER_ID, handle, token);
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"

#if (CAN_RX_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ClearPolledRx(Can_ControllerRuntimeType *state) { state->rx_pending = 0u; }
static CAN_CODE EcuStatus Can_DeliverImmediateRx(Can_ControllerRuntimeType *state)
{
    (void)state;
    return ECU_OK;
}
static CAN_CODE void Can_ReadImmediateRx(Can_ControllerRuntimeType *state)
{
    (void)state;
    /* Polling owns receive publication. */
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ClearPolledRx(Can_ControllerRuntimeType *state)
{
    (void)state; /* Immediate receive has no polling buffer to retire. */
}
static CAN_CODE EcuStatus Can_DeliverImmediateRx(Can_ControllerRuntimeType *state)
{
    return Can_DeliverRx(state);
}
static CAN_CODE void Can_ReadImmediateRx(Can_ControllerRuntimeType *state) { Can_ReadRx(state); }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#if (CAN_TX_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ConfirmImmediateTx(Can_ControllerRuntimeType *state)
{
    (void)state;
    /* Polling owns transmit confirmation. */
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ConfirmImmediateTx(Can_ControllerRuntimeType *state)
{
    Can_ConfirmTx(state);
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#if (CAN_BUSOFF_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_RecordBusOff(Can_ControllerRuntimeType *state)
{
    if (state->can_bus_off == 0u) {
        state->busoff_notification_pending = 1u;
    }
}
static CAN_CODE void Can_PublishImmediateBusOff(Can_ControllerRuntimeType *state)
{
    (void)state;
    /* Polling owns bus-off publication. */
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_RecordBusOff(Can_ControllerRuntimeType *state)
{
    state->busoff_notification_pending = 1u;
}
static CAN_CODE void Can_PublishImmediateBusOff(Can_ControllerRuntimeType *state)
{
    Can_PublishBusOff(state, 1u);
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
