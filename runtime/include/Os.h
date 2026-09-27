/** @file
 * @brief Host virtual scheduler and clock interface.
 */
#ifndef OS_H
#define OS_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Reset the single-core host clock and configured schedule.
 * @param[in] config Generated ECU configuration.
 */
void Os_Init(const EcuConfig *config);
/** @brief Advance periodic Com transmission and receive timers.
 * @param[in] now_ms New host clock in milliseconds.
 * @return Host scheduling status.
 */
EcuStatus Os_Advance(uint64_t now_ms);
/** @brief Read the current host clock.
 * @return Current host clock in milliseconds.
 */
uint64_t Os_Now(void);

#endif
