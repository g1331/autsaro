#include "Security.h"
#include <stddef.h>
#ifdef _WIN32
#include <errno.h>
#include <io.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include <bcrypt.h>

#define SECURITY_SECRET_SIZE 32u
#define SECURITY_STATE_SIZE 16u
#define SECURITY_MAX_ATTEMPTS 3u
#define SECURITY_DELAY_MS UINT64_C(5000)

static const uint8_t key_domain[] = "AUTOSAR-HOST-SECURITY-v1";
static uint8_t secret[SECURITY_SECRET_SIZE];
static uint8_t pending_seed[ECU_SECURITY_SEED_SIZE];
static const char *state_file;
static uint8_t attempts;
static uint8_t pending;
static uint8_t unlocked;
static uint8_t faulted;
static uint64_t delay_until;

static uint32_t Checksum(const uint8_t *data, size_t length) {
    uint32_t crc = UINT32_MAX;
    size_t i;
    for (i = 0u; i < length; ++i) {
        unsigned bit;
        crc ^= data[i];
        for (bit = 0u; bit < 8u; ++bit) {
            crc = (crc >> 1u) ^ ((crc & 1u) != 0u ? UINT32_C(0xedb88320) : 0u);
        }
    }
    return ~crc;
}

static uint32_t Load32(const uint8_t *data) {
    return (uint32_t)data[0] | ((uint32_t)data[1] << 8u) | ((uint32_t)data[2] << 16u) |
           ((uint32_t)data[3] << 24u);
}

static EcuStatus SaveAttempts(uint8_t value) {
    uint8_t record[SECURITY_STATE_SIZE] = {0};
    uint32_t crc;
    FILE *file;
    memcpy(record, "SEC1", 4u);
    record[4] = value;
    crc = Checksum(record, 12u);
    record[12] = (uint8_t)crc;
    record[13] = (uint8_t)(crc >> 8u);
    record[14] = (uint8_t)(crc >> 16u);
    record[15] = (uint8_t)(crc >> 24u);
    file = fopen(state_file, "wb");
    if (file == NULL) {
        return ECU_ERR_NVM;
    }
    if (fwrite(record, 1u, sizeof(record), file) != sizeof(record) || fflush(file) != 0 ||
        _commit(_fileno(file)) != 0) {
        (void)fclose(file);
        return ECU_ERR_NVM;
    }
    if (fclose(file) != 0) {
        return ECU_ERR_NVM;
    }
    attempts = value;
    return ECU_OK;
}

static EcuStatus LoadAttempts(void) {
    uint8_t record[SECURITY_STATE_SIZE];
    FILE *file = fopen(state_file, "rb");
    int valid;
    if (file == NULL) {
        if (errno != ENOENT) {
            return ECU_ERR_NVM;
        }
        return SaveAttempts(0u);
    }
    valid = fread(record, 1u, sizeof(record), file) == sizeof(record) && fgetc(file) == EOF &&
            !ferror(file);
    if (fclose(file) != 0 || !valid) {
        return ECU_ERR_NVM;
    }
    if (memcmp(record, "SEC1", 4u) != 0 || record[4] > SECURITY_MAX_ATTEMPTS ||
        memcmp(&record[5], "\0\0\0\0\0\0\0", 7u) != 0 ||
        Load32(&record[12]) != Checksum(record, 12u)) {
        return ECU_ERR_NVM;
    }
    attempts = record[4];
    return ECU_OK;
}

uint8_t Security_GetAttemptCounter(void) { return attempts; }

EcuStatus Security_SetAttemptCounter(uint8_t value) {
    if (value > SECURITY_MAX_ATTEMPTS || faulted != 0u || state_file == NULL) {
        return ECU_ERR_CONFIG;
    }
    if (SaveAttempts(value) != ECU_OK) {
        faulted = 1u;
        Security_Lock();
        return ECU_ERR_NVM;
    }
    return ECU_OK;
}

static int ExpectedKey(uint8_t result[ECU_SECURITY_KEY_SIZE]) {
    BCRYPT_ALG_HANDLE algorithm = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    uint8_t digest[32];
    NTSTATUS status = BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, NULL,
                                                  BCRYPT_ALG_HANDLE_HMAC_FLAG);
    if (status >= 0) {
        status = BCryptCreateHash(algorithm, &hash, NULL, 0u, secret, sizeof(secret), 0u);
    }
    if (status >= 0) {
        status = BCryptHashData(hash, (PUCHAR)key_domain, (ULONG)(sizeof(key_domain) - 1u), 0u);
    }
    if (status >= 0) {
        status = BCryptHashData(hash, pending_seed, sizeof(pending_seed), 0u);
    }
    if (status >= 0) {
        status = BCryptFinishHash(hash, digest, sizeof(digest), 0u);
    }
    if (status >= 0) {
        memcpy(result, digest, ECU_SECURITY_KEY_SIZE);
    }
    SecureZeroMemory(digest, sizeof(digest));
    if (hash != NULL) {
        (void)BCryptDestroyHash(hash);
    }
    if (algorithm != NULL) {
        (void)BCryptCloseAlgorithmProvider(algorithm, 0u);
    }
    return status >= 0;
}

EcuStatus Security_Init(int enabled, const char *key_path, const char *state_path) {
    FILE *file;
    int valid;
    Security_Lock();
    SecureZeroMemory(secret, sizeof(secret));
    state_file = NULL;
    attempts = 0u;
    faulted = 0u;
    delay_until = 0u;
    if (!enabled) {
        return key_path == NULL && state_path == NULL ? ECU_OK : ECU_ERR_CONFIG;
    }
    if (key_path == NULL || state_path == NULL || key_path[0] == '\0' || state_path[0] == '\0' ||
        strcmp(key_path, state_path) == 0) {
        return ECU_ERR_CONFIG;
    }
    file = fopen(key_path, "rb");
    if (file == NULL) {
        return ECU_ERR_CONFIG;
    }
    valid = fread(secret, 1u, sizeof(secret), file) == sizeof(secret) && fgetc(file) == EOF &&
            !ferror(file);
    if (fclose(file) != 0 || !valid) {
        SecureZeroMemory(secret, sizeof(secret));
        return ECU_ERR_CONFIG;
    }
    state_file = state_path;
    if (LoadAttempts() != ECU_OK) {
        SecureZeroMemory(secret, sizeof(secret));
        return ECU_ERR_NVM;
    }
    if (attempts >= SECURITY_MAX_ATTEMPTS) {
        delay_until = SECURITY_DELAY_MS;
    }
    return ECU_OK;
}

void Security_Lock(void) {
    unlocked = 0u;
    pending = 0u;
    SecureZeroMemory(pending_seed, sizeof(pending_seed));
}

int Security_IsUnlocked(void) { return unlocked != 0u && faulted == 0u; }

uint8_t Security_RequestSeed(uint8_t seed[ECU_SECURITY_SEED_SIZE], uint64_t now_ms) {
    if (faulted != 0u) {
        return 0x22u;
    }
    if (attempts >= SECURITY_MAX_ATTEMPTS) {
        if (now_ms < delay_until) {
            return 0x37u;
        }
        if (SaveAttempts(0u) != ECU_OK) {
            faulted = 1u;
            Security_Lock();
            return 0x22u;
        }
    }
    if (unlocked != 0u) {
        memset(seed, 0, ECU_SECURITY_SEED_SIZE);
        return 0u;
    }
    if (BCryptGenRandom(NULL, pending_seed, sizeof(pending_seed), BCRYPT_USE_SYSTEM_PREFERRED_RNG) <
        0) {
        Security_Lock();
        return 0x22u;
    }
    memcpy(seed, pending_seed, ECU_SECURITY_SEED_SIZE);
    pending = 1u;
    return 0u;
}

uint8_t Security_SendKey(const uint8_t key[ECU_SECURITY_KEY_SIZE], uint64_t now_ms) {
    uint8_t expected[ECU_SECURITY_KEY_SIZE];
    uint8_t difference = 0u;
    size_t i;
    if (faulted != 0u) {
        return 0x22u;
    }
    if (pending == 0u || unlocked != 0u) {
        return 0x24u;
    }
    if (!ExpectedKey(expected)) {
        Security_Lock();
        return 0x22u;
    }
    for (i = 0u; i < sizeof(expected); ++i) {
        difference |= (uint8_t)(expected[i] ^ key[i]);
    }
    SecureZeroMemory(expected, sizeof(expected));
    pending = 0u;
    SecureZeroMemory(pending_seed, sizeof(pending_seed));
    if (difference == 0u) {
        if (SaveAttempts(0u) != ECU_OK) {
            faulted = 1u;
            Security_Lock();
            return 0x22u;
        }
        unlocked = 1u;
        return 0u;
    }
    if (SaveAttempts((uint8_t)(attempts + 1u)) != ECU_OK) {
        faulted = 1u;
        Security_Lock();
        return 0x22u;
    }
    if (attempts == SECURITY_MAX_ATTEMPTS) {
        delay_until =
            now_ms <= UINT64_MAX - SECURITY_DELAY_MS ? now_ms + SECURITY_DELAY_MS : UINT64_MAX;
        return 0x36u;
    }
    return 0x35u;
}
#else
EcuStatus Security_Init(int enabled, const char *key_path, const char *state_path) {
    return enabled || key_path != NULL || state_path != NULL ? ECU_ERR_CONFIG : ECU_OK;
}

void Security_Lock(void) {}
int Security_IsUnlocked(void) { return 0; }
uint8_t Security_RequestSeed(uint8_t seed[ECU_SECURITY_SEED_SIZE], uint64_t now_ms) {
    (void)seed;
    (void)now_ms;
    return 0x11u;
}
uint8_t Security_SendKey(const uint8_t key[ECU_SECURITY_KEY_SIZE], uint64_t now_ms) {
    (void)key;
    (void)now_ms;
    return 0x11u;
}
uint8_t Security_GetAttemptCounter(void) { return 0u; }
EcuStatus Security_SetAttemptCounter(uint8_t value) {
    (void)value;
    return ECU_ERR_CONFIG;
}
#endif
