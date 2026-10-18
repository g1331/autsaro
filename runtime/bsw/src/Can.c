/** @file Standard Can driver for one Classical CAN controller on the controlled host. */
#include "Can_Internal.h"
#include "Ecu_ProfileConfig.h"
#include "Det.h"
#include "CanIf.h"
#include "SchM_Can.h"
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

/* Can_IdType frame-type bit 30 marks CAN FD; standard IDs use bits 0-10 (SWS_Can_00416). */
#define CAN_ID_FD_FLAG      0x40000000u
#define CAN_ID_STANDARD_MAX 0x7FFu

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
static uint8 can_initialized CAN_VAR_CLEARED;             /**< 1 between Init and DeInit. */
static uint64 driver_lifetime CAN_VAR_CLEARED;            /**< Successful Init count. */
static Can_TxSinkType tx_sink CAN_VAR_CLEARED;            /**< Host output; set by Init. */
static CanMode can_controller_mode CAN_VAR_CLEARED;       /**< Controller state; Init: STOPPED. */
static uint8 bus_off CAN_VAR_CLEARED;                     /**< In bus-off; Init resets. */
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
static uint32 tx_id CAN_VAR_CLEARED;                     /**< Standard identifier. */
static uint8 tx_length CAN_VAR_CLEARED;                  /**< Length, 0 to 8 bytes. */
static uint8 tx_payload[8] CAN_VAR_CLEARED;              /**< tx_length bytes valid. */
static PduIdType tx_handle CAN_VAR_CLEARED;              /**< swPduHandle of the frame. */
static PduIdType tx_confirmation_handle CAN_VAR_CLEARED; /**< swPduHandle to confirm. */
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
/* Lifetime tokens are never reset by Init: old physical outputs must not
 * complete a new same-handle request after cancellation or reinitialization. */
static uint64 host_tx_sequence CAN_VAR_CLEARED;          /**< Token of the last admitted frame. */
static uint64 host_tx_cancelled_through CAN_VAR_CLEARED; /**< Tokens up to here never complete. */
static uint64 host_tx_completed_through CAN_VAR_CLEARED; /**< Tokens up to here have completed. */
#endif

static uint32 interrupt_disable_count CAN_VAR_CLEARED; /**< Disable nesting; Init resets. */

/* Mode and bus-off indications. mode_notification_pending is set while pending_controller_mode
 * still has to be given to CanIf; Init resets it. indication_processing[CAN_INDICATION_*] is 1
 * while that kind is being delivered; it is never reset, so it outlives a reinitialization. */
static Can_ControllerStateType pending_controller_mode CAN_VAR_CLEARED; /**< Mode to indicate. */
static uint8 mode_notification_pending CAN_VAR_CLEARED;                 /**< Mode not yet sent. */
static uint8 indication_processing[2] CAN_VAR_CLEARED;                  /**< Kind in delivery. */

/* Shadow receive buffer: rx_id, rx_length and rx_payload are valid while rx_pending. */
static uint8 rx_pending CAN_VAR_CLEARED;         /**< Frame buffered; Init resets. */
static uint8 rx_processing CAN_VAR_CLEARED;      /**< CanIf_RxIndication running. */
static uint32 rx_id CAN_VAR_CLEARED;             /**< Standard identifier. */
static uint8 rx_length CAN_VAR_CLEARED;          /**< Length, 0 to 8 bytes. */
static uint8 rx_payload[8] CAN_VAR_CLEARED;      /**< rx_length bytes valid. */
static uint8 rx_overrun_pending CAN_VAR_CLEARED; /**< CAN_E_DATALOST to report; Init resets. */

#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

static CAN_CODE EcuStatus Can_DeliverRx(void);
/* Scheduled-function bodies, also run on direct (non-polled) events (SWS_BSW_00133). */
static CAN_CODE void Can_ConfirmTx(void);
static CAN_CODE void Can_ReadRx(void);

CAN_CODE void Can_Init(const Can_ConfigType *Config)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized == 0u) && (Config != NULL_PTR) &&
        ((Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) || (Config->sink != NULL_PTR)) &&
        (driver_lifetime != UINT64_MAX)) {
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
        host_tx_cancelled_through = host_tx_sequence;
#endif
        ++driver_lifetime;
        tx_sink = Config->sink;
        can_initialized = 1u;
        can_controller_mode = CAN_STOPPED;
        bus_off = 0u;
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
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
        host_tx_cancelled_through = host_tx_sequence;
#endif
        can_initialized = 0u;
        tx_cancelled = 1u;
        tx_sink = NULL_PTR;
        can_controller_mode = CAN_STOPPED;
        bus_off = 0u;
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
            bus_off = 0u;
            pending_controller_mode = CAN_CS_STARTED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if (Transition == CAN_CS_STOPPED) {
            /* The state machine also loops STOPPED to STOPPED; SLEEP to STOPPED leaves the
             * logical sleep without touching the already stopped controller (SWS_Can_00267). */
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
            host_tx_cancelled_through = host_tx_sequence;
#endif
            can_controller_mode = CAN_STOPPED;
#if (CAN_RX_POLLING == STD_ON)
            rx_pending = 0u;
#endif
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
        *ErrorStatePtr = (bus_off != 0u) ? CAN_ERRORSTATE_BUSOFF : CAN_ERRORSTATE_ACTIVE;
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

CAN_CODE void Can_DisableControllerInterrupts(uint8 Controller)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count < UINT32_MAX)) {
        ++interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_EnableControllerInterrupts(uint8 Controller)
{
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (Controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count != 0u)) {
        --interrupt_disable_count;
    }
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE Std_ReturnType Can_CheckWakeup(uint8 Controller)
{
    (void)Controller;
    return E_NOT_OK;
}

static CAN_CODE EcuStatus Can_WriteHost(Can_HwHandleType hth, const Can_PduType *pdu)
{
    EcuStatus result = ECU_ERR_CONFIG;
    if ((hth != CAN_TX_HOH) || (pdu == NULL_PTR) || (pdu->sdu == NULL_PTR)) {
        /* Invalid host configuration or caller input. */
    } else if ((pdu->id & ~CAN_ID_FD_FLAG) > CAN_ID_STANDARD_MAX) {
        /* No FD configuration: the FD bit is not evaluated (SWS_Can_00486). */
        result = ECU_ERR_FRAME_ID;
    } else if (pdu->length > 8u) {
        result = ECU_ERR_FRAME_DLC;
    } else {
        /* The area guards data only (ADR 0018), so entering it is bounded and keeps
         * Can_Write non-blocking (SWS_Can_00275); CAN_BUSY means an occupied slot. */
        SchM_Enter_Can_CAN_STATE();
        if (can_initialized == 0u) {
            /* Can_Init has not installed a host output callback. */
        } else if (can_controller_mode != CAN_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else if ((tx_pending != 0u) || (tx_in_flight != 0u) || (tx_confirmation_pending != 0u) ||
                   (tx_confirming != 0u)) {
            result = ECU_ERR_CAN_BUSY;
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
        } else if (host_tx_sequence == UINT64_MAX) {
            result = ECU_ERR_CAN_BUSY;
#endif
        } else {
            uint8 i;
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
            ++host_tx_sequence;
#endif
            tx_id = pdu->id & ~CAN_ID_FD_FLAG;
            tx_handle = pdu->swPduHandle;
            tx_length = pdu->length;
            for (i = 0u; i < pdu->length; ++i) {
                tx_payload[i] = pdu->sdu[i];
            }
            tx_pending = 1u;
            result = ECU_OK;
        }
        SchM_Exit_Can_CAN_STATE();
    }
    return result;
}

CAN_CODE Std_ReturnType Can_Write(Can_HwHandleType Hth, const Can_PduType *PduInfo)
{
    Std_ReturnType result = E_NOT_OK;
    EcuStatus host_result = Can_WriteHost(Hth, PduInfo);
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
        const PduIdType handle = tx_handle;
        const Can_TxSinkType sink = tx_sink;
        const uint64 lifetime = driver_lifetime;
        uint8 payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        uint8 i;
        for (i = 0u; i < length; ++i) {
            payload[i] = tx_payload[i];
        }
        tx_pending = 0u;
        tx_in_flight = 1u;
        tx_cancelled = 0u;
        SchM_Exit_Can_CAN_STATE();
        result = Ecu_ExecutionTransmit(sink, handle, id, length, payload);
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
#if !(CAN_TX_POLLING == STD_ON)
    Can_ConfirmTx();
#endif
    return result;
}

/* Mode and bus-off indications leave the driver one context at a time, in the order
 * they were recorded (ADR 0018): a context that finds one in delivery only records,
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
#if !(CAN_BUSOFF_POLLING == STD_ON)
    uint8 notify_busoff = 0u;
#endif
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
#if (CAN_BUSOFF_POLLING == STD_ON)
        if (bus_off == 0u) {
            busoff_notification_pending = 1u;
        }
#else
        busoff_notification_pending = 1u;
#endif
        mode_notification_pending = 0u;
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
        host_tx_cancelled_through = host_tx_sequence;
#endif
        can_controller_mode = CAN_STOPPED;
#if (CAN_RX_POLLING == STD_ON)
        rx_pending = 0u;
#endif
        bus_off = 1u;
        tx_pending = 0u;
        tx_cancelled = 1u;
        tx_confirmation_pending = 0u;
#if !(CAN_BUSOFF_POLLING == STD_ON)
        notify_busoff = 1u;
#endif
    } else {
        mode_notification_pending = 0u;
        can_controller_mode = mode;
        bus_off = 0u;
        if (mode != CAN_STARTED) {
#if (CAN_RX_POLLING == STD_ON)
            rx_pending = 0u;
#endif
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
            host_tx_cancelled_through = host_tx_sequence;
#endif
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
#if !(CAN_BUSOFF_POLLING == STD_ON)
    if (notify_busoff != 0u) {
        Can_PublishBusOff(1u);
    }
#endif
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
    if (bus_off != 0u) {
        mode = CAN_BUS_OFF;
    }
    SchM_Exit_Can_CAN_STATE();
    return mode;
}

CAN_CODE EcuStatus Can_InternalTransmitPdu(PduIdType pdu_id,
                                           uint32 id,
                                           uint8 dlc,
                                           const uint8 data[8])
{
    uint8 payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    Can_PduType pdu;
    EcuStatus result = ECU_ERR_CONFIG;
    if (data != NULL_PTR) {
        uint8 i;
        for (i = 0u; (i < dlc) && (i < sizeof(payload)); ++i) {
            payload[i] = data[i];
        }
        pdu.swPduHandle = pdu_id;
        pdu.length = dlc;
        pdu.id = id;
        pdu.sdu = payload;
        result = Can_WriteHost(CAN_TX_HOH, &pdu);
        if (result == ECU_OK) {
            result = Can_InternalHostFlush();
        }
    }
    return result;
}

CAN_CODE uint64 Can_InternalHostTransmitToken(void)
{
    uint64 token = UINT64_C(0);
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
    SchM_Enter_Can_CAN_STATE();
    if ((can_initialized != 0u) && (tx_in_flight != 0u) &&
        (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED)) {
        token = host_tx_sequence;
    }
    SchM_Exit_Can_CAN_STATE();
#endif
    return token;
}

CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64 token)
{
    Std_ReturnType result = E_NOT_OK;
#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)
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
#else
    (void)handle;
    (void)token;
#endif
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

static CAN_CODE EcuStatus Can_ReceiveHost(uint32 id, uint8 dlc, const uint8 data[8], uint8 accepted)
{
    EcuStatus result = ECU_OK;
    uint8 lost = 0u;
    SchM_Enter_Can_CAN_STATE();
    if (id > CAN_ID_STANDARD_MAX) {
        result = ECU_ERR_FRAME_ID;
    } else if (dlc > 8u) {
        result = ECU_ERR_FRAME_DLC;
    } else if ((can_initialized == 0u) || (can_controller_mode != CAN_STARTED)) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL_PTR) {
        result = ECU_ERR_CONFIG;
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
#if !(CAN_RX_POLLING == STD_ON)
        result = Can_DeliverRx();
#endif
    }
    SchM_Exit_Can_CAN_STATE();
#if !(CAN_RX_POLLING == STD_ON)
    if (lost != 0u) {
        Can_ReadRx();
    }
#else
    (void)lost;
#endif
    return result;
}

CAN_CODE EcuStatus Can_InternalInject(uint32 id, uint8 dlc, const uint8 data[8], uint64 now_ms)
{
    (void)now_ms; /* CanIf_RxIndication carries no reception time. */
    return Can_ReceiveHost(id, dlc, data, 0u);
}

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
        uint8 payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
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
        (void)Det_ReportRuntimeError(CAN_MODULE_ID, 0u, 0x08u, CAN_E_DATALOST);
        SchM_Enter_Can_CAN_STATE();
    }
    (void)Can_DeliverRx();
    SchM_Exit_Can_CAN_STATE();
}

CAN_CODE void Can_MainFunction_Write(void) { Can_ConfirmTx(); }

CAN_CODE void Can_MainFunction_Read(void) { Can_ReadRx(); }

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
