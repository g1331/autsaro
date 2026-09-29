#include "Ecu_HostBatch.h"
#include <string.h>

typedef struct {
    const char *data;
    size_t length;
} Ecu_BatchToken;

static int equals(Ecu_BatchToken token, const char *word, size_t length) {
    return (token.length == length) && (memcmp(token.data, word, length) == 0);
}

static int digit(char value) {
    int result = -1;
    if ((value >= '0') && (value <= '9')) {
        result = value - '0';
    } else if ((value >= 'a') && (value <= 'f')) {
        result = (value - 'a') + 10;
    } else if ((value >= 'A') && (value <= 'F')) {
        result = (value - 'A') + 10;
    } else {
        /* No sign, Unicode digit or other encoding is accepted. */
    }
    return result;
}

static int number(Ecu_BatchToken token, unsigned base, uint64_t maximum, uint64_t *value) {
    uint64_t result = UINT64_C(0);
    size_t index = 0u;
    if ((base == 16u) && (token.length > 2u) && (token.data[0] == '0') &&
        ((token.data[1] == 'x') || (token.data[1] == 'X'))) {
        index = 2u;
    }
    if (index == token.length) {
        return 0;
    }
    for (; index < token.length; ++index) {
        int next = digit(token.data[index]);
        if ((next < 0) || ((unsigned)next >= base) || ((uint64_t)next > maximum) ||
            (result > ((maximum - (uint64_t)next) / base))) {
            return 0;
        }
        result = (result * base) + (uint64_t)next;
    }
    *value = result;
    return 1;
}

static StatusType reject(Ecu_HostBatch *batch, StatusType status) {
    if (batch->open != 0u) {
        batch->rejected = 1u;
    }
    return status;
}

void Ecu_HostBatchInitialize(Ecu_HostBatch *batch) {
    if (batch != NULL) {
        (void)memset(batch, 0, sizeof(*batch));
    }
}

StatusType Ecu_HostBatchParse(Ecu_HostBatch *batch, const char *line, size_t length,
                              Ecu_BatchCommand *command) {
    Ecu_BatchToken tokens[4];
    unsigned count = 0u;
    size_t index = 0u;
    if ((batch == NULL) || (line == NULL) || (command == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (batch->executing != 0u) {
        return E_OS_STATE;
    }
    if ((length == 0u) || (length > ECU_BATCH_LINE_CAPACITY)) {
        return reject(batch, E_OS_VALUE);
    }
    while (index < length) {
        size_t start;
        if (line[index] == ' ') {
            ++index;
            continue;
        }
        if (count == 4u) {
            return reject(batch, E_OS_VALUE);
        }
        start = index;
        while ((index < length) && (line[index] != ' ')) {
            if ((line[index] < '!') || (line[index] > '~')) {
                return reject(batch, E_OS_VALUE);
            }
            ++index;
        }
        tokens[count].data = &line[start];
        tokens[count].length = index - start;
        ++count;
    }
    if (count == 0u) {
        return reject(batch, E_OS_VALUE);
    }
    if (equals(tokens[0], "BEGIN", 5u) != 0) {
        uint64_t at;
        if (batch->open != 0u) {
            return reject(batch, E_OS_STATE);
        }
        if ((count != 2u) || (number(tokens[1], 10u, UINT64_MAX, &at) == 0)) {
            return E_OS_VALUE;
        }
        if ((at < batch->completed_epoch) || ((at - batch->completed_epoch) > ECU_BATCH_MAX_SPAN)) {
            return E_OS_VALUE;
        }
        batch->epoch = at;
        batch->count = 0u;
        batch->open = 1u;
        batch->rejected = 0u;
        *command = ECU_BATCH_BEGIN;
        return E_OK;
    }
    if (equals(tokens[0], "COMMIT", 6u) != 0) {
        StatusType status = E_OK;
        if (batch->open == 0u) {
            return E_OS_STATE;
        }
        if ((count != 1u) || (batch->rejected != 0u)) {
            status = E_OS_VALUE;
        } else if ((batch->batch_id == UINT64_MAX) ||
                   ((uint64_t)batch->count + UINT64_C(1) > (UINT64_MAX - batch->sequence))) {
            status = E_OS_LIMIT;
        } else {
            batch->executing = 1u;
            batch->input_sequences_reserved = 0u;
            batch->first_input_sequence = UINT64_C(0);
            *command = ECU_BATCH_COMMIT;
        }
        batch->open = 0u;
        if (status != E_OK) {
            batch->count = 0u;
            batch->rejected = 0u;
        }
        return status;
    }
    if (equals(tokens[0], "RX", 2u) != 0) {
        uint64_t id;
        uint64_t dlc;
        unsigned base = 10u;
        Ecu_BatchFrame frame = {0};
        size_t byte;
        if (batch->open == 0u) {
            return E_OS_STATE;
        }
        if (batch->rejected != 0u) {
            return E_OS_VALUE;
        }
        if (batch->count >= ECU_BATCH_CAPACITY) {
            return reject(batch, E_OS_LIMIT);
        }
        if ((count == 4u) && (tokens[1].length > 2u) && (tokens[1].data[0] == '0') &&
            ((tokens[1].data[1] == 'x') || (tokens[1].data[1] == 'X'))) {
            base = 16u;
        }
        if ((count != 4u) || (number(tokens[1], base, UINT64_C(2047), &id) == 0) ||
            (number(tokens[2], 10u, UINT64_C(8), &dlc) == 0) || (dlc == UINT64_C(0)) ||
            (tokens[3].length != (2u * (size_t)dlc))) {
            return reject(batch, E_OS_VALUE);
        }
        frame.id = (uint32_t)id;
        frame.dlc = (uint8_t)dlc;
        for (byte = 0u; byte < (size_t)dlc; ++byte) {
            int high = digit(tokens[3].data[2u * byte]);
            int low = digit(tokens[3].data[(2u * byte) + 1u]);
            if ((high < 0) || (low < 0)) {
                return reject(batch, E_OS_VALUE);
            }
            frame.data[byte] = (uint8_t)(((unsigned)high * 16u) + (unsigned)low);
        }
        batch->frames[batch->count] = frame;
        ++batch->count;
        *command = ECU_BATCH_RX;
        return E_OK;
    }
    return reject(batch, E_OS_VALUE);
}

StatusType Ecu_HostBatchComplete(Ecu_HostBatch *batch) {
    if (batch == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (batch->executing == 0u) {
        return E_OS_STATE;
    }
    batch->completed_epoch = batch->epoch;
    ++batch->batch_id;
    if (batch->input_sequences_reserved == 0u) {
        batch->first_input_sequence =
            (batch->count == 0u) ? UINT64_C(0) : batch->sequence + UINT64_C(1);
        batch->sequence += (uint64_t)batch->count;
    }
    ++batch->sequence;
    batch->executing = 0u;
    return E_OK;
}
