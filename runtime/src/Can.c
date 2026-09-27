#include "Can.h"
#include "CanIf.h"
#include <stddef.h>
#if defined(_WIN32)
#include <windows.h>
static SRWLOCK can_lock = SRWLOCK_INIT;
static void Can_Lock(void) { AcquireSRWLockExclusive(&can_lock); }
static void Can_Unlock(void) { ReleaseSRWLockExclusive(&can_lock); }
#else
#include <pthread.h>
static pthread_mutex_t can_lock = PTHREAD_MUTEX_INITIALIZER;
static void Can_Lock(void) { (void)pthread_mutex_lock(&can_lock); }
static void Can_Unlock(void) { (void)pthread_mutex_unlock(&can_lock); }
#endif

static CanMode controller_mode;
static CanTxSink tx_sink;
static uint8_t initialized;

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
    Can_Unlock();
}

Std_ReturnType Can_SetControllerMode(uint8_t controller, Can_ControllerStateType transition) {
    Std_ReturnType result = E_NOT_OK;
    Can_Lock();
    if ((initialized != 0u) && (controller == 0u) &&
        ((transition == CAN_CS_STARTED) || (transition == CAN_CS_STOPPED))) {
        if ((transition == CAN_CS_STARTED) && (controller_mode == CAN_STOPPED)) {
            controller_mode = CAN_STARTED;
            result = E_OK;
        } else if ((transition == CAN_CS_STOPPED) && (controller_mode == CAN_STARTED)) {
            controller_mode = CAN_STOPPED;
            result = E_OK;
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
        } else {
            *mode = CAN_CS_STOPPED;
        }
        result = E_OK;
    }
    Can_Unlock();
    return result;
}

static EcuStatus Can_WriteHost(Can_HwHandleType hth, const Can_PduType *pdu) {
    EcuStatus result = ECU_ERR_CONFIG;
    Can_Lock();
    if ((initialized == 0u) || (hth != 0u) || (pdu == NULL) || (pdu->sdu == NULL)) {
        /* Invalid host configuration or caller input. */
    } else if (controller_mode != CAN_STARTED) {
        result = ECU_ERR_CONTROLLER;
    } else if (pdu->id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if ((pdu->length < 1u) || (pdu->length > 8u)) {
        result = ECU_ERR_FRAME_DLC;
    } else {
        result = tx_sink(pdu->id, pdu->length, pdu->sdu);
    }
    Can_Unlock();
    return result;
}

Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu) {
    Std_ReturnType result = E_NOT_OK;
    if (Can_WriteHost(hth, pdu) == ECU_OK) {
        result = E_OK;
    }
    return result;
}

void Can_SetMode(CanMode mode) {
    Can_Lock();
    controller_mode = mode;
    Can_Unlock();
}

CanMode Can_GetMode(void) {
    CanMode mode;
    Can_Lock();
    mode = controller_mode;
    Can_Unlock();
    return mode;
}

EcuStatus Can_Transmit(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    uint8_t payload[8] = {0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    Can_PduType pdu;
    EcuStatus result = ECU_ERR_CONFIG;
    if (data != NULL) {
        size_t i;
        for (i = 0u; (i < dlc) && (i < sizeof(payload)); ++i) {
            payload[i] = data[i];
        }
        pdu.swPduHandle = 0u;
        pdu.length = dlc;
        pdu.id = id;
        pdu.sdu = payload;
        result = Can_WriteHost(0u, &pdu);
    }
    return result;
}

EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    EcuStatus result = ECU_OK;
    if (id > 0x7ffu) {
        result = ECU_ERR_FRAME_ID;
    } else if ((dlc < 1u) || (dlc > 8u)) {
        result = ECU_ERR_FRAME_DLC;
    } else if (Can_GetMode() != CAN_STARTED) {
        result = ECU_ERR_CONTROLLER;
    } else if (data == NULL) {
        result = ECU_ERR_CONFIG;
    } else {
        result = CanIf_RxIndication(id, dlc, data, now_ms);
    }
    return result;
}
