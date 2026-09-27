#include "NvM.h"
#include "NvM_HostStorage.h"
#include <string.h>

#define SLOT_SIZE 32u
#define SLOT_COUNT 2u

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
        data[i] = (uint8_t)(value >> ((uint64_t)i * UINT64_C(8)));
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
        value |= (uint64_t)data[i] << ((uint64_t)i * UINT64_C(8));
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
            crc = (crc >> 1u) ^ (((crc & 1u) != 0u) ? 0xedb88320u : 0u);
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
    (void)memset(slot, 0, SLOT_SIZE);
    (void)memcpy(slot, "NVH1", 4u);
    Store64(&slot[4], number);
    Store32(&slot[12], fingerprint);
    slot[16] = status;
    Store32(&slot[28], Checksum(slot, 28u));
}

static int ValidSlot(const uint8_t slot[SLOT_SIZE]) {
    size_t i;
    int valid = 1;
    if ((slot[0] != UINT8_C(0x4e)) || (slot[1] != UINT8_C(0x56)) || (slot[2] != UINT8_C(0x48)) ||
        (slot[3] != UINT8_C(0x31)) || (Load64(&slot[4]) == 0u) ||
        (Load32(&slot[12]) != fingerprint) || ((slot[16] & 0x80u) != 0u) ||
        (Load32(&slot[28]) != Checksum(slot, 28u))) {
        valid = 0;
    } else {
        for (i = 17u; i < 28u; ++i) {
            if (slot[i] != 0u) {
                valid = 0;
                break;
            }
        }
    }
    return valid;
}

static EcuStatus WriteSlot(unsigned index, uint64_t number, uint8_t status) {
    uint8_t slot[SLOT_SIZE];
    EcuStatus result = ECU_ERR_NVM;
    EncodeSlot(slot, number, status);
    if (NvM_HostWrite((size_t)index * SLOT_SIZE, slot, sizeof(slot)) != 0) {
        /* The host write or durability sync failed. */
    } else {
        result = ECU_OK;
    }
    return result;
}

EcuStatus NvM_Init(const EcuConfig *config, const char *path, uint8_t *status) {
    uint8_t slots[SLOT_COUNT][SLOT_SIZE];
    unsigned i;
    int chosen = -1;
    unsigned valid_count = 0u;
    long length;
    EcuStatus result = ECU_ERR_CONFIG;
    NvM_HostClose();
    if ((path != NULL) && (path[0] != '\0') && (config != NULL) && (config->diagnostic != NULL) &&
        (config->diagnostic->dtc != NULL) && (status != NULL)) {
        fingerprint = ConfigFingerprint(config);
        result = ECU_ERR_NVM;
        switch (NvM_HostOpen(path)) {
        case NVM_HOST_OPEN_CREATED:
            if ((WriteSlot(0u, 1u, 0x50u) == ECU_OK) && (WriteSlot(1u, 2u, 0x50u) == ECU_OK)) {
                current_slot = 1u;
                sequence = 2u;
                *status = 0x50u;
                result = ECU_OK;
            }
            break;
        case NVM_HOST_OPEN_EXISTING:
            length = NvM_HostLength();
            if ((length >= (long)SLOT_SIZE) && (length <= (long)sizeof(slots))) {
                (void)memset(slots, 0, sizeof(slots));
                if (NvM_HostRead(&slots[0][0], (size_t)length) == 0) {
                    for (i = 0u; i < SLOT_COUNT; ++i) {
                        if (ValidSlot(slots[i]) != 0) {
                            ++valid_count;
                            if ((chosen < 0) ||
                                (Load64(&slots[i][4]) > Load64(&slots[chosen][4]))) {
                                chosen = (int)i;
                            }
                        }
                    }
                    if ((chosen >= 0) && (valid_count == SLOT_COUNT)) {
                        current_slot = (unsigned)chosen;
                        sequence = Load64(&slots[chosen][4]);
                        *status = slots[chosen][16];
                        result = ECU_OK;
                    }
                }
            }
            break;
        default:
            break;
        }
        if (result != ECU_OK) {
            NvM_HostClose();
        }
    }
    return result;
}

EcuStatus NvM_Write(uint8_t status) {
    unsigned next_slot;
    EcuStatus result = ECU_ERR_NVM;
    if ((sequence != UINT64_MAX) && ((status & 0x80u) == 0u)) {
        next_slot = 1u - current_slot;
        result = WriteSlot(next_slot, sequence + 1u, status);
        if (result == ECU_OK) {
            current_slot = next_slot;
            ++sequence;
        }
    }
    return result;
}
