/** @file
 * @brief Host persistence interface for the configured DTC.
 */
#ifndef NVM_H
#define NVM_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Load or initialize the host-only single-byte Dem status block.
 * @param[in] config Generated ECU configuration with a DTC.
 * @param[in] path Host storage file path.
 * @param[out] status Restored or initial DTC status byte.
 * @return Host persistence status.
 */
EcuStatus NvM_Init(const EcuConfig *config, const char *path, uint8_t *status);
/** @brief Persist one DTC status byte in the host storage file.
 * @param[in] status DTC status byte to store.
 * @return Host persistence status.
 */
EcuStatus NvM_Write(uint8_t status);

#endif
