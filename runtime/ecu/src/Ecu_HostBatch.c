/** @file HostBatchV2 line parser and bounded staging, with no automotive state access. */
#include "Ecu_HostBatch.h"
#include "Can_GeneralTypes.h"
#include <string.h>

typedef struct {
    const char *data;
    size_t length;
} Ecu_BatchTokenType;

static int equals(Ecu_BatchTokenType token, const char *word, size_t length)
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

static boolean number(Ecu_BatchTokenType token, uint64_t maximum, uint64_t *value)
{
    uint64_t result = UINT64_C(0);
    size_t index = 0u;
    boolean valid = token.length != 0u;
    while ((index < token.length) && valid) {
        int next = (int)token.data[index] - (int)'0';
        valid = (next >= 0) && (next <= 9) && ((uint64_t)next <= maximum) &&
                (result <= ((maximum - (uint64_t)next) / UINT64_C(10)));
        if (valid) {
            result = (result * UINT64_C(10)) + (uint64_t)next;
        }
        ++index;
    }
    if (valid) {
        *value = result;
    }
    return valid;
}

static boolean decode_payload(Ecu_BatchTokenType token, Ecu_BatchFrame *frame)
{
    boolean valid;
    if (frame->dlc == 0u) {
        valid = equals(token, "-", 1u) != 0;
    } else {
        size_t byte = 0u;
        valid = token.length == (2u * (size_t)frame->dlc);
        while ((byte < (size_t)frame->dlc) && valid) {
            int high = hex_digit(token.data[2u * byte]);
            int low = hex_digit(token.data[(2u * byte) + 1u]);
            valid = (high >= 0) && (low >= 0);
            if (valid) {
                frame->data[byte] = (uint8_t)(((unsigned)high * 16u) + (unsigned)low);
            }
            ++byte;
        }
    }
    return valid;
}

static boolean decode_frame(const Ecu_BatchTokenType tokens[7], Ecu_BatchFrame *frame)
{
    boolean extended = equals(tokens[2], "EXTENDED", 8u) != 0;
    boolean fd = equals(tokens[3], "1", 1u) != 0;
    uint64_t id;
    uint64_t length;
    uint64_t channel;
    boolean valid = number(tokens[1], UINT8_MAX, &channel) &&
                    (fd || (equals(tokens[3], "0", 1u) != 0)) &&
                    (extended || (equals(tokens[2], "STANDARD", 8u) != 0));
    if (valid) {
        valid = number(tokens[4], extended ? CAN_ID_EXTENDED_MAX : CAN_ID_STANDARD_MAX, &id);
    }
    if (valid) {
        valid = number(tokens[5], CAN_FD_MAX_LENGTH, &length);
    }
    if (valid) {
        frame->id =
            (uint32_t)id | (extended ? CAN_ID_EXTENDED_FLAG : 0u) | (fd ? CAN_ID_FD_FLAG : 0u);
        frame->dlc = (uint8_t)length;
        frame->channel = (uint8_t)channel;
        valid = Can_IsWireLength(frame->id, frame->dlc) && decode_payload(tokens[6], frame);
    }
    return valid;
}

static boolean decode_mode(const Ecu_BatchTokenType tokens[7], Ecu_BatchFrame *frame)
{
    uint64_t channel;
    boolean valid = number(tokens[1], UINT8_MAX, &channel);
    if (valid) {
        frame->channel = (uint8_t)channel;
        if (equals(tokens[2], "STOPPED", 7u) != 0) {
            frame->kind = ECU_BATCH_STOPPED;
        } else if (equals(tokens[2], "OFFLINE", 7u) != 0) {
            frame->kind = ECU_BATCH_OFFLINE;
        } else if (equals(tokens[2], "BUS_OFF", 7u) != 0) {
            frame->kind = ECU_BATCH_BUS_OFF;
        } else if (equals(tokens[2], "STARTED", 7u) != 0) {
            frame->kind = ECU_BATCH_STARTED;
        } else {
            valid = FALSE;
        }
    }
    return valid;
}

static unsigned tokenize(const char *line, size_t length, Ecu_BatchTokenType tokens[7])
{
    unsigned count = 0u;
    size_t index = 0u;
    boolean valid = length != 0u;
    while ((index < length) && valid) {
        if (line[index] == ' ') {
            ++index;
        } else {
            size_t start = index;
            valid = count < 7u;
            if (valid) {
                while ((index < length) && (line[index] != ' ') && valid) {
                    valid = (line[index] >= '!') && (line[index] <= '~');
                    ++index;
                }
                tokens[count].data = &line[start];
                tokens[count].length = index - start;
                ++count;
            }
        }
    }
    return valid ? count : 0u;
}

static StatusType reject(Ecu_HostBatch *batch, StatusType status)
{
    if (batch->open != 0u) {
        batch->rejected = 1u;
    }
    return status;
}

static StatusType begin_batch(Ecu_HostBatch *batch,
                              unsigned count,
                              const Ecu_BatchTokenType tokens[7],
                              Ecu_BatchCommand *command)
{
    uint64_t at;
    StatusType status = E_OS_VALUE;
    if (batch->open != 0u) {
        status = reject(batch, E_OS_STATE);
    } else if (count == 2u) {
        boolean valid = number(tokens[1], UINT64_MAX, &at);
        if (valid && (at >= batch->completed_epoch) &&
            ((at - batch->completed_epoch) <= ECU_BATCH_MAX_SPAN)) {
            batch->epoch = at;
            batch->count = 0u;
            batch->open = 1u;
            batch->rejected = 0u;
            *command = ECU_BATCH_BEGIN;
            status = E_OK;
        }
    } else {
        /* BEGIN requires exactly one epoch. */
    }
    return status;
}

static StatusType commit_batch(Ecu_HostBatch *batch, unsigned count, Ecu_BatchCommand *command)
{
    StatusType status = E_OS_STATE;
    if (batch->open != 0u) {
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
            status = E_OK;
        }
        batch->open = 0u;
        if (status != E_OK) {
            batch->count = 0u;
            batch->rejected = 0u;
        }
    }
    return status;
}

static StatusType stage_input(Ecu_HostBatch *batch,
                              unsigned count,
                              const Ecu_BatchTokenType tokens[7],
                              Ecu_BatchCommand kind,
                              Ecu_BatchCommand *command)
{
    Ecu_BatchFrame frame = {0};
    StatusType status = E_OS_VALUE;
    if (batch->open == 0u) {
        status = E_OS_STATE;
    } else if (batch->rejected != 0u) {
        status = E_OS_VALUE;
    } else if (batch->count >= ECU_BATCH_CAPACITY) {
        status = reject(batch, E_OS_LIMIT);
    } else {
        const boolean valid = (kind == ECU_BATCH_RX)
                                  ? ((count == 7u) && decode_frame(tokens, &frame))
                                  : ((count == 3u) && decode_mode(tokens, &frame));
        if (valid) {
            batch->frames[batch->count] = frame;
            ++batch->count;
            *command = kind;
            status = E_OK;
        } else {
            status = reject(batch, E_OS_VALUE);
        }
    }
    return status;
}

static StatusType dispatch_batch_command(Ecu_HostBatch *batch,
                                         unsigned count,
                                         const Ecu_BatchTokenType tokens[7],
                                         Ecu_BatchCommand *command)
{
    StatusType status = E_OS_VALUE;
    if (count == 0u) {
        status = reject(batch, E_OS_VALUE);
    } else if (batch->negotiated == 0u) {
        if ((count == 2u) && (equals(tokens[0], "PROTOCOL", 8u) != 0) &&
            (equals(tokens[1], ECU_BATCH_PROTOCOL, sizeof(ECU_BATCH_PROTOCOL) - 1u) != 0)) {
            batch->negotiated = 1u;
            *command = ECU_BATCH_NEGOTIATE;
            status = E_OK;
        }
    } else if (equals(tokens[0], "BEGIN", 5u) != 0) {
        status = begin_batch(batch, count, tokens, command);
    } else if (equals(tokens[0], "COMMIT", 6u) != 0) {
        status = commit_batch(batch, count, command);
    } else if (equals(tokens[0], "RX", 2u) != 0) {
        status = stage_input(batch, count, tokens, ECU_BATCH_RX, command);
    } else if (equals(tokens[0], "MODE", 4u) != 0) {
        status = stage_input(batch, count, tokens, ECU_BATCH_MODE, command);
    } else {
        status = reject(batch, E_OS_VALUE);
    }
    return status;
}

/* MR12 RULE 8.7 VIOLATION: the probe delivery retains the host-batch library export. */
StatusType Ecu_HostBatchParse(Ecu_HostBatch *batch,
                              const char *line,
                              size_t length,
                              Ecu_BatchCommand *command)
{
    StatusType status;
    if ((batch == NULL) || (line == NULL) || (command == NULL)) {
        status = E_OS_ILLEGAL_ADDRESS;
    } else if (batch->executing != 0u) {
        status = E_OS_STATE;
    } else if ((length == 0u) || (length > ECU_BATCH_LINE_CAPACITY)) {
        status = reject(batch, E_OS_VALUE);
    } else {
        Ecu_BatchTokenType tokens[7];
        unsigned count = tokenize(line, length, tokens);
        status = dispatch_batch_command(batch, count, tokens, command);
    }
    return status;
}

StatusType Ecu_HostBatchComplete(Ecu_HostBatch *batch)
{
    StatusType status;
    if (batch == NULL) {
        status = E_OS_ILLEGAL_ADDRESS;
    } else if (batch->executing == 0u) {
        status = E_OS_STATE;
    } else {
        batch->completed_epoch = batch->epoch;
        ++batch->batch_id;
        if (batch->input_sequences_reserved == 0u) {
            batch->first_input_sequence =
                (batch->count == 0u) ? UINT64_C(0) : (batch->sequence + UINT64_C(1));
            batch->sequence += (uint64_t)batch->count;
        }
        ++batch->sequence;
        batch->executing = 0u;
        status = E_OK;
    }
    return status;
}
