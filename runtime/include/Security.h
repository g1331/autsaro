/** @file
 * @brief Host-only SecurityAccess state interface.
 */
#ifndef ECU_SECURITY_H
#define ECU_SECURITY_H

#include <stdint.h>
#include "Ecu_Status.h"

#define ECU_SECURITY_SEED_SIZE 16u
#define ECU_SECURITY_KEY_SIZE 16u

EcuStatus Security_Init(int enabled, const char *key_path, const char *state_path);
void Security_Lock(void);
int Security_IsUnlocked(void);
uint8_t Security_RequestSeed(uint8_t seed[ECU_SECURITY_SEED_SIZE], uint64_t now_ms);
uint8_t Security_SendKey(const uint8_t key[ECU_SECURITY_KEY_SIZE], uint64_t now_ms);
uint8_t Security_GetAttemptCounter(void);
EcuStatus Security_SetAttemptCounter(uint8_t value);

#endif
