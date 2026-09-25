#ifndef CANIF_H
#define CANIF_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

void CanIf_Init(const EcuConfig *config);
EcuStatus CanIf_Transmit(size_t frame_index, const uint8_t data[8]);
EcuStatus CanIf_RxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#endif
