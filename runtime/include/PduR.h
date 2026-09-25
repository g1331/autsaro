#ifndef PDUR_H
#define PDUR_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Status.h"

EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8]);
EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);

#endif
