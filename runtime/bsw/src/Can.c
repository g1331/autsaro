/** @file Standard Can driver for one CAN controller on the controlled host. */
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
#ifndef CAN_CONTROLLER_ID
#define CAN_CONTROLLER_ID 0u
#endif
#ifndef CAN_CANIF_CONTROLLER_ID
#define CAN_CANIF_CONTROLLER_ID 0u
#endif
#ifndef CAN_TX_HOH
#define CAN_TX_HOH 0u
#endif
#define CAN_HOH_ID_STANDARD 0u
#define CAN_HOH_ID_EXTENDED 1u
#define CAN_HOH_ID_MIXED    2u
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

/* Every variable in this section is read and written only inside CAN_STATE. "Init resets"
 * means Can_Init and Can_DeInit both reset it; the others keep their value across both.
 * driver_lifetime counts successful Can_Init calls: a context that returns from a callback into
 * another lifetime drops its result. tx_sink is the synchronous output from the Init
 * configuration and is NULL after Can_DeInit. bus_off stays set after a host bus-off until
 * STARTED or a host mode change. */
static uint8 can_initialized CAN_VAR_CLEARED;  /**< 1 between Init and DeInit. */
static uint64 driver_lifetime CAN_VAR_CLEARED; /**< Successful Init count. */
static Can_TxSinkType tx_sink CAN_VAR_CLEARED; /**< Host output; set by Init. */
static uint8 fd_padding CAN_VAR_CLEARED;       /**< FD padding byte; set by Init. */
static uint32 fd_tx_bitrate CAN_VAR_CLEARED;   /**< FD Tx bitrate; zero disables FD; set by Init. */
static boolean fd_tx_brs CAN_VAR_CLEARED;      /**< FD bitrate switch; set by Init. */
static CanMode can_controller_mode CAN_VAR_CLEARED;       /**< Controller state; Init: STOPPED. */
static uint8 can_bus_off CAN_VAR_CLEARED;                 /**< In bus-off; Init resets. */
static uint8 busoff_notification_pending CAN_VAR_CLEARED; /**< Bus-off for CanIf; Init resets. */

/* Transmit slot, one L-PDU at a time. Can_Write sets tx_pending; the flush clears it and sets
 * tx_in_flight while the host output runs outside the area; a successful synchronous output,
 * or a queued completion, sets tx_confirmation_pending; Can_ConfirmTx clears it and sets
 * tx_confirming while CanIf_TxConfirmation runs. Can_Write returns CAN_BUSY while any of the
 * four is set. Synchronous outputs keep exactly one set at a time. A queued output frees the
 * slot when the flush returns, so the next frame's tx_pending or tx_in_flight can coexist with
 * the previous frame's tx_confirmation_pending or tx_confirming. A stop (STOPPED, bus-off or a
 * host SLEEP) and Can_DeInit clear tx_pending and tx_confirmation_pending and set tx_cancelled,
 * so the running output yields no confirmation; the next output clears tx_cancelled. Can_Init
 * clears the two flags only. tx_in_flight and tx_confirming are cleared only by the context
 * that set them, so they outlive a reinitialization. */
static uint8 tx_pending CAN_VAR_CLEARED;              /**< Admitted, not yet output. */
static uint8 tx_in_flight CAN_VAR_CLEARED;            /**< Host output running. */
static uint8 tx_cancelled CAN_VAR_CLEARED;            /**< Output gets no confirmation. */
static uint8 tx_confirmation_pending CAN_VAR_CLEARED; /**< Confirmation for CanIf. */
static uint8 tx_confirming CAN_VAR_CLEARED;           /**< CanIf_TxConfirmation running. */

/* Admitted frame, valid while tx_pending; tx_confirmation_handle is valid while
 * tx_confirmation_pending. */
static uint32 tx_id CAN_VAR_CLEARED;                        /**< Encoded identifier. */
static uint8 tx_length CAN_VAR_CLEARED;                     /**< Wire length, 0 to 64 bytes. */
static uint8 tx_payload[CAN_FD_MAX_LENGTH] CAN_VAR_CLEARED; /**< tx_length bytes valid. */
static PduIdType tx_handle CAN_VAR_CLEARED;                 /**< swPduHandle of the frame. */
static PduIdType tx_confirmation_handle CAN_VAR_CLEARED;    /**< swPduHandle to confirm. */
#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"
/* Lifetime tokens are never reset by Init: old physical outputs must not
 * complete a new same-handle request after cancellation or reinitialization. */
static uint64 host_tx_sequence CAN_VAR_CLEARED;          /**< Token of the last admitted frame. */
static uint64 host_tx_cancelled_through CAN_VAR_CLEARED; /**< Tokens up to here never complete. */
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
static uint64 host_tx_completed_through CAN_VAR_CLEARED; /**< Tokens up to here have completed. */
#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"
#endif

#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

static uint32 interrupt_disable_count CAN_VAR_CLEARED; /**< Disable nesting; Init resets. */

/* Mode and bus-off indications. mode_notification_pending is set while pending_controller_mode
 * still has to be given to CanIf; Init resets it. indication_processing[CAN_INDICATION_*] is 1
 * while that kind is being delivered; it is never reset, so it outlives a reinitialization. */
static Can_ControllerStateType pending_controller_mode CAN_VAR_CLEARED; /**< Mode to indicate. */
static uint8 mode_notification_pending CAN_VAR_CLEARED;                 /**< Mode not yet sent. */
static uint8 indication_processing[2] CAN_VAR_CLEARED;                  /**< Kind in delivery. */

/* Shadow receive buffer: rx_id, rx_length and rx_payload are valid while rx_pending. */
static uint8 rx_pending CAN_VAR_CLEARED;                    /**< Frame buffered; Init resets. */
static uint8 rx_processing CAN_VAR_CLEARED;                 /**< CanIf_RxIndication running. */
static uint32 rx_id CAN_VAR_CLEARED;                        /**< Encoded identifier. */
static uint8 rx_length CAN_VAR_CLEARED;                     /**< Wire length, 0 to 64 bytes. */
static uint8 rx_payload[CAN_FD_MAX_LENGTH] CAN_VAR_CLEARED; /**< rx_length bytes valid. */
static uint8 rx_overrun_pending CAN_VAR_CLEARED; /**< CAN_E_DATALOST to report; Init resets. */

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

static CAN_CODE void Can_CancelHostTransmit(void);
static CAN_CODE EcuStatus Can_AdmitTransmit(const Can_PduType *pdu, const uint8 *data);
static CAN_CODE void Can_ClearPolledRx(void);
static CAN_CODE EcuStatus Can_DeliverImmediateRx(void);
static CAN_CODE void Can_ReadImmediateRx(void);
static CAN_CODE void Can_ConfirmImmediateTx(void);
static CAN_CODE void Can_RecordBusOff(void);
static CAN_CODE void Can_PublishImmediateBusOff(void);
static CAN_CODE EcuStatus Can_DeliverRx(void);
/* Scheduled-function bodies, also run on direct (non-polled) events (SWS_BSW_00133). */
static CAN_CODE void Can_ConfirmTx(void);
static CAN_CODE void Can_ReadRx(void);

CAN_CODE void Can_Init(const Can_ConfigType *Config)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized == 0u) && (Config != NULL_PTR) &&
        ((Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) || (Config->sink != NULL_PTR)) &&
        ((Config->data_bitrate == 0u) || (Config->nominal_bitrate != 0u)) &&
        (driver_lifetime != UINT64_MAX)) {
        (void)Can_CancelHostTransmit();
        ++driver_lifetime;
        tx_sink = Config->sink;
        fd_padding = Config->fd_padding;
        fd_tx_bitrate = Config->data_bitrate;
        fd_tx_brs = (fd_tx_bitrate != 0u) && Config->brs;
        if ((fd_tx_bitrate != 0u) && (!fd_tx_brs)) {
            fd_tx_bitrate = Config->nominal_bitrate;
        }
        can_initialized = 1u;
        can_controller_mode = CAN_STOPPED;
        can_bus_off = 0u;
        busoff_notification_pending = 0u;
        tx_pending = 0u;
        tx_confirmation_pending = 0u;
        rx_pending = 0u;
        rx_overrun_pending = 0u;
        interrupt_disable_count = 0u;
        mode_notification_pending = 0u;
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_DeInit(void)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (can_controller_mode != CAN_STARTED)) {
        (void)Can_CancelHostTransmit();
        can_initialized = 0u;
        tx_cancelled = 1u;
        tx_sink = NULL_PTR;
        can_controller_mode = CAN_STOPPED;
        can_bus_off = 0u;
        busoff_notification_pending = 0u;
        tx_pending = 0u;
        tx_confirmation_pending = 0u;
        rx_pending = 0u;
        rx_overrun_pending = 0u;
        interrupt_disable_count = 0u;
        mode_notification_pending = 0u;
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE Std_ReturnType Can_SetControllerMode(uint8 Controller, Can_ControllerStateType Transition)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (mode_notification_pending == 0u) && (busoff_notification_pending == 0u)) {
        if ((Transition == CAN_CS_STARTED) && (can_controller_mode == CAN_STOPPED)) {
            can_controller_mode = CAN_STARTED;
            can_bus_off = 0u;
            pending_controller_mode = CAN_CS_STARTED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if (Transition == CAN_CS_STOPPED) {
            /* The state machine also loops STOPPED to STOPPED; SLEEP to STOPPED leaves the
             * logical sleep without touching the already stopped controller (SWS_Can_00267). */
            (void)Can_CancelHostTransmit();
            can_controller_mode = CAN_STOPPED;
            (void)Can_ClearPolledRx();
            tx_pending = 0u;
            tx_cancelled = 1u;
            tx_confirmation_pending = 0u;
            pending_controller_mode = CAN_CS_STOPPED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if ((Transition == CAN_CS_SLEEP) &&
                   ((can_controller_mode == CAN_STOPPED) || (can_controller_mode == CAN_SLEEP))) {
            /* Logical sleep: the virtual controller stays stopped (SWS_Can_00258, 00404). */
            can_controller_mode = CAN_SLEEP;
            pending_controller_mode = CAN_CS_SLEEP;
            mode_notification_pending = 1u;
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
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (ControllerModePtr != NULL_PTR)) {
        if (can_controller_mode == CAN_STARTED) {
            *ControllerModePtr = CAN_CS_STARTED;
        } else if (can_controller_mode == CAN_SLEEP) {
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
    if ((can_initialized != 0u) && (ControllerId == CAN_CONTROLLER_ID) &&
        (ErrorStatePtr != NULL_PTR)) {
        *ErrorStatePtr = (can_bus_off != 0u) ? CAN_ERRORSTATE_BUSOFF : CAN_ERRORSTATE_ACTIVE;
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
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count < UINT32_MAX)) {
        ++interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
CAN_CODE void Can_EnableControllerInterrupts(uint8 Controller)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count != 0u)) {
        --interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

static CAN_CODE EcuStatus Can_StoreTransmit(const Can_PduType *pdu, const uint8 *data)
{
    uint8 i;
    /* SWS_Can_00486: a non-FD controller ignores the FD bit. */
    tx_id = (fd_tx_bitrate != 0u) ? pdu->id : (pdu->id & ~CAN_ID_FD_FLAG);
    tx_handle = pdu->swPduHandle;
    tx_length = pdu->length;
    if ((tx_id & CAN_ID_FD_FLAG) != 0u) {
        tx_length = Can_FdDlcToLength(Can_FdLengthToDlc(pdu->length));
    }
    for (i = 0u; i < pdu->length; ++i) {
        tx_payload[i] = data[i];
    }
    /* SWS_Can_00502: the driver owns the bytes added to reach the wire DLC. */
    for (i = pdu->length; i < tx_length; ++i) {
        tx_payload[i] = fd_padding;
    }
    tx_pending = 1u;
    return ECU_OK;
}

static CAN_CODE boolean Can_HohMatchesId(uint8 format, Can_IdType id)
{
    return (format == CAN_HOH_ID_MIXED) ||
           ((format == CAN_HOH_ID_EXTENDED) == ((id & CAN_ID_EXTENDED_FLAG) != 0u));
}

static CAN_CODE EcuStatus Can_WriteHost(Can_HwHandleType hth,
                                        const Can_PduType *pdu,
                                        const uint8 *data)
{
    EcuStatus result = ECU_ERR_CONFIG;
    if ((hth != CAN_TX_HOH) || (pdu == NULL_PTR) || (data == NULL_PTR)) {
        /* Invalid host configuration or caller input. */
    } else if (!Can_IsValidId(pdu->id) || !Can_HohMatchesId(CAN_TX_ID_TYPE, pdu->id)) {
        result = ECU_ERR_FRAME_ID;
    } else {
        /* The area guards data only, so entering it is bounded and keeps
         * Can_Write non-blocking (SWS_Can_00275); CAN_BUSY means an occupied slot. */
        SchM_Enter_Can_CAN_STATE();
        if (can_initialized == 0u) {
            /* Can_Init has not installed a host output callback. */
        } else if ((pdu->length > CAN_FD_MAX_LENGTH) ||
                   ((pdu->length > CAN_CLASSICAL_MAX_LENGTH) &&
                    ((fd_tx_bitrate == 0u) || ((pdu->id & CAN_ID_FD_FLAG) == 0u)))) {
            /* SWS_Can_00218: CanDevErrorDetect is disabled in this profile. */
            result = ECU_ERR_FRAME_DLC;
        } else if (can_controller_mode != CAN_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else if ((tx_pending != 0u) || (tx_in_flight != 0u) || (tx_confirmation_pending != 0u) ||
                   (tx_confirming != 0u)) {
            result = ECU_ERR_CAN_BUSY;
        } else {
            result = Can_AdmitTransmit(pdu, data);
        }
        SchM_Exit_Can_CAN_STATE();
    }
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

CAN_CODE EcuStatus Can_InternalHostFlush(void)
{
    EcuStatus result = ECU_OK;
    SchM_Enter_Can_CAN_STATE();
    if (can_initialized == 0u) {
        result = ECU_ERR_CONFIG;
    } else if ((tx_pending != 0u) && (tx_in_flight == 0u)) {
        const uint32 id = tx_id;
        const uint8 length = tx_length;
        const boolean fd = (id & CAN_ID_FD_FLAG) != 0u;
        const boolean brs = fd && fd_tx_brs;
        const uint32 data_phase_bitrate = fd ? fd_tx_bitrate : 0u;
        const PduIdType handle = tx_handle;
        const Can_TxSinkType sink = tx_sink;
        const uint64 lifetime = driver_lifetime;
        uint8 payload[CAN_FD_MAX_LENGTH] = {0};
        uint8 i;
        for (i = 0u; i < length; ++i) {
            payload[i] = tx_payload[i];
        }
        tx_pending = 0u;
        tx_in_flight = 1u;
        tx_cancelled = 0u;
        SchM_Exit_Can_CAN_STATE();
        result = Ecu_ExecutionTransmit(sink, handle, id, length, payload, brs, data_phase_bitrate);
        SchM_Enter_Can_CAN_STATE();
        tx_in_flight = 0u;
        if ((can_initialized != 0u) && (lifetime == driver_lifetime) &&
            (can_controller_mode == CAN_STARTED) && (tx_cancelled == 0u) && (result == ECU_OK) &&
            (Ecu_Policy.tx_confirmation == ECU_TX_SYNCHRONOUS)) {
            tx_confirmation_handle = handle;
            tx_confirmation_pending = 1u;
        }
    } else {
        /* No admitted output is ready. */
    }
    SchM_Exit_Can_CAN_STATE();
    (void)Can_ConfirmImmediateTx();
    return result;
}

/* Mode and bus-off indications leave the driver one context at a time, in the order
 * they were recorded: a context that finds one in delivery only records,
 * and the delivering context (or the next main function) publishes it. `drain` keeps
 * delivering what is recorded during a callback; polled publication stops after one. */
static CAN_CODE void Can_PublishMode(uint8 drain)
{
    uint8 more = 1u;
    while (more != 0u) {
        Can_ControllerStateType mode = CAN_CS_STOPPED;
        more = 0u;
        SchM_Enter_Can_CAN_STATE();
        if ((can_initialized != 0u) && (mode_notification_pending != 0u) &&
            (indication_processing[CAN_INDICATION_MODE] == 0u)) {
            mode = pending_controller_mode;
            mode_notification_pending = 0u;
            indication_processing[CAN_INDICATION_MODE] = 1u;
            more = 1u;
        }
        SchM_Exit_Can_CAN_STATE();
        if (more != 0u) {
            CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, mode);
            SchM_Enter_Can_CAN_STATE();
            indication_processing[CAN_INDICATION_MODE] = 0u;
            SchM_Exit_Can_CAN_STATE();
            more = drain;
        }
    }
}

static CAN_CODE void Can_PublishBusOff(uint8 drain)
{
    uint8 more = 1u;
    while (more != 0u) {
        more = 0u;
        SchM_Enter_Can_CAN_STATE();
        if ((can_initialized != 0u) && (busoff_notification_pending != 0u) &&
            (indication_processing[CAN_INDICATION_BUSOFF] == 0u)) {
            busoff_notification_pending = 0u;
            indication_processing[CAN_INDICATION_BUSOFF] = 1u;
            more = 1u;
        }
        SchM_Exit_Can_CAN_STATE();
        if (more != 0u) {
            CanIf_ControllerBusOff(CAN_CANIF_CONTROLLER_ID);
            SchM_Enter_Can_CAN_STATE();
            indication_processing[CAN_INDICATION_BUSOFF] = 0u;
            SchM_Exit_Can_CAN_STATE();
            more = drain;
        }
    }
}

CAN_CODE void Can_InternalSetMode(CanMode mode)
{
    uint8 notify_mode = 0u;
    uint8 notify_busoff = 0u;
    SchM_Enter_Can_CAN_STATE();
    if (can_initialized == 0u) {
        /* Host mutation requires Can_Init. */
    } else if ((mode != CAN_STARTED) && (mode != CAN_STOPPED) && (mode != CAN_SLEEP) &&
               (mode != CAN_BUS_OFF)) {
        /* Ignore unsupported host mode values without changing a pending indication. */
    } else if ((can_controller_mode == CAN_SLEEP) && (mode != CAN_STOPPED)) {
        /* Logical sleep can only exit via STOPPED. */
    } else if ((busoff_notification_pending != 0u) && (mode != CAN_BUS_OFF)) {
        /* Publish the observed bus-off before an explicit host recovery. */
    } else if (mode == CAN_BUS_OFF) {
        (void)Can_RecordBusOff();
        mode_notification_pending = 0u;
        (void)Can_CancelHostTransmit();
        can_controller_mode = CAN_STOPPED;
        (void)Can_ClearPolledRx();
        can_bus_off = 1u;
        tx_pending = 0u;
        tx_cancelled = 1u;
        tx_confirmation_pending = 0u;
        notify_busoff = 1u;
    } else {
        mode_notification_pending = 0u;
        can_controller_mode = mode;
        can_bus_off = 0u;
        if (mode != CAN_STARTED) {
            (void)Can_ClearPolledRx();
            (void)Can_CancelHostTransmit();
            tx_pending = 0u;
            tx_cancelled = 1u;
            tx_confirmation_pending = 0u;
        }
        if (mode == CAN_STARTED) {
            pending_controller_mode = CAN_CS_STARTED;
            notify_mode = 1u;
        } else if (mode == CAN_STOPPED) {
            pending_controller_mode = CAN_CS_STOPPED;
            notify_mode = 1u;
        } else if (mode == CAN_SLEEP) {
            pending_controller_mode = CAN_CS_SLEEP;
            notify_mode = 1u;
        } else {
            /* Bus-off was handled above; other values have no CanIf mode. */
        }
        mode_notification_pending = notify_mode;
    }
    SchM_Exit_Can_CAN_STATE();
    /* The bus-off and mode branches above are exclusive; a polled bus-off is
     * published by Can_MainFunction_BusOff instead. */
    if (notify_busoff != 0u) {
        (void)Can_PublishImmediateBusOff();
    }
    if (notify_mode != 0u) {
        Can_PublishMode(1u);
    }
}

CAN_CODE void Can_MainFunction_BusOff(void) { Can_PublishBusOff(0u); }

CAN_CODE void Can_MainFunction_Wakeup(void) { Can_PublishMode(0u); }
CAN_CODE void Can_MainFunction_Mode(void) { Can_PublishMode(0u); }

CAN_CODE CanMode Can_InternalGetMode(void)
{
    CanMode mode;
    SchM_Enter_Can_CAN_STATE();
    mode = can_controller_mode;
    if (can_bus_off != 0u) {
        mode = CAN_BUS_OFF;
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

static CAN_CODE void Can_ConfirmTx(void)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (tx_confirmation_pending != 0u) && (tx_confirming == 0u)) {
        const PduIdType handle = tx_confirmation_handle;
        tx_confirmation_pending = 0u;
        tx_confirming = 1u;
        SchM_Exit_Can_CAN_STATE();
        CanIf_TxConfirmation(handle);
        SchM_Enter_Can_CAN_STATE();
        tx_confirming = 0u;
    }
    SchM_Exit_Can_CAN_STATE();
}

static CAN_CODE EcuStatus Can_ReceiveHost(uint32 id, uint8 dlc, const uint8 *data, uint8 accepted)
{
    EcuStatus result = ECU_OK;
    uint8 lost = 0u;
    SchM_Enter_Can_CAN_STATE();
    if (!Can_IsValidId(id)) {
        result = ECU_ERR_FRAME_ID;
    } else if ((!Can_IsWireLength(id, dlc)) ||
               (((id & CAN_ID_FD_FLAG) != 0u) && (fd_tx_bitrate == 0u))) {
        result = ECU_ERR_FRAME_DLC;
    } else if ((can_initialized == 0u) || (can_controller_mode != CAN_STARTED)) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL_PTR) {
        result = ECU_ERR_CONFIG;
    } else if (!Can_HohMatchesId(CAN_RX_ID_TYPE, id)) {
        /* The hardware object ignores identifiers of the other width. */
    } else if ((rx_processing != 0u) || (rx_pending != 0u)) {
        result = ECU_ERR_CAN_BUSY;
        lost = accepted;
        if (accepted != 0u) {
            rx_overrun_pending = 1u;
        }
    } else {
        uint8 i;
        rx_id = id;
        rx_length = dlc;
        for (i = 0u; i < dlc; ++i) {
            rx_payload[i] = data[i];
        }
        rx_pending = 1u;
        result = Can_DeliverImmediateRx();
    }
    SchM_Exit_Can_CAN_STATE();
    if (lost != 0u) {
        (void)Can_ReadImmediateRx();
    }
    return result;
}

CAN_CODE EcuStatus Can_InternalInject(uint32 id, uint8 dlc, const uint8 data[8], uint64 now_ms)
{
    (void)now_ms; /* CanIf_RxIndication carries no reception time. */
    return Can_ReceiveHost(id, dlc, data, 0u);
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
    return Can_ReceiveHost(id, dlc, data, 1u);
}

/* Called with the driver lock held; returns with it held. The result belongs
 * to this snapshot, including when the callback cancels/reinitializes the driver. */
static CAN_CODE EcuStatus Can_DeliverRx(void)
{
    EcuStatus result = ECU_OK;
    if ((can_initialized != 0u) && (rx_processing == 0u) && (rx_pending != 0u)) {
        uint32 id = rx_id;
        uint8 length = rx_length;
        uint8 payload[CAN_FD_MAX_LENGTH] = {0};
        const uint64 lifetime = driver_lifetime;
        const Can_HwType mailbox = {id, CAN_RX_HOH, CAN_CANIF_CONTROLLER_ID};
        PduInfoType info = {payload, NULL_PTR, length};
        uint8 i;
        for (i = 0u; i < length; ++i) {
            payload[i] = rx_payload[i];
        }
        rx_pending = 0u;
        rx_processing = 1u;
        SchM_Exit_Can_CAN_STATE();
        CanIf_RxIndication(&mailbox, &info);
        SchM_Enter_Can_CAN_STATE();
        if ((can_initialized == 0u) || (driver_lifetime != lifetime)) {
            result = ECU_ERR_CONTROLLER;
        }
        rx_processing = 0u;
    }
    return result;
}

static CAN_CODE void Can_ReadRx(void)
{
    uint8 lost;
    SchM_Enter_Can_CAN_STATE();
    lost = rx_overrun_pending;
    rx_overrun_pending = 0u;
    if (lost != 0u) {
        SchM_Exit_Can_CAN_STATE();
        /* SWS_Can_00226: report before Rx can reentrantly replace consumers. */
        (void)Det_ReportRuntimeError(CAN_MODULE_ID, 0u, CAN_SID_MAIN_FUNCTION_READ, CAN_E_DATALOST);
        SchM_Enter_Can_CAN_STATE();
    }
    (void)Can_DeliverRx();
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_MainFunction_Write(void) { Can_ConfirmTx(); }

CAN_CODE void Can_MainFunction_Read(void) { Can_ReadRx(); }

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"

/* Optional actions are selected around complete mapped definitions. */

#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_CancelHostTransmit(void) { host_tx_cancelled_through = host_tx_sequence; }
static CAN_CODE EcuStatus Can_AdmitTransmit(const Can_PduType *pdu, const uint8 *data)
{
    EcuStatus result = ECU_ERR_CAN_BUSY;
    if (host_tx_sequence != UINT64_MAX) {
        ++host_tx_sequence;
        result = Can_StoreTransmit(pdu, data);
    }
    return result;
}
CAN_CODE uint64 Can_InternalHostTransmitToken(void)
{
    uint64 token = UINT64_C(0);
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (tx_in_flight != 0u) &&
        (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED)) {
        token = host_tx_sequence;
    }
    SchM_Exit_Can_CAN_STATE();
    return token;
}

CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64 token)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) &&
        (token > host_tx_cancelled_through) && (token > host_tx_completed_through) &&
        (token <= host_tx_sequence)) {
        if ((tx_confirmation_pending != 0u) || (tx_confirming != 0u)) {
            result = CAN_BUSY;
        } else {
            tx_confirmation_handle = handle;
            tx_confirmation_pending = 1u;
            host_tx_completed_through = token;
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
static CAN_CODE void Can_CancelHostTransmit(void)
{
    /* This configuration has no queued host token to cancel. */
}
static CAN_CODE EcuStatus Can_AdmitTransmit(const Can_PduType *pdu, const uint8 *data)
{
    return Can_StoreTransmit(pdu, data);
}
CAN_CODE uint64 Can_InternalHostTransmitToken(void) { return UINT64_C(0); }
CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64 token)
{
    Std_ReturnType result = E_NOT_OK;
    (void)handle;
    (void)token;
    return result;
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#if (CAN_RX_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ClearPolledRx(void) { rx_pending = 0u; }
static CAN_CODE EcuStatus Can_DeliverImmediateRx(void) { return ECU_OK; }
static CAN_CODE void Can_ReadImmediateRx(void) { /* Polling owns receive publication. */ }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ClearPolledRx(void)
{
    /* Immediate receive has no polling buffer to retire. */
}
static CAN_CODE EcuStatus Can_DeliverImmediateRx(void) { return Can_DeliverRx(); }
static CAN_CODE void Can_ReadImmediateRx(void) { Can_ReadRx(); }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#if (CAN_TX_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ConfirmImmediateTx(void) { /* Polling owns transmit confirmation. */ }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_ConfirmImmediateTx(void) { Can_ConfirmTx(); }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif

#if (CAN_BUSOFF_POLLING == STD_ON)
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_RecordBusOff(void)
{
    if (can_bus_off == 0u) {
        busoff_notification_pending = 1u;
    }
}
static CAN_CODE void Can_PublishImmediateBusOff(void) { /* Polling owns bus-off publication. */ }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#else
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
static CAN_CODE void Can_RecordBusOff(void) { busoff_notification_pending = 1u; }
static CAN_CODE void Can_PublishImmediateBusOff(void) { Can_PublishBusOff(1u); }
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
