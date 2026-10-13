#include "Can_Internal.h"
#include "Det.h"
#include "Can_HostLock.h"
#include "CanIf.h"
#include "SchM_Can.h"
#include "SchM_CanIf.h"
#include "Ecu_Execution.h"
#include <stddef.h>

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
#define CAN_RX_POLLING 0
#endif
#ifndef CAN_HOST_QUEUED_COMPLETION
#define CAN_HOST_QUEUED_COMPLETION 0
#endif
#ifndef CAN_TX_POLLING
#define CAN_TX_POLLING 0
#endif
#ifndef CAN_BUSOFF_POLLING
#define CAN_BUSOFF_POLLING 0
#endif
#ifndef CAN_ZERO_LENGTH_SUPPORTED
#define CAN_ZERO_LENGTH_SUPPORTED 0
#endif

#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

static CanMode can_controller_mode CAN_VAR_CLEARED;
static uint8_t bus_off CAN_VAR_CLEARED;
static uint8_t busoff_notification_pending CAN_VAR_CLEARED;
static CanTxSink tx_sink CAN_VAR_CLEARED;
static uint8_t can_initialized CAN_VAR_CLEARED;
static uint64_t driver_lifetime CAN_VAR_CLEARED;
static uint8_t tx_pending CAN_VAR_CLEARED;
static uint8_t tx_in_flight CAN_VAR_CLEARED;
static uint8_t tx_cancelled CAN_VAR_CLEARED;
static uint32_t tx_id CAN_VAR_CLEARED;
static uint8_t tx_length CAN_VAR_CLEARED;
static uint8_t tx_payload[8] CAN_VAR_CLEARED;
static PduIdType tx_handle CAN_VAR_CLEARED;
static uint8_t tx_confirmation_pending CAN_VAR_CLEARED;
static PduIdType tx_confirmation_handle CAN_VAR_CLEARED;
static uint8_t tx_confirming CAN_VAR_CLEARED;
#if CAN_HOST_QUEUED_COMPLETION == 1
/* Lifetime tokens are never reset by Init: old physical outputs must not
 * complete a new same-handle request after cancellation or reinitialization. */
static uint64_t host_tx_sequence CAN_VAR_CLEARED;
static uint64_t host_tx_cancelled_through CAN_VAR_CLEARED;
static uint64_t host_tx_completed_through CAN_VAR_CLEARED;
#endif
static uint32_t interrupt_disable_count CAN_VAR_CLEARED;
static uint8_t mode_notification_pending CAN_VAR_CLEARED;
static uint8_t mode_notification_processing CAN_VAR_CLEARED;
static Can_ControllerStateType pending_controller_mode CAN_VAR_CLEARED;
static uint8_t rx_pending CAN_VAR_CLEARED;
static uint8_t rx_processing CAN_VAR_CLEARED;
static uint32_t rx_id CAN_VAR_CLEARED;
static uint8_t rx_length CAN_VAR_CLEARED;
static uint8_t rx_payload[8] CAN_VAR_CLEARED;
static uint8_t rx_overrun_pending CAN_VAR_CLEARED;

#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

static CAN_CODE EcuStatus Can_DeliverRx(void);

CAN_CODE void Can_Init(const Can_ConfigType *config) {
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized == 0u) && (config != NULL) &&
        ((Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) || (config->sink != NULL)) &&
        (driver_lifetime != UINT64_MAX)) {
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        ++driver_lifetime;
        tx_sink = config->sink;
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
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
}

CAN_CODE void Can_DeInit(void) {
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized != 0u) && (can_controller_mode != CAN_STARTED)) {
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        can_initialized = 0u;
        tx_cancelled = 1u;
        tx_sink = NULL;
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
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
}

CAN_CODE Std_ReturnType Can_SetBaudrate(uint8_t controller, uint16_t baud_rate_config_id) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (baud_rate_config_id == 0u)) {
        /* The fixed virtual target has one baud-rate configuration and no registers to change. */
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

CAN_CODE Std_ReturnType Can_SetControllerMode(uint8_t controller,
                                              Can_ControllerStateType transition) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (mode_notification_pending == 0u) && (busoff_notification_pending == 0u) &&
        ((transition == CAN_CS_STARTED) || (transition == CAN_CS_STOPPED) ||
         (transition == CAN_CS_SLEEP))) {
        if ((transition == CAN_CS_STARTED) && (can_controller_mode == CAN_STOPPED)) {
            can_controller_mode = CAN_STARTED;
            bus_off = 0u;
            pending_controller_mode = CAN_CS_STARTED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if ((transition == CAN_CS_STOPPED) &&
                   ((can_controller_mode == CAN_STARTED) || (can_controller_mode == CAN_SLEEP))) {
#if CAN_HOST_QUEUED_COMPLETION == 1
            host_tx_cancelled_through = host_tx_sequence;
#endif
            can_controller_mode = CAN_STOPPED;
#if CAN_RX_POLLING == 1
            rx_pending = 0u;
#endif
            tx_pending = 0u;
            tx_cancelled = 1u;
            tx_confirmation_pending = 0u;
            pending_controller_mode = CAN_CS_STOPPED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if ((transition == CAN_CS_SLEEP) &&
                   ((can_controller_mode == CAN_STOPPED) || (can_controller_mode == CAN_SLEEP))) {
            if (can_controller_mode != CAN_SLEEP) {
                can_controller_mode = CAN_SLEEP;
                pending_controller_mode = CAN_CS_SLEEP;
                mode_notification_pending = 1u;
            }
            result = E_OK;
        } else {
            /* Unsupported transition leaves the current state unchanged. */
        }
    }
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerMode(uint8_t controller, Can_ControllerStateType *mode) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) && (mode != NULL)) {
        if (can_controller_mode == CAN_STARTED) {
            *mode = CAN_CS_STARTED;
        } else if (can_controller_mode == CAN_SLEEP) {
            *mode = CAN_CS_SLEEP;
        } else {
            *mode = CAN_CS_STOPPED;
        }
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerErrorState(uint8_t controller,
                                                    Can_ErrorStateType *error_state) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) && (error_state != NULL)) {
        *error_state = (bus_off != 0u) ? CAN_ERRORSTATE_BUSOFF : CAN_ERRORSTATE_ACTIVE;
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

CAN_CODE Std_ReturnType Can_GetControllerRxErrorCounter(uint8_t controller,
                                                        uint8_t *error_counter) {
    (void)controller;
    (void)error_counter;
    return E_NOT_OK;
}

CAN_CODE Std_ReturnType Can_GetControllerTxErrorCounter(uint8_t controller,
                                                        uint8_t *error_counter) {
    (void)controller;
    (void)error_counter;
    return E_NOT_OK;
}

CAN_CODE void Can_DisableControllerInterrupts(uint8_t controller) {
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count < UINT32_MAX)) {
        ++interrupt_disable_count;
    }
    Can_Unlock();
}

CAN_CODE void Can_EnableControllerInterrupts(uint8_t controller) {
    Can_Lock();
    if ((can_initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count != 0u)) {
        --interrupt_disable_count;
    }
    Can_Unlock();
}

CAN_CODE Std_ReturnType Can_CheckWakeup(uint8_t controller) {
    (void)controller;
    return E_NOT_OK;
}

static CAN_CODE EcuStatus Can_WriteHost(Can_HwHandleType hth, const Can_PduType *pdu) {
    EcuStatus result = ECU_ERR_CONFIG;
    if ((hth != CAN_TX_HOH) || (pdu == NULL) || (pdu->sdu == NULL)) {
        /* Invalid host configuration or caller input. */
    } else if (pdu->id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if (pdu->length > 8u) {
        result = ECU_ERR_FRAME_DLC;
#if CAN_ZERO_LENGTH_SUPPORTED == 0
    } else if (pdu->length == 0u) {
        result = ECU_ERR_FRAME_DLC;
#endif
    } else {
        int locked = Can_TryLock();
        if (locked < 0) {
            result = ECU_ERR_CONTROLLER;
        } else if (locked == 0) {
            result = ECU_ERR_CAN_BUSY;
        } else {
            if (can_initialized == 0u) {
                /* Can_Init has not installed a host output callback. */
            } else if (can_controller_mode != CAN_STARTED) {
                result = ECU_ERR_CONTROLLER;
            } else if ((tx_pending != 0u) || (tx_in_flight != 0u) ||
                       (tx_confirmation_pending != 0u) || (tx_confirming != 0u)) {
                result = ECU_ERR_CAN_BUSY;
#if CAN_HOST_QUEUED_COMPLETION == 1
            } else if (host_tx_sequence == UINT64_MAX) {
                result = ECU_ERR_CAN_BUSY;
#endif
            } else {
                size_t i;
#if CAN_HOST_QUEUED_COMPLETION == 1
                ++host_tx_sequence;
#endif
                tx_id = pdu->id;
                tx_handle = pdu->swPduHandle;
                tx_length = pdu->length;
                for (i = 0u; i < (size_t)pdu->length; ++i) {
                    tx_payload[i] = pdu->sdu[i];
                }
                tx_pending = 1u;
                result = ECU_OK;
            }
            Can_Unlock();
        }
    }
    return result;
}

CAN_CODE Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu) {
    Std_ReturnType result = E_NOT_OK;
    EcuStatus host_result = Can_WriteHost(hth, pdu);
    if (host_result == ECU_OK) {
        result = E_OK;
    } else if (host_result == ECU_ERR_CAN_BUSY) {
        result = CAN_BUSY;
    } else {
        /* Invalid request retains E_NOT_OK. */
    }
    return result;
}

CAN_CODE EcuStatus Can_InternalHostFlush(void) {
    EcuStatus result = ECU_OK;
    Can_Lock();
    if (can_initialized == 0u) {
        result = ECU_ERR_CONFIG;
    } else if ((tx_pending != 0u) && (tx_in_flight == 0u)) {
        const uint32_t id = tx_id;
        const uint8_t length = tx_length;
        const PduIdType handle = tx_handle;
        const CanTxSink sink = tx_sink;
        const uint64_t lifetime = driver_lifetime;
        uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        size_t i;
        for (i = 0u; i < (size_t)length; ++i) {
            payload[i] = tx_payload[i];
        }
        tx_pending = 0u;
        tx_in_flight = 1u;
        tx_cancelled = 0u;
        Can_Unlock();
        result = Ecu_ExecutionTransmit(sink, handle, id, length, payload);
        Can_Lock();
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
    Can_Unlock();
#if CAN_TX_POLLING == 0
    Can_MainFunction_Write();
#endif
    return result;
}

CAN_CODE void Can_InternalSetMode(CanMode mode) {
    uint8_t notify_mode = 0u;
#if CAN_BUSOFF_POLLING == 0
    uint8_t notify_busoff = 0u;
#endif
    Can_ControllerStateType indication = CAN_CS_STOPPED;
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
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
#if CAN_BUSOFF_POLLING == 1
        if (bus_off == 0u) {
            busoff_notification_pending = 1u;
        }
#endif
        mode_notification_pending = 0u;
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        can_controller_mode = CAN_STOPPED;
#if CAN_RX_POLLING == 1
        rx_pending = 0u;
#endif
        bus_off = 1u;
        tx_pending = 0u;
        tx_cancelled = 1u;
        tx_confirmation_pending = 0u;
#if CAN_BUSOFF_POLLING == 0
        notify_busoff = 1u;
#endif
    } else {
        mode_notification_pending = 0u;
        can_controller_mode = mode;
        bus_off = 0u;
        if (mode != CAN_STARTED) {
#if CAN_RX_POLLING == 1
            rx_pending = 0u;
#endif
#if CAN_HOST_QUEUED_COMPLETION == 1
            host_tx_cancelled_through = host_tx_sequence;
#endif
            tx_pending = 0u;
            tx_cancelled = 1u;
            tx_confirmation_pending = 0u;
        }
        if (mode == CAN_STARTED) {
            indication = CAN_CS_STARTED;
            notify_mode = 1u;
        } else if (mode == CAN_STOPPED) {
            indication = CAN_CS_STOPPED;
            notify_mode = 1u;
        } else if (mode == CAN_SLEEP) {
            indication = CAN_CS_SLEEP;
            notify_mode = 1u;
        } else {
            /* Bus-off was handled above; other values have no CanIf mode. */
        }
    }
    Can_Unlock();
    /* The bus-off and mode branches above are exclusive; a polled bus-off is
     * published by Can_MainFunction_BusOff instead. */
#if CAN_BUSOFF_POLLING == 0
    if (notify_busoff != 0u) {
        CanIf_ControllerBusOff(CAN_CANIF_CONTROLLER_ID);
    }
#endif
    if (notify_mode != 0u) {
        CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, indication);
    }
    SchM_Exit_CanIf_CANIF_STATE();
}

CAN_CODE void Can_MainFunction_BusOff(void) {
    uint8_t notify = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized != 0u) && (busoff_notification_pending != 0u)) {
        busoff_notification_pending = 0u;
        notify = 1u;
    }
    Can_Unlock();
    if (notify != 0u) {
        CanIf_ControllerBusOff(CAN_CANIF_CONTROLLER_ID);
    }
    SchM_Exit_CanIf_CANIF_STATE();
}

static CAN_CODE void Can_PollMode(void) {
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized != 0u) && (mode_notification_pending != 0u) &&
        (mode_notification_processing == 0u)) {
        const Can_ControllerStateType mode = pending_controller_mode;
        mode_notification_pending = 0u;
        mode_notification_processing = 1u;
        Can_Unlock();
        CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, mode);
        Can_Lock();
        mode_notification_processing = 0u;
    }
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
}

CAN_CODE void Can_MainFunction_Wakeup(void) { Can_PollMode(); }
CAN_CODE void Can_MainFunction_Mode(void) { Can_PollMode(); }

CAN_CODE CanMode Can_InternalGetMode(void) {
    CanMode mode;
    Can_Lock();
    mode = can_controller_mode;
    if (bus_off != 0u) {
        mode = CAN_BUS_OFF;
    }
    Can_Unlock();
    return mode;
}

CAN_CODE EcuStatus Can_InternalTransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc,
                                           const uint8_t data[8]) {
    uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    Can_PduType pdu;
    EcuStatus result = ECU_ERR_CONFIG;
    if (data != NULL) {
        size_t i;
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

CAN_CODE uint64_t Can_InternalHostTransmitToken(void) {
    uint64_t token = UINT64_C(0);
#if CAN_HOST_QUEUED_COMPLETION == 1
    Can_Lock();
    if ((can_initialized != 0u) && (tx_in_flight != 0u) &&
        (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED)) {
        token = host_tx_sequence;
    }
    Can_Unlock();
#endif
    return token;
}

CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64_t token) {
    Std_ReturnType result = E_NOT_OK;
#if CAN_HOST_QUEUED_COMPLETION == 1
    Can_Lock();
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
    Can_Unlock();
#else
    (void)handle;
    (void)token;
#endif
    return result;
}

CAN_CODE void Can_MainFunction_Write(void) {
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if ((can_initialized != 0u) && (tx_confirmation_pending != 0u) && (tx_confirming == 0u)) {
        const PduIdType handle = tx_confirmation_handle;
        tx_confirmation_pending = 0u;
        tx_confirming = 1u;
        Can_Unlock();
        CanIf_TxConfirmation(handle);
        Can_Lock();
        tx_confirming = 0u;
    }
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
}

static CAN_CODE EcuStatus Can_ReceiveHost(uint32_t id, uint8_t dlc, const uint8_t data[8],
                                          uint8_t accepted) {
    EcuStatus result = ECU_OK;
    uint8_t lost = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    if (id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if (dlc > 8u) {
        result = ECU_ERR_FRAME_DLC;
#if CAN_ZERO_LENGTH_SUPPORTED == 0
    } else if (dlc == 0u) {
        result = ECU_ERR_FRAME_DLC;
#endif
    } else if ((can_initialized == 0u) || (can_controller_mode != CAN_STARTED)) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL) {
        result = ECU_ERR_CONFIG;
    } else if ((rx_processing != 0u) || (rx_pending != 0u)) {
        result = ECU_ERR_CAN_BUSY;
        lost = accepted;
        if (accepted != 0u) {
            rx_overrun_pending = 1u;
        }
    } else {
        size_t i;
        rx_id = id;
        rx_length = dlc;
        for (i = 0u; i < (size_t)dlc; ++i) {
            rx_payload[i] = data[i];
        }
        rx_pending = 1u;
#if CAN_RX_POLLING == 0
        result = Can_DeliverRx();
#endif
    }
    Can_Unlock();
#if CAN_RX_POLLING == 0
    if (lost != 0u) {
        Can_MainFunction_Read();
    }
#else
    (void)lost;
#endif
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}

CAN_CODE EcuStatus Can_InternalInject(uint32_t id, uint8_t dlc, const uint8_t data[8],
                                      uint64_t now_ms) {
    (void)now_ms; /* CanIf_RxIndication carries no reception time. */
    return Can_ReceiveHost(id, dlc, data, 0u);
}

CAN_CODE EcuStatus Can_InternalRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8],
                                            uint64_t now_ms) {
    /* Already received by a platform adapter: occupied shadow storage is a real overrun.
     * Host admission uses Can_InternalInject and remains retryable without a lost-frame report. */
    (void)now_ms;
    return Can_ReceiveHost(id, dlc, data, 1u);
}

/* Called with the driver lock held; returns with it held. The result belongs
 * to this snapshot, including when the callback cancels/reinitializes the driver. */
static CAN_CODE EcuStatus Can_DeliverRx(void) {
    EcuStatus result = ECU_OK;
    if ((can_initialized != 0u) && (rx_processing == 0u) && (rx_pending != 0u)) {
        uint32_t id = rx_id;
        uint8_t length = rx_length;
        uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        const uint64_t lifetime = driver_lifetime;
        const Can_HwType mailbox = {id, CAN_RX_HOH, CAN_CANIF_CONTROLLER_ID};
        PduInfoType info = {payload, NULL_PTR, length};
        size_t i;
        for (i = 0u; i < (size_t)length; ++i) {
            payload[i] = rx_payload[i];
        }
        rx_pending = 0u;
        rx_processing = 1u;
        Can_Unlock();
        CanIf_RxIndication(&mailbox, &info);
        Can_Lock();
        if ((can_initialized == 0u) || (driver_lifetime != lifetime)) {
            result = ECU_ERR_CONTROLLER;
        }
        rx_processing = 0u;
    }
    return result;
}

CAN_CODE void Can_MainFunction_Read(void) {
    uint8_t lost;
    SchM_Enter_CanIf_CANIF_STATE();
    Can_Lock();
    lost = rx_overrun_pending;
    rx_overrun_pending = 0u;
    if (lost != 0u) {
        Can_Unlock();
        /* SWS_Can_00226: report before Rx can reentrantly replace consumers. */
        (void)Det_ReportRuntimeError(CAN_MODULE_ID, 0u, 0x08u, CAN_E_DATALOST);
        Can_Lock();
    }
    (void)Can_DeliverRx();
    Can_Unlock();
    SchM_Exit_CanIf_CANIF_STATE();
}

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
