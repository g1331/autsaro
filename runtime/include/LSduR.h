#ifndef LSDUR_H
#define LSDUR_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

void LSduR_Init(const EcuDiagnosticConfig *config);
EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]);
EcuStatus LSduR_CanTpRxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms);
EcuStatus LSduR_PduRTransmit(size_t frame_index, const uint8_t data[8]);
EcuStatus LSduR_CanIfRxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);

#endif
