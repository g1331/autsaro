/* Independent native consumer. The actual bridge writes/flushes before acknowledging IO. */
#include "Ecu_Target.h"
#include "Ecu_Config.h"
#include "Os_Host.h"
#include <stdio.h>
#include <string.h>

extern const EcuConfig Ecu_Config;
static uint8_t outputs[32][8];
static uint8_t lengths[32];
static size_t count;
static unsigned held_receipts;
static Ecu_HostBatch batch;

static void require_at(int accepted, unsigned line) {
    if (accepted == 0) {
        (void)fprintf(stderr, "queued profile assertion: line=%u\n", line);
        ShutdownOS(E_OS_STATE);
    }
}
#define require(accepted) require_at((accepted), __LINE__)

static int sink(const Ecu_OutputRecord *output, const Ecu_HostBatch *staging, StatusType status,
                void *context) {
    (void)staging;
    (void)status;
    (void)context;
    if (output != NULL) {
        Os_TickCompletion completion;
        /* A queued send cannot publish its epoch receipt before physical output succeeds. */
        require(Os_TargetTickCompletion(output->epoch, &completion) == E_OS_NOFUNC);
        ++held_receipts;
        if (output->can_id == 0x708u) {
            require((count < 32u) && (output->dlc == 8u));
            memcpy(outputs[count], output->data, 8u);
            lengths[count] = output->dlc;
            ++count;
        }
        if (printf("queued_output epoch=%llu id=%u dlc=%u\n", (unsigned long long)output->epoch,
                   output->can_id, output->dlc) < 0) {
            return 0;
        }
    }
    return fflush(stdout) == 0;
}

static StatusType receive(const uint8_t bytes[8], uint64_t span) {
    char line[ECU_BATCH_LINE_CAPACITY];
    Ecu_BatchCommand command;
    int length = snprintf(line, sizeof(line), "BEGIN %llu",
                          (unsigned long long)(batch.completed_epoch + span));
    require((length > 0) && ((size_t)length < sizeof(line)));
    require(Ecu_HostBatchParse(&batch, line, (size_t)length, &command) == E_OK);
    require(command == ECU_BATCH_BEGIN);
    length = snprintf(line, sizeof(line), "RX 1792 8 %02X%02X%02X%02X%02X%02X%02X%02X", bytes[0],
                      bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]);
    require((length > 0) && ((size_t)length < sizeof(line)));
    require(Ecu_HostBatchParse(&batch, line, (size_t)length, &command) == E_OK);
    require(command == ECU_BATCH_RX);
    require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
    require(command == ECU_BATCH_COMMIT);
    return Ecu_HostBatchExecute(&batch, sink, NULL);
}

static void frame(size_t index, const uint8_t expected[8]) {
    require((index < count) && (lengths[index] == 8u));
    require(memcmp(outputs[index], expected, 8u) == 0);
}

static Os_HostThreadResult control(void *argument) {
    static const uint8_t read_did[8] = {3u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    static const uint8_t value[8] = {7u, 0x62u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    static const uint8_t unknown_info[8] = {5u, 0x22u, 0xdeu, 0xadu, 0xf1u, 0x86u, 0u, 0u};
    static const uint8_t info_reply[8] = {4u, 0x62u, 0xf1u, 0x86u, 1u, 0u, 0u, 0u};
    static const uint8_t extended[8] = {2u, 0x10u, 3u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t session_reply[8] = {6u, 0x50u, 3u, 0u, 50u, 1u, 0xf4u, 0u};
    static const uint8_t ordered[8] = {5u, 0x22u, 0xf1u, 0x86u, 0x12u, 0x34u, 0u, 0u};
    static const uint8_t first[8] = {0x10u, 10u, 0x62u, 0xf1u, 0x86u, 3u, 0x12u, 0x34u};
    static const uint8_t tail[8] = {0x21u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t reverse[8] = {5u, 0x22u, 0x12u, 0x34u, 0xf1u, 0x86u, 0u, 0u};
    static const uint8_t reverse_first[8] = {0x10u, 10u, 0x62u, 0x12u, 0x34u, 0u, 0u, 0u};
    static const uint8_t reverse_tail[8] = {0x21u, 0u, 0xf1u, 0x86u, 3u, 0u, 0u, 0u};
    static const uint8_t clear[8] = {0x30u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t three[8] = {7u, 0x22u, 0xf1u, 0x86u, 0xf1u, 0x86u, 0xf1u, 0x86u};
    static const uint8_t too_many[8] = {3u, 0x7fu, 0x22u, 0x13u, 0u, 0u, 0u, 0u};
    static const uint8_t routine[8] = {4u, 0x31u, 1u, 0x12u, 0x34u, 0u, 0u, 0u};
    static const uint8_t unsupported[8] = {3u, 0x7fu, 0x31u, 0x11u, 0u, 0u, 0u, 0u};
    static const uint8_t wait[8] = {0x31u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t request_first[8] = {0x10u, 9u, 0x22u, 0xf1u, 0x86u, 0xf1u, 0x86u, 0xf1u};
    static const uint8_t request_tail[8] = {0x21u, 0x86u, 0xf1u, 0x86u, 0u, 0u, 0u, 0u};
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    Ecu_HostBatchInitialize(&batch);
    require(receive(read_did, 1u) == E_OK);
    frame(0u, value);
    require(receive(unknown_info, 1u) == E_OK);
    frame(1u, info_reply);
    require(receive(extended, 1u) == E_OK);
    frame(2u, session_reply);
    require(receive(ordered, 1u) == E_OK);
    frame(3u, first);
    require(receive(clear, 1u) == E_OK);
    frame(4u, tail);
    require(receive(reverse, 1u) == E_OK);
    frame(5u, reverse_first);
    require(receive(clear, 1u) == E_OK);
    frame(6u, reverse_tail);
    require(receive(three, 1u) == E_OK);
    frame(7u, too_many);
    require(receive(routine, 1u) == E_OK);
    frame(8u, unsupported);
    require(receive(ordered, 1u) == E_OK);
    frame(9u, first);
    require(receive(wait, 1u) == E_OS_VALUE);
    require((batch.input_status == ECU_ERR_TP_FLOW) && (count == 10u));
    require(receive(read_did, 1u) == E_OK);
    frame(10u, value);
    require(receive(request_first, 1u) == E_OK);
    require((count == 12u) && (outputs[11][0] == 0x30u));
    require(receive(request_tail, Ecu_Config.diagnostic->n_cr_ms) == E_OK);
    require((batch.transport_count == 0u) && (batch.input_status == ECU_OK));
    frame(12u, too_many);
    require(receive(read_did, 1u) == E_OK);
    frame(13u, value);
    require((count == 14u) && (held_receipts >= 14u));
    puts("SEMANTIC_ECU PASS queued/flush session/p2 order/max-two dlc/wait deadline/recovery");
    require(fflush(stdout) == 0);
    ShutdownOS(E_OK);
    return 0u;
}

int main(void) {
    Os_HostHandle thread;
    require(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(control, NULL);
    require(thread != NULL);
    require(Os_HostClose(thread) != 0);
    StartOS(1u);
    return 94;
}
