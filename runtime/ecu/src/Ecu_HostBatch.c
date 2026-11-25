/** @file HostBatchV2 line parser and bounded staging, with no automotive state access. */
#include "Ecu_HostBatch.h"
#include "Can_GeneralTypes.h"
#include <string.h>

typedef struct {
    const char *data;
    size_t length;
} Ecu_BatchToken;

static int equals(Ecu_BatchToken token, const char *word, size_t length)
{
    int result = 0;
    if ((token.length == length) && (strncmp(token.data, word, length) == 0)) {
        result = 1;
    }
    return result;
}

static int hex_digit(char value)
{
    int result = -1;
    if ((value >= '0') && (value <= '9')) {
        result = (int)value - (int)'0';
    } else if ((value >= 'a') && (value <= 'f')) {
        result = ((int)value - (int)'a') + 10;
    } else {
        /* Payload uses lowercase hexadecimal. */
    }
    return result;
}

static boolean number(Ecu_BatchToken token, uint64_t maximum, uint64_t *value)
{
    uint64_t result = UINT64_C(0);
    size_t index;
    for (index = 0u; index < token.length; ++index) {
        int next = (int)token.data[index] - (int)'0';
        if ((next < 0) || (next > 9) || ((uint64_t)next > maximum) ||
            (result > ((maximum - (uint64_t)next) / UINT64_C(10)))) {
            return FALSE;
        }
        result = (result * UINT64_C(10)) + (uint64_t)next;
    }
    *value = result;
    return TRUE;
}

static boolean decode_frame(const Ecu_BatchToken tokens[7], Ecu_BatchFrame *frame)
{
    boolean extended = equals(tokens[2], "EXTENDED", 8u) != 0;
    uint64_t id;
    uint64_t length;
    size_t byte;
    if ((equals(tokens[1], "0", 1u) == 0) || (equals(tokens[3], "0", 1u) == 0) ||
        (!extended && (equals(tokens[2], "STANDARD", 8u) == 0)) ||
        !number(tokens[4], extended ? CAN_ID_EXTENDED_MAX : CAN_ID_STANDARD_MAX, &id) ||
        !number(tokens[5], UINT64_C(8), &length)) {
        return FALSE;
    }
    if (length == UINT64_C(0)) {
        if (equals(tokens[6], "-", 1u) == 0) {
            return FALSE;
        }
    } else if (tokens[6].length != (2u * (size_t)length)) {
        return FALSE;
    }
    frame->id = (uint32_t)id | (extended ? CAN_ID_EXTENDED_FLAG : 0u);
    frame->dlc = (uint8_t)length;
    for (byte = 0u; byte < (size_t)length; ++byte) {
        int high = hex_digit(tokens[6].data[2u * byte]);
        int low = hex_digit(tokens[6].data[(2u * byte) + 1u]);
        if ((high < 0) || (low < 0)) {
            return FALSE;
        }
        frame->data[byte] = (uint8_t)(((unsigned)high * 16u) + (unsigned)low);
    }
    return TRUE;
}

static StatusType reject(Ecu_HostBatch *batch, StatusType status)
{
    if (batch->open != 0u) {
        batch->rejected = 1u;
    }
    return status;
}

/* MR12 RULE 8.7 VIOLATION: the probe delivery retains the host-batch library export. */
StatusType Ecu_HostBatchParse(Ecu_HostBatch *batch,
                              const char *line,
                              size_t length,
                              Ecu_BatchCommand *command)
{
    Ecu_BatchToken tokens[7];
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
        if (count == 7u) {
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
    if (batch->negotiated == 0u) {
        if ((count != 2u) || (equals(tokens[0], "PROTOCOL", 8u) == 0) ||
            (equals(tokens[1], ECU_BATCH_PROTOCOL, sizeof(ECU_BATCH_PROTOCOL) - 1u) == 0)) {
            return E_OS_VALUE;
        }
        batch->negotiated = 1u;
        *command = ECU_BATCH_NEGOTIATE;
        return E_OK;
    }
    if (equals(tokens[0], "BEGIN", 5u) != 0) {
        uint64_t at;
        boolean parsed = count == 2u;
        if (batch->open != 0u) {
            return reject(batch, E_OS_STATE);
        }
        if (parsed) {
            parsed = number(tokens[1], UINT64_MAX, &at);
        }
        if (!parsed) {
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
                   (((uint64_t)batch->count + UINT64_C(1)) > (UINT64_MAX - batch->sequence))) {
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
        Ecu_BatchFrame frame = {0};
        if (batch->open == 0u) {
            return E_OS_STATE;
        }
        if (batch->rejected != 0u) {
            return E_OS_VALUE;
        }
        if (batch->count >= ECU_BATCH_CAPACITY) {
            return reject(batch, E_OS_LIMIT);
        }
        if ((count != 7u) || !decode_frame(tokens, &frame)) {
            return reject(batch, E_OS_VALUE);
        }
        batch->frames[batch->count] = frame;
        ++batch->count;
        *command = ECU_BATCH_RX;
        return E_OK;
    }
    return reject(batch, E_OS_VALUE);
}

StatusType Ecu_HostBatchComplete(Ecu_HostBatch *batch)
{
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
            (batch->count == 0u) ? UINT64_C(0) : (batch->sequence + UINT64_C(1));
        batch->sequence += (uint64_t)batch->count;
    }
    ++batch->sequence;
    batch->executing = 0u;
    return E_OK;
}
