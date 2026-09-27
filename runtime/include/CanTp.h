/** @file
 * @brief Bounded host DoCAN transport interface.
 */
#ifndef CANTP_H
#define CANTP_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/* One physical, normal-addressed 11-bit Classical CAN connection. */
void CanTp_Init(const EcuDiagnosticConfig *config);
EcuStatus CanTp_RxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms);
EcuStatus CanTp_Transmit(size_t length, uint64_t now_ms);
EcuStatus CanTp_AdvanceTime(uint64_t now_ms);

#endif
