#include "Can.h"
#include "Can_HostLock.h"
#include "CanIf.h"
#include <stddef.h>

static CanMode controller_mode;
static uint8_t bus_off;
static CanTxSink tx_sink;
static uint8_t initialized;
static uint8_t tx_pending;
static uint8_t tx_in_flight;
static uint32_t tx_id;
static uint8_t tx_length;
static uint8_t tx_payload[8];
static uint32_t interrupt_disable_count;

void Can_Init(const Can_ConfigType *config) {
    Can_Lock();
    tx_sink = NULL;
    initialized = 0u;
    if (config != NULL) {
        tx_sink = config->sink;
        if (tx_sink != NULL) {
            initialized = 1u;
        }
    }
    controller_mode = CAN_STOPPED;
    bus_off = 0u;
    tx_pending = 0u;
    tx_in_flight = 0u;
    interrupt_disable_count = 0u;
    Can_Unlock();
}

void Can_DeInit(void) {
    Can_Lock();
    if ((initialized != 0u) && (controller_mode != CAN_STARTED)) {
        initialized = 0u;
        tx_sink = NULL;
        controller_mode = CAN_STOPPED;
        bus_off = 0u;
        tx_pending = 0u;
        tx_in_flight = 0u;
        interrupt_disable_count = 0u;
    }
    Can_Unlock();
}

Std_ReturnType Can_SetBaudrate(uint8_t controller, uint16_t baud_rate_config_id) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == 0u) && (baud_rate_config_id == 0u)) {
        /* The fixed virtual target has one baud-rate configuration and no registers to change. */
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_SetControllerMode(uint8_t controller, Can_ControllerStateType transition) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == 0u) &&
        ((transition == CAN_CS_STARTED) || (transition == CAN_CS_STOPPED) ||
         (transition == CAN_CS_SLEEP))) {
        if ((transition == CAN_CS_STARTED) && (controller_mode == CAN_STOPPED)) {
            controller_mode = CAN_STARTED;
            bus_off = 0u;
            result = E_OK;
        } else if ((transition == CAN_CS_STOPPED) &&
                   ((controller_mode == CAN_STARTED) || (controller_mode == CAN_SLEEP))) {
            controller_mode = CAN_STOPPED;
            tx_pending = 0u;
            result = E_OK;
        } else if ((transition == CAN_CS_SLEEP) &&
                   ((controller_mode == CAN_STOPPED) || (controller_mode == CAN_SLEEP))) {
            controller_mode = CAN_SLEEP;
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
    if ((initialized != 0u) && (controller == 0u) && (mode != NULL)) {
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
    if ((initialized != 0u) && (controller == 0u) && (error_state != NULL)) {
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
    if ((initialized != 0u) && (controller == 0u) && (interrupt_disable_count < UINT32_MAX)) {
        ++interrupt_disable_count;
    }
    Can_Unlock();
}

void Can_EnableControllerInterrupts(uint8_t controller) {
    Can_Lock();
    if ((initialized != 0u) && (controller == 0u) && (interrupt_disable_count != 0u)) {
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
    if ((hth != 0u) || (pdu == NULL) || (pdu->sdu == NULL)) {
        /* Invalid host configuration or caller input. */
    } else if (pdu->id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if ((pdu->length < 1u) || (pdu->length > 8u)) {
        result = ECU_ERR_FRAME_DLC;
    } else if (Can_TryLock() == 0) {
        result = ECU_ERR_CAN_BUSY;
    } else {
        if (initialized == 0u) {
            /* Can_Init has not installed a host output callback. */
        } else if (controller_mode != CAN_STARTED) {
            result = ECU_ERR_CONTROLLER;
        } else if ((tx_pending != 0u) || (tx_in_flight != 0u)) {
            result = ECU_ERR_CAN_BUSY;
        } else {
            size_t i;
            tx_id = pdu->id;
            tx_length = pdu->length;
            for (i = 0u; i < (size_t)pdu->length; ++i) {
                tx_payload[i] = pdu->sdu[i];
            }
            tx_pending = 1u;
            result = ECU_OK;
        }
        Can_Unlock();
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
        uint8_t payload[8];
        size_t i;
        for (i = 0u; i < (size_t)length; ++i) {
            payload[i] = tx_payload[i];
        }
        tx_pending = 0u;
        tx_in_flight = 1u;
        result = tx_sink(id, length, payload);
        tx_in_flight = 0u;
    }
    Can_Unlock();
    return result;
}

void Can_SetMode(CanMode mode) {
    Can_Lock();
    if ((controller_mode == CAN_SLEEP) && (mode != CAN_STOPPED)) {
        /* Logical sleep can only exit via STOPPED. */
    } else if (mode == CAN_BUS_OFF) {
        controller_mode = CAN_STOPPED;
        bus_off = 1u;
        tx_pending = 0u;
    } else {
        controller_mode = mode;
        bus_off = 0u;
        if (mode != CAN_STARTED) {
            tx_pending = 0u;
        }
    }
    Can_Unlock();
}

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

EcuStatus Can_Transmit(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    Can_PduType pdu;
    EcuStatus result = ECU_ERR_CONFIG;
    if (data != NULL) {
        Std_ReturnType write_result;
        size_t i;
        for (i = 0u; (i < dlc) && (i < sizeof(payload)); ++i) {
            payload[i] = data[i];
        }
        pdu.swPduHandle = 0u;
        pdu.length = dlc;
        pdu.id = id;
        pdu.sdu = payload;
        write_result = Can_Write(0u, &pdu);
        if (write_result == E_OK) {
            result = Can_HostFlush();
        } else if (write_result == CAN_BUSY) {
            result = ECU_ERR_CAN_BUSY;
        } else if (id > 0x7ffu) {
            result = ECU_ERR_FRAME_ID;
        } else if ((dlc < 1u) || (dlc > 8u)) {
            result = ECU_ERR_FRAME_DLC;
        } else {
            Can_Lock();
            if (initialized != 0u) {
                result = ECU_ERR_CONTROLLER;
            }
            Can_Unlock();
        }
    }
    return result;
}

EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    Can_Lock();
    if (id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if ((dlc < 1u) || (dlc > 8u)) {
        result = ECU_ERR_FRAME_DLC;
    } else if (controller_mode != CAN_STARTED) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL) {
        result = ECU_ERR_CONFIG;
    } else {
        result = CanIf_RxIndication(id, dlc, data, now_ms);
    }
    Can_Unlock();
    return result;
}
