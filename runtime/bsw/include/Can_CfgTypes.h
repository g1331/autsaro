/** @file Hardware-specific configuration for the controlled host CAN target. */
#ifndef CAN_CFGTYPES_H
#define CAN_CFGTYPES_H
#include "Can_Version.h"
#include "Can_GeneralTypes.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CAN_AR_RELEASE_MAJOR_VERSION) || !defined(CAN_AR_RELEASE_MINOR_VERSION)
#error "Can_CfgTypes.h -> Can_GeneralTypes.h: missing AUTOSAR release"
#elif (CAN_AR_RELEASE_MAJOR_VERSION != 4U) || (CAN_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can_CfgTypes.h -> Can_GeneralTypes.h: incompatible AUTOSAR release"
#endif
#include "Platform_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PLATFORM_AR_RELEASE_MAJOR_VERSION) || !defined(PLATFORM_AR_RELEASE_MINOR_VERSION)
#error "Can_CfgTypes.h -> Platform_Types.h: missing AUTOSAR release"
#elif (PLATFORM_AR_RELEASE_MAJOR_VERSION != 4U) || (PLATFORM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can_CfgTypes.h -> Platform_Types.h: incompatible AUTOSAR release"
#endif
#include "Ecu_Status.h"
/** @brief Host callback that emits one CAN frame.
 * @param[in] id Encoded CAN identifier.
 * @param[in] dlc Number of valid bytes in data, 0 to 64.
 * @param[in] data Frame buffer with dlc valid bytes.
 * @return Host transmission status.
 */
typedef EcuStatus (*Can_TxSinkType)(uint32 id, uint8 dlc, const uint8 *data);

/** One generated CanController and its explicit CanIf binding. */
typedef struct {
    uint8 controller;       /**< Driver controller identifier. */
    uint8 can_if;           /**< CanIf controller used in callbacks. */
    uint32 nominal_bitrate; /**< Configured nominal rate in bit/s. */
    uint32 data_bitrate;    /**< Configured FD rate in bit/s, zero for Classical. */
    boolean brs;            /**< Select FD data bitrate on transmission. */
} Can_ControllerConfigType;

#define CAN_HOH_ID_STANDARD 0u /**< Standard identifiers. */
#define CAN_HOH_ID_EXTENDED 1u /**< Extended identifiers. */
#define CAN_HOH_ID_MIXED    2u /**< Standard and extended identifiers. */

/** One configured hardware object in the driver's shared handle namespace. */
typedef struct {
    Can_HwHandleType handle; /**< Configured driver HOH identifier. */
    uint8 controller;        /**< Owning driver controller identifier. */
    uint8 format;            /**< Configured CAN_HOH_ID_* acceptance. */
    boolean receive;         /**< TRUE for an HRH, FALSE for an HTH. */
    uint8 fd_padding;        /**< Wire padding for an FD transmit object. */
} Can_HohConfigType;

/** @brief Implementation-specific Can_Init data for the host target (SWS_Can_00413). */
typedef struct {
    Can_TxSinkType sink;    /**< Synchronous sink for each frame; NULL selects the linked queued
                             * adapter. Set by the generated host configuration, no ECUC parameter. */
    uint32 nominal_bitrate; /**< CanControllerBaudRate in bits per second. */
    uint32 data_bitrate;    /**< CanControllerFdBaudRate in bits per second; zero disables FD. */
    boolean brs;      /**< CanControllerTxBitRateSwitch; TRUE selects the data bitrate for FD Tx. */
    uint8 fd_padding; /**< CanFdPaddingValue of the transmit HOH. */
    const Can_ControllerConfigType *controllers; /**< Generated controller rows. */
    uint16 controller_count;                     /**< Number of controller rows. */
    const Can_HohConfigType *hardware;           /**< Generated hardware objects. */
    uint32 hardware_count;                       /**< Number of hardware objects. */
} Can_ConfigType;

#endif
