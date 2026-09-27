/** @file
 * @brief Host-only DCM callback ABI types.
 */
#ifndef ECU_DCM_CALLBACK_TYPES_H
#define ECU_DCM_CALLBACK_TYPES_H

#include <stdint.h>

/** @brief Host callback return type; not the full AUTOSAR Std_Types.h ABI. */
typedef uint8_t Std_ReturnType;
/** @brief Host negative-response-code output type. */
typedef uint8_t Dcm_NegativeResponseCodeType;

/** @brief Successful host callback result. */
#define E_OK 0x00u
/** @brief Failed host callback result. */
#define E_NOT_OK 0x01u
/** @brief Host callback negative code for a programming failure. */
#define DCM_E_GENERALPROGRAMMINGFAILURE 0x72u

#endif
