/** @file
 * @brief Host-only SecurityAccess state interface.
 */
#ifndef ECU_SECURITY_H
#define ECU_SECURITY_H

#include <stdint.h>
#include "Ecu_Status.h"

/** @brief Fixed host SecurityAccess seed size in bytes. */
#define ECU_SECURITY_SEED_SIZE 16u
/** @brief Fixed host SecurityAccess key size in bytes. */
#define ECU_SECURITY_KEY_SIZE 16u

/** @brief Load the optional host SecurityAccess key and state.
 * @param[in] enabled Nonzero when the host security profile is configured.
 * @param[in] key_path Key-file path for the host profile.
 * @param[in] state_path Failed-attempt persistence path.
 * @return Host initialization status.
 */
EcuStatus Security_Init(int enabled, const char *key_path, const char *state_path);
/** @brief Clear the current unlock state and pending seed. */
void Security_Lock(void);
/** @brief Query whether the host security level is unlocked.
 * @return Nonzero only while unlocked and free of storage faults.
 */
int Security_IsUnlocked(void);
/** @brief Obtain a seed for the configured host security level.
 * @param[out] seed Fixed-size seed buffer; zeroed when already unlocked.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Zero on success, otherwise a UDS negative response code.
 */
uint8_t Security_RequestSeed(uint8_t seed[ECU_SECURITY_SEED_SIZE], uint64_t now_ms);
/** @brief Check a key against the pending seed and update the attempt state.
 * @param[in] key Fixed-size key supplied by the diagnostic client.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Zero on success, otherwise a UDS negative response code.
 */
uint8_t Security_SendKey(const uint8_t key[ECU_SECURITY_KEY_SIZE], uint64_t now_ms);
/** @brief Read the persisted failed-key attempt count.
 * @return Current failed attempt count.
 */
uint8_t Security_GetAttemptCounter(void);
/** @brief Store a failed-key attempt count for the host profile.
 * @param[in] value New failed attempt count.
 * @return Host persistence status.
 */
EcuStatus Security_SetAttemptCounter(uint8_t value);

#endif
