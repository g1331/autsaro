/** @file
 * @brief Host ECU startup and validation interface.
 */
#ifndef ECU_RUNTIME_H
#define ECU_RUNTIME_H

#include "Can.h"
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Validate the generated configuration before initializing host modules.
 * @param[in] config Generated ECU configuration.
 * @param[in] sink Host callback that emits transmitted CAN frames.
 * @param[in] nvm_path Host DTC storage path, when DTC persistence is configured.
 * @param[in] security_key_path Host key-file path, when SecurityAccess is configured.
 * @param[in] security_state_path Host security-state path, when SecurityAccess is configured.
 * @return ECU_OK after initialization, otherwise an error without starting the ECU.
 */
EcuStatus Ecu_Init(const EcuConfig *config, CanTxSink sink, const char *nvm_path,
                   const char *security_key_path, const char *security_state_path);

#endif
