/** @file Hardware-specific configuration for the controlled host CAN target. */
#ifndef CAN_CFGTYPES_H
#define CAN_CFGTYPES_H
#include <stdint.h>
#include "Ecu_Status.h"
/** @brief Host callback that emits one CAN frame.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid bytes in data.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
typedef EcuStatus (*CanTxSink)(uint32_t id, uint8_t dlc, const uint8_t data[8]);

/** @brief Hardware-specific Can Driver initialization data for the host target. */
typedef struct {
    CanTxSink sink; /**< Required synchronous sink; NULL for the linked queued adapter. */
} Can_ConfigType;

#endif
