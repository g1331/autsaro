/** @file Hardware-specific configuration for the controlled host CAN target. */
#ifndef CAN_CFGTYPES_H
#define CAN_CFGTYPES_H
#include "Can_Version.h"
#include "Platform_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PLATFORM_AR_RELEASE_MAJOR_VERSION) || !defined(PLATFORM_AR_RELEASE_MINOR_VERSION)
#error "Can_CfgTypes.h -> Platform_Types.h: missing AUTOSAR release"
#elif (PLATFORM_AR_RELEASE_MAJOR_VERSION != 4U) || (PLATFORM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can_CfgTypes.h -> Platform_Types.h: incompatible AUTOSAR release"
#endif
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
    Can_TxSinkType sink; /**< Synchronous sink for each frame; NULL selects the linked queued
                          * adapter. Set by the generated host configuration, no ECUC parameter. */
} Can_ConfigType;

#endif
