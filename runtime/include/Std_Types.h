/** @file
 * @brief C99 host subset of common BSW status types.
 */
#ifndef STD_TYPES_H
#define STD_TYPES_H

#include <stdint.h>

/** @brief Return status used by standard-facing host BSW entry points. */
typedef uint8_t Std_ReturnType;

/** @brief Request accepted by a standard-facing host BSW entry point. */
#define E_OK 0x00u
/** @brief Request rejected by a standard-facing host BSW entry point. */
#define E_NOT_OK 0x01u

#endif
