#ifndef COM_H
#define COM_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

void Com_Init(const EcuConfig *config);
EcuStatus Com_SetSignal(uint16_t id, uint32_t value);
EcuStatus Com_GetSignal(uint16_t id, uint32_t *value, uint8_t *valid);
EcuStatus Com_TriggerTransmit(size_t frame_index);
EcuStatus Com_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);
void Com_AdvanceTime(uint64_t now_ms);

#endif
