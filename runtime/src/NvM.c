#ifndef _WIN32
#define _POSIX_C_SOURCE 200809L
#endif
#include "NvM.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <io.h>
#else
#include <unistd.h>
#endif

#define SLOT_SIZE 32u
#define SLOT_COUNT 2u

static FILE *storage;
static uint32_t fingerprint;
static uint64_t sequence;
static unsigned current_slot;

static void Store32(uint8_t *data, uint32_t value) {
    unsigned i;
    for (i = 0u; i < 4u; ++i) {
        data[i] = (uint8_t)(value >> (i * 8u));
    }
}

static void Store64(uint8_t *data, uint64_t value) {
    unsigned i;
    for (i = 0u; i < 8u; ++i) {
        data[i] = (uint8_t)(value >> (i * 8u));
    }
}

static uint32_t Load32(const uint8_t *data) {
    unsigned i;
    uint32_t value = 0u;
    for (i = 0u; i < 4u; ++i) {
        value |= (uint32_t)data[i] << (i * 8u);
    }
    return value;
}

static uint64_t Load64(const uint8_t *data) {
    unsigned i;
    uint64_t value = 0u;
    for (i = 0u; i < 8u; ++i) {
        value |= (uint64_t)data[i] << (i * 8u);
    }
    return value;
}

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

static uint32_t ConfigFingerprint(const EcuConfig *config) {
    const EcuDtcConfig *dtc = config->diagnostic->dtc;
    const EcuFrameConfig *frame = &config->frames[dtc->monitor_frame_index];
    uint8_t fields[15];
    Store32(fields, dtc->code);
    fields[4] = (uint8_t)dtc->monitor_frame_index;
    fields[5] = (uint8_t)(dtc->monitor_frame_index >> 8u);
    Store32(&fields[6], frame->id);
    fields[10] = frame->dlc;
    Store32(&fields[11], frame->timeout_ms);
    return Checksum(fields, sizeof(fields));
}

static void EncodeSlot(uint8_t slot[SLOT_SIZE], uint64_t number, uint8_t status) {
    memset(slot, 0, SLOT_SIZE);
    memcpy(slot, "NVH1", 4u);
    Store64(&slot[4], number);
    Store32(&slot[12], fingerprint);
    slot[16] = status;
    Store32(&slot[28], Checksum(slot, 28u));
}

static int ValidSlot(const uint8_t slot[SLOT_SIZE]) {
    size_t i;
    if (memcmp(slot, "NVH1", 4u) != 0 || Load64(&slot[4]) == 0u ||
        Load32(&slot[12]) != fingerprint || (slot[16] & 0x80u) != 0u ||
        Load32(&slot[28]) != Checksum(slot, 28u)) {
        return 0;
    }
    for (i = 17u; i < 28u; ++i) {
        if (slot[i] != 0u) {
            return 0;
        }
    }
    return 1;
}

static EcuStatus WriteSlot(unsigned index, uint64_t number, uint8_t status) {
    uint8_t slot[SLOT_SIZE];
    EncodeSlot(slot, number, status);
    if (fseek(storage, (long)(index * SLOT_SIZE), SEEK_SET) != 0 ||
        fwrite(slot, 1u, SLOT_SIZE, storage) != SLOT_SIZE || fflush(storage) != 0) {
        return ECU_ERR_NVM;
    }
#ifdef _WIN32
    if (_commit(_fileno(storage)) != 0) {
#else
    if (fsync(fileno(storage)) != 0) {
#endif
        return ECU_ERR_NVM;
    }
    return ECU_OK;
}

EcuStatus NvM_Init(const EcuConfig *config, const char *path, uint8_t *status) {
    uint8_t slots[SLOT_COUNT][SLOT_SIZE];
    unsigned i;
    int chosen = -1;
    unsigned valid_count = 0u;
    long length;
    if (storage != NULL) {
        (void)fclose(storage);
        storage = NULL;
    }
    if (path == NULL || path[0] == '\0' || config == NULL || config->diagnostic == NULL ||
        config->diagnostic->dtc == NULL || status == NULL) {
        return ECU_ERR_CONFIG;
    }
    fingerprint = ConfigFingerprint(config);
    storage = fopen(path, "r+b");
    if (storage == NULL) {
        if (errno != ENOENT) {
            return ECU_ERR_NVM;
        }
        storage = fopen(path, "w+b");
        if (storage == NULL) {
            return ECU_ERR_NVM;
        }
        if (WriteSlot(0u, 1u, 0x50u) != ECU_OK || WriteSlot(1u, 2u, 0x50u) != ECU_OK) {
            (void)fclose(storage);
            storage = NULL;
            return ECU_ERR_NVM;
        }
        current_slot = 1u;
        sequence = 2u;
        *status = 0x50u;
        return ECU_OK;
    }
    if (fseek(storage, 0L, SEEK_END) != 0 || (length = ftell(storage)) < (long)SLOT_SIZE ||
        length > (long)sizeof(slots) || fseek(storage, 0L, SEEK_SET) != 0) {
        (void)fclose(storage);
        storage = NULL;
        return ECU_ERR_NVM;
    }
    memset(slots, 0, sizeof(slots));
    if (fread(slots, 1u, (size_t)length, storage) != (size_t)length) {
        (void)fclose(storage);
        storage = NULL;
        return ECU_ERR_NVM;
    }
    for (i = 0u; i < SLOT_COUNT; ++i) {
        if (ValidSlot(slots[i])) {
            ++valid_count;
            if (chosen < 0 || Load64(&slots[i][4]) > Load64(&slots[chosen][4])) {
                chosen = (int)i;
            }
        }
    }
    if (chosen < 0 || valid_count != SLOT_COUNT) {
        (void)fclose(storage);
        storage = NULL;
        return ECU_ERR_NVM;
    }
    current_slot = (unsigned)chosen;
    sequence = Load64(&slots[chosen][4]);
    *status = slots[chosen][16];
    return ECU_OK;
}

EcuStatus NvM_Write(uint8_t status) {
    unsigned next_slot;
    if (storage == NULL || sequence == UINT64_MAX || (status & 0x80u) != 0u) {
        return ECU_ERR_NVM;
    }
    next_slot = 1u - current_slot;
    if (WriteSlot(next_slot, sequence + 1u, status) != ECU_OK) {
        return ECU_ERR_NVM;
    }
    current_slot = next_slot;
    ++sequence;
    return ECU_OK;
}
