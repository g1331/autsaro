/** @file
 * @brief Host-only DCM callback ABI types.
 */
#ifndef ECU_DCM_CALLBACK_TYPES_H
#define ECU_DCM_CALLBACK_TYPES_H

#include <stdint.h>
#include "Std_Types.h"

/** @brief Host negative-response-code output type. */
typedef uint8_t Dcm_NegativeResponseCodeType;

/** @brief Host callback negative code for a programming failure. */
#define DCM_E_GENERALPROGRAMMINGFAILURE 0x72u

#endif
