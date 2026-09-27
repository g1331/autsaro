/** @file
 * @brief Host signal access interface.
 */
#ifndef RTE_H
#define RTE_H

#include <stdint.h>
#include "Ecu_Status.h"

/** @brief Write one generated host signal through Com.
 * @param[in] id Generated signal identifier.
 * @param[in] value Unsigned value to store.
 * @return Host signal status.
 */
EcuStatus Rte_WriteSignal(uint16_t id, uint32_t value);
/** @brief Read one generated host signal through Com.
 * @param[in] id Generated signal identifier.
 * @param[out] value Current unsigned value.
 * @param[out] valid Nonzero when a received signal has a valid value.
 * @return Host signal status.
 */
EcuStatus Rte_ReadSignal(uint16_t id, uint32_t *value, uint8_t *valid);

#endif
