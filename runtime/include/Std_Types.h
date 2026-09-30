/** @file
 * @brief C99 host subset of common BSW status types.
 */
#ifndef STD_TYPES_H
#define STD_TYPES_H

#include <stdint.h>
/** @brief Eight-bit boolean used by standard-facing host interfaces. */
typedef uint8_t boolean;
#define FALSE 0
#define TRUE 1

/** @brief Return status used by standard-facing host BSW entry points. */
typedef uint8_t Std_ReturnType;

/** @brief Standard module software identity returned by GetVersionInfo. */
typedef struct {
    uint16_t vendorID;
    uint16_t moduleID;
    uint8_t sw_major_version;
    uint8_t sw_minor_version;
    uint8_t sw_patch_version;
} Std_VersionInfoType;

/** @brief Request accepted by a standard-facing host BSW entry point. */
#define E_OK 0x00u
/** @brief Request rejected by a standard-facing host BSW entry point. */
#define E_NOT_OK 0x01u

#endif
