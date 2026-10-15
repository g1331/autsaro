/** @file Hardware-specific configuration for the controlled host CAN target. */
#ifndef CAN_CFGTYPES_H
#define CAN_CFGTYPES_H
#include "Platform_Types.h"
#include "Ecu_Status.h"
/** @brief Host callback that emits one CAN frame.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid bytes in data, 0 to 8.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
typedef EcuStatus (*Can_TxSinkType)(uint32 id, uint8 dlc, const uint8 data[8]);

/** @brief Implementation-specific Can_Init data for the host target (SWS_Can_00413). */
typedef struct {
    /** Synchronous host sink for each accepted frame; NULL selects the linked queued adapter.
     * Set by the generated host configuration, not by an ECUC parameter. */
    Can_TxSinkType sink;
} Can_ConfigType;

#endif
