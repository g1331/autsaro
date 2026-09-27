/** @file
 * @brief Host-only DCM callback ABI types.
 */
#ifndef ECU_DCM_CALLBACK_TYPES_H
#define ECU_DCM_CALLBACK_TYPES_H

#include <stdint.h>

/* Host-target callback ABI subset; not AUTOSAR Std_Types.h or Rte_Dcm_Type.h. */
typedef uint8_t Std_ReturnType;
typedef uint8_t Dcm_NegativeResponseCodeType;

#define E_OK 0x00u
#define E_NOT_OK 0x01u
#define DCM_E_GENERALPROGRAMMINGFAILURE 0x72u

#endif
