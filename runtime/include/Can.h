#ifndef CAN_H
#define CAN_H

#include <stdint.h>
#include "Ecu_Status.h"

typedef enum {
    CAN_STOPPED = 0,
    CAN_STARTED = 1,
    CAN_BUS_OFF = 2
} CanMode;

typedef EcuStatus (*CanTxSink)(uint32_t id, uint8_t dlc, const uint8_t data[8]);

void Can_Init(CanTxSink sink);
void Can_SetMode(CanMode mode);
CanMode Can_GetMode(void);
EcuStatus Can_Transmit(uint32_t id, uint8_t dlc, const uint8_t data[8]);
EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#endif
