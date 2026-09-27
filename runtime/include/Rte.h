/** @file
 * @brief Host signal access interface.
 */
#ifndef RTE_H
#define RTE_H

#include <stdint.h>
#include "Ecu_Status.h"

EcuStatus Rte_WriteSignal(uint16_t id, uint32_t value);
EcuStatus Rte_ReadSignal(uint16_t id, uint32_t *value, uint8_t *valid);

#endif
