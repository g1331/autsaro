#include "Can.h"
#include "Can_HostLock.h"
#include "CanIf.h"
#include "SchM_Can.h"
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

static CanMode controller_mode;
static uint8_t bus_off;
static uint8_t busoff_notification_pending;
static CanTxSink tx_sink;
static uint8_t initialized;
static uint8_t tx_pending;
static uint8_t tx_in_flight;
static uint32_t tx_id;
static uint8_t tx_length;
static uint8_t tx_payload[8];
static PduIdType tx_handle;
static uint8_t tx_confirmation_pending;
static PduIdType tx_confirmation_handle;
static uint8_t tx_confirming;
#if CAN_HOST_QUEUED_COMPLETION == 1
/* Lifetime tokens are never reset by Init: old physical outputs must not
 * complete a new same-handle request after cancellation or reinitialization. */
static uint64_t host_tx_sequence;
static uint64_t host_tx_cancelled_through;
static uint64_t host_tx_completed_through;
#endif
static uint32_t interrupt_disable_count;
static uint8_t mode_notification_pending;
static uint8_t mode_notification_processing;
static Can_ControllerStateType pending_controller_mode;
static uint8_t rx_pending;
static uint8_t rx_processing;
static uint32_t rx_id;
static uint8_t rx_length;
static uint8_t rx_payload[8];
static uint64_t rx_time_ms;
static EcuStatus rx_result;

#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Can_MemMap.h"

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

void Can_Init(const Can_ConfigType *config) {
    Can_Lock();
    if ((initialized == 0u) && (config != NULL) &&
        ((Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) || (config->sink != NULL))) {
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        tx_sink = config->sink;
        initialized = 1u;
        controller_mode = CAN_STOPPED;
        bus_off = 0u;
        busoff_notification_pending = 0u;
        tx_pending = 0u;
        tx_in_flight = 0u;
        tx_confirmation_pending = 0u;
        tx_confirming = 0u;
        rx_pending = 0u;
        rx_processing = 0u;
        interrupt_disable_count = 0u;
        mode_notification_pending = 0u;
        mode_notification_processing = 0u;
    }
    Can_Unlock();
}

void Can_DeInit(void) {
    Can_Lock();
    if ((initialized != 0u) && (controller_mode != CAN_STARTED) &&
        (mode_notification_pending == 0u) && (busoff_notification_pending == 0u)) {
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        initialized = 0u;
        tx_sink = NULL;
        controller_mode = CAN_STOPPED;
        bus_off = 0u;
        busoff_notification_pending = 0u;
        tx_pending = 0u;
        tx_in_flight = 0u;
        tx_confirmation_pending = 0u;
        tx_confirming = 0u;
        rx_pending = 0u;
        rx_processing = 0u;
        interrupt_disable_count = 0u;
        mode_notification_pending = 0u;
        mode_notification_processing = 0u;
    }
    Can_Unlock();
}

Std_ReturnType Can_SetBaudrate(uint8_t controller, uint16_t baud_rate_config_id) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) && (baud_rate_config_id == 0u)) {
        /* The fixed virtual target has one baud-rate configuration and no registers to change. */
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_SetControllerMode(uint8_t controller, Can_ControllerStateType transition) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (mode_notification_pending == 0u) && (busoff_notification_pending == 0u) &&
        ((transition == CAN_CS_STARTED) || (transition == CAN_CS_STOPPED) ||
         (transition == CAN_CS_SLEEP))) {
        if ((transition == CAN_CS_STARTED) && (controller_mode == CAN_STOPPED)) {
            controller_mode = CAN_STARTED;
            bus_off = 0u;
            pending_controller_mode = CAN_CS_STARTED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if ((transition == CAN_CS_STOPPED) &&
                   ((controller_mode == CAN_STARTED) || (controller_mode == CAN_SLEEP))) {
#if CAN_HOST_QUEUED_COMPLETION == 1
            host_tx_cancelled_through = host_tx_sequence;
#endif
            controller_mode = CAN_STOPPED;
#if CAN_RX_POLLING == 1
            rx_pending = 0u;
#endif
            tx_pending = 0u;
            pending_controller_mode = CAN_CS_STOPPED;
            mode_notification_pending = 1u;
            result = E_OK;
        } else if ((transition == CAN_CS_SLEEP) &&
                   ((controller_mode == CAN_STOPPED) || (controller_mode == CAN_SLEEP))) {
            if (controller_mode != CAN_SLEEP) {
                controller_mode = CAN_SLEEP;
                pending_controller_mode = CAN_CS_SLEEP;
                mode_notification_pending = 1u;
            }
            result = E_OK;
        } else {
            /* Unsupported transition leaves the current state unchanged. */
        }
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_GetControllerMode(uint8_t controller, Can_ControllerStateType *mode) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) && (mode != NULL)) {
        if (controller_mode == CAN_STARTED) {
            *mode = CAN_CS_STARTED;
        } else if (controller_mode == CAN_SLEEP) {
            *mode = CAN_CS_SLEEP;
        } else {
            *mode = CAN_CS_STOPPED;
        }
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_GetControllerErrorState(uint8_t controller, Can_ErrorStateType *error_state) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) && (error_state != NULL)) {
        *error_state = (bus_off != 0u) ? CAN_ERRORSTATE_BUSOFF : CAN_ERRORSTATE_ACTIVE;
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_GetControllerRxErrorCounter(uint8_t controller, uint8_t *error_counter) {
    (void)controller;
    (void)error_counter;
    return E_NOT_OK;
}

Std_ReturnType Can_GetControllerTxErrorCounter(uint8_t controller, uint8_t *error_counter) {
    (void)controller;
    (void)error_counter;
    return E_NOT_OK;
}

void Can_DisableControllerInterrupts(uint8_t controller) {
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count < UINT32_MAX)) {
        ++interrupt_disable_count;
    }
    Can_Unlock();
}

void Can_EnableControllerInterrupts(uint8_t controller) {
    Can_Lock();
    if ((initialized != 0u) && (controller == CAN_CONTROLLER_ID) &&
        (interrupt_disable_count != 0u)) {
        --interrupt_disable_count;
    }
    Can_Unlock();
}

Std_ReturnType Can_CheckWakeup(uint8_t controller) {
    (void)controller;
    return E_NOT_OK;
}

static EcuStatus Can_WriteHost(Can_HwHandleType hth, const Can_PduType *pdu) {
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
            if (initialized == 0u) {
                /* Can_Init has not installed a host output callback. */
            } else if (controller_mode != CAN_STARTED) {
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

Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu) {
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

EcuStatus Can_HostFlush(void) {
    EcuStatus result = ECU_OK;
    Can_Lock();
    if (tx_pending != 0u) {
        uint32_t id = tx_id;
        uint8_t length = tx_length;
        uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        size_t i;
        for (i = 0u; i < (size_t)length; ++i) {
            payload[i] = tx_payload[i];
        }
        tx_pending = 0u;
        tx_in_flight = 1u;
        result = Ecu_ExecutionTransmit(tx_sink, tx_handle, id, length, payload);
        tx_in_flight = 0u;
        if ((result == ECU_OK) && (Ecu_Policy.tx_confirmation == ECU_TX_SYNCHRONOUS)) {
            tx_confirmation_handle = tx_handle;
            tx_confirmation_pending = 1u;
#if CAN_TX_POLLING == 0
            Can_MainFunction_Write();
#endif
        }
    }
    Can_Unlock();
    return result;
}

void Can_SetMode(CanMode mode) {
    Can_Lock();
    if ((mode != CAN_STARTED) && (mode != CAN_STOPPED) && (mode != CAN_SLEEP) &&
        (mode != CAN_BUS_OFF)) {
        /* Ignore unsupported host mode values without changing a pending indication. */
    } else if ((controller_mode == CAN_SLEEP) && (mode != CAN_STOPPED)) {
        /* Logical sleep can only exit via STOPPED. */
    } else if ((busoff_notification_pending != 0u) && (mode != CAN_BUS_OFF)) {
        /* Publish the observed bus-off before an explicit host recovery. */
    } else if (mode == CAN_BUS_OFF) {
#if CAN_BUSOFF_POLLING == 1
        if ((initialized != 0u) && (bus_off == 0u)) {
            busoff_notification_pending = 1u;
        }
#endif
        mode_notification_pending = 0u;
#if CAN_HOST_QUEUED_COMPLETION == 1
        host_tx_cancelled_through = host_tx_sequence;
#endif
        controller_mode = CAN_STOPPED;
#if CAN_RX_POLLING == 1
        rx_pending = 0u;
#endif
        bus_off = 1u;
        tx_pending = 0u;
#if CAN_BUSOFF_POLLING == 0
        CanIf_ControllerBusOff(CAN_CANIF_CONTROLLER_ID);
#endif
    } else {
        mode_notification_pending = 0u;
        controller_mode = mode;
        bus_off = 0u;
        if (mode != CAN_STARTED) {
#if CAN_RX_POLLING == 1
            rx_pending = 0u;
#endif
#if CAN_HOST_QUEUED_COMPLETION == 1
            host_tx_cancelled_through = host_tx_sequence;
#endif
            tx_pending = 0u;
        }
        if (mode == CAN_STARTED) {
            CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, CAN_CS_STARTED);
        } else if (mode == CAN_STOPPED) {
            CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, CAN_CS_STOPPED);
        } else if (mode == CAN_SLEEP) {
            CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, CAN_CS_SLEEP);
        } else {
            /* Bus-off was handled above; other values have no CanIf mode. */
        }
    }
    Can_Unlock();
}

void Can_MainFunction_BusOff(void) {
    Can_Lock();
    if (busoff_notification_pending != 0u) {
        busoff_notification_pending = 0u;
        CanIf_ControllerBusOff(CAN_CANIF_CONTROLLER_ID);
    }
    Can_Unlock();
}

static void Can_PollMode(void) {
    Can_Lock();
    if ((mode_notification_pending != 0u) && (mode_notification_processing == 0u)) {
        Can_ControllerStateType mode = pending_controller_mode;
        mode_notification_pending = 0u;
        mode_notification_processing = 1u;
        CanIf_ControllerModeIndication(CAN_CANIF_CONTROLLER_ID, mode);
        mode_notification_processing = 0u;
    }
    Can_Unlock();
}

void Can_MainFunction_Wakeup(void) { Can_PollMode(); }
void Can_MainFunction_Mode(void) { Can_PollMode(); }

CanMode Can_GetMode(void) {
    CanMode mode;
    Can_Lock();
    mode = controller_mode;
    if (bus_off != 0u) {
        mode = CAN_BUS_OFF;
    }
    Can_Unlock();
    return mode;
}

EcuStatus Can_TransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc, const uint8_t data[8]) {
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
            result = Can_HostFlush();
        }
    }
    return result;
}

uint64_t Can_HostTransmitToken(void) {
    uint64_t token = UINT64_C(0);
#if CAN_HOST_QUEUED_COMPLETION == 1
    Can_Lock();
    if ((initialized != 0u) && (tx_in_flight != 0u) &&
        (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED)) {
        token = host_tx_sequence;
    }
    Can_Unlock();
#endif
    return token;
}

Std_ReturnType Can_HostCompleteTransmit(PduIdType handle, uint64_t token) {
    Std_ReturnType result = E_NOT_OK;
#if CAN_HOST_QUEUED_COMPLETION == 1
    Can_Lock();
    if ((initialized != 0u) && (Ecu_Policy.tx_confirmation == ECU_TX_QUEUED) &&
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

void Can_MainFunction_Write(void) {
    Can_Lock();
    if ((tx_confirmation_pending != 0u) && (tx_confirming == 0u)) {
        PduIdType handle = tx_confirmation_handle;
        tx_confirmation_pending = 0u;
        tx_confirming = 1u;
        CanIf_TxConfirmation(handle);
        tx_confirming = 0u;
    }
    Can_Unlock();
}

EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    Can_Lock();
    if (id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if (dlc > 8u) {
        result = ECU_ERR_FRAME_DLC;
#if CAN_ZERO_LENGTH_SUPPORTED == 0
    } else if (dlc == 0u) {
        result = ECU_ERR_FRAME_DLC;
#endif
    } else if (controller_mode != CAN_STARTED) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL) {
        result = ECU_ERR_CONFIG;
    } else if ((rx_processing != 0u) || (rx_pending != 0u)) {
        result = ECU_ERR_CAN_BUSY;
    } else {
        size_t i;
        rx_id = id;
        rx_length = dlc;
        rx_time_ms = now_ms;
        for (i = 0u; i < (size_t)dlc; ++i) {
            rx_payload[i] = data[i];
        }
        rx_pending = 1u;
#if CAN_RX_POLLING == 0
        Can_MainFunction_Read();
        result = rx_result;
#endif
    }
    Can_Unlock();
    return result;
}

void Can_MainFunction_Read(void) {
    Can_Lock();
    if ((rx_processing == 0u) && (rx_pending != 0u)) {
        uint32_t id = rx_id;
        uint8_t length = rx_length;
        uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
        uint64_t now_ms = rx_time_ms;
        size_t i;
        for (i = 0u; i < (size_t)length; ++i) {
            payload[i] = rx_payload[i];
        }
        rx_pending = 0u;
        rx_processing = 1u;
        rx_result = CanIf_HostRxIndication(id, length, payload, now_ms);
        rx_processing = 0u;
    }
    Can_Unlock();
}

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
