#include "Ecu_HostBatch.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static Ecu_HostBatch batch;

static StatusType line(const char *text) {
    Ecu_BatchCommand command = ECU_BATCH_RX;
    StatusType status = Ecu_HostBatchParse(&batch, text, strlen(text), &command);
    if (status != E_OK) {
        assert(command == ECU_BATCH_RX);
    }
    return status;
}

static void reject_invalid_arguments(void)
{
    Ecu_BatchCommand command = ECU_BATCH_RX;
    assert(line("BEGIN 3") == E_OK);
    assert(Ecu_HostBatchParse(NULL, "COMMIT", 6u, &command) == E_OS_ILLEGAL_ADDRESS);
    assert(Ecu_HostBatchParse(&batch, NULL, 6u, &command) == E_OS_ILLEGAL_ADDRESS);
    assert(Ecu_HostBatchParse(&batch, "COMMIT", 6u, NULL) == E_OS_ILLEGAL_ADDRESS);
    assert(command == ECU_BATCH_RX);
    assert(batch.open == 1u && batch.rejected == 0u && batch.executing == 0u);
    assert(batch.epoch == UINT64_C(3) && batch.count == 0u && batch.sequence == UINT64_C(0));
    assert(line("COMMIT") == E_OK);
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
}

static void reject_line(const char *text, StatusType expected)
{
    Ecu_BatchCommand command = ECU_BATCH_BEGIN;
    assert(Ecu_HostBatchParse(&batch, text, strlen(text), &command) == expected);
    assert(command == ECU_BATCH_BEGIN);
}

static void reject_whitespace_in_open_batch(void)
{
    reject_line("COMMIT", E_OS_STATE);
    reject_line("   ", E_OS_VALUE);
    assert(batch.open == 0u && batch.rejected == 0u);
    assert(line("BEGIN 4") == E_OK);
    assert(line("RX 0 STANDARD 0 800 4 78563412") == E_OK);
    reject_line("   ", E_OS_VALUE);
    assert(batch.open == 1u && batch.rejected == 1u && batch.count == 1u);
    reject_line("RX 0 STANDARD 0 801 4 12345678", E_OS_VALUE);
    assert(batch.count == 1u && batch.frames[0].id == 800u);
    reject_line("COMMIT", E_OS_VALUE);
    assert(batch.open == 0u && batch.rejected == 0u && batch.executing == 0u);
    assert(batch.count == 0u && batch.sequence == UINT64_C(1));
    assert(batch.completed_epoch == UINT64_C(3) && batch.batch_id == UINT64_C(1));
    assert(line("BEGIN 4") == E_OK);
    assert(line("COMMIT") == E_OK);
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
    assert(batch.completed_epoch == UINT64_C(4) && batch.batch_id == UINT64_C(2));
}

static void verify_v2_identity(void)
{
    Ecu_HostBatchInitialize(&batch);
    reject_line("BEGIN 0", E_OS_VALUE);
    reject_line("PROTOCOL HostBatchV1", E_OS_VALUE);
    assert(line("PROTOCOL HostBatchV2") == E_OK);
    assert(line("BEGIN 0") == E_OK);
    assert(line("RX 0 STANDARD 0 2047 0 -") == E_OK);
    assert(line("RX 0 EXTENDED 0 536870911 1 af") == E_OK);
    assert(batch.frames[0].id == 0x7ffu && batch.frames[0].dlc == 0u);
    assert(batch.frames[1].id == 0x9fffffffu && batch.frames[1].data[0] == 0xafu);
    assert(line("COMMIT") == E_OK);
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
}

int main(void) {
    verify_v2_identity();
    unsigned index;
    static const char *const malformed[] = {
        "RX",
        "RX 0 STANDARD 0 bad 1 aa",
        "RX 0 STANDARD 0 1 bad AA",
        "RX 0 STANDARD 0 800 9 000000000000000000",
        "RX 0 STANDARD 0 2048 4 78563412",
        "RX 0 STANDARD 0 -1 4 78563412",
        "RX 0 STANDARD 0 800 0 00",
        "RX 0 STANDARD 0 800 4 7856341",
        "RX 0 STANDARD 0 800 4 78563412ff",
        "RX 0 STANDARD 0 800 4 7856341z",
        "RX 0 STANDARD 0 800 4 78563412 tail",
        "RX 0 STANDARD 0 800 4",
        "RX 0 STANDARD 0 0x 4 78563412",
        "RX 0 STANDARD 0 800 4.0 78563412",
        "RX 0 STANDARD 0 18446744073709551616 4 78563412",
        "RX 0 STANDARD 0 800 -4 78563412",
        "UNKNOWN",
        "BEGIN 1",
    };
    Ecu_HostBatchInitialize(&batch);
    assert(line("PROTOCOL HostBatchV2") == E_OK);
    assert(line("BEGIN") == E_OS_VALUE);
    assert(batch.open == 0u && batch.count == 0u);
    assert(line("BEGIN 0") == E_OK);
    for (index = 0u; index < 256u; ++index) {
        assert(line("RX 0 STANDARD 0 800 4 78563412") == E_OK);
    }
    assert(batch.count == 256u);
    assert(batch.frames[255].id == 800u && batch.frames[255].dlc == 4u);
    assert(batch.frames[255].data[0] == 0x78u && batch.frames[255].data[3] == 0x12u);
    assert(batch.completed_epoch == UINT64_C(0) && batch.batch_id == UINT64_C(0));
    assert(line("COMMIT") == E_OK);
    assert(line("BEGIN 1") == E_OS_STATE);
    assert(batch.count == 256u && batch.sequence == UINT64_C(0));
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
    assert(batch.batch_id == UINT64_C(1) && batch.sequence == UINT64_C(257));
    assert(Ecu_HostBatchComplete(&batch) == E_OS_STATE);
    assert(line("BEGIN 0") == E_OK);
    for (index = 0u; index < 256u; ++index) {
        assert(line("RX 0 STANDARD 0 800 4 78563412") == E_OK);
    }
    assert(line("RX 0 STANDARD 0 800 4 78563412") == E_OS_LIMIT);
    assert(line("COMMIT") == E_OS_VALUE);
    assert(batch.count == 0u && batch.batch_id == UINT64_C(1));
    for (index = 0u; index < sizeof(malformed) / sizeof(malformed[0]); ++index) {
        assert(line("BEGIN 0") == E_OK);
        assert(line(malformed[index]) != E_OK);
        assert(line("COMMIT") == E_OS_VALUE);
        assert(batch.completed_epoch == UINT64_C(0) && batch.batch_id == UINT64_C(1));
    }
    assert(line("BEGIN 1001") == E_OS_VALUE);
    assert(line("BEGIN 18446744073709551616") == E_OS_VALUE);
    assert(line("BEGIN -1") == E_OS_VALUE);
    assert(line("BEGIN +1") == E_OS_VALUE);
    assert(line("BEGIN 0x1") == E_OS_VALUE);
    assert(line("BEGIN 1000") == E_OK);
    assert(line("COMMIT") == E_OK);
    assert(batch.completed_epoch == UINT64_C(0));
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
    assert(batch.completed_epoch == UINT64_C(1000));
    assert(line("BEGIN 999") == E_OS_VALUE);
    assert(line("BEGIN 1000") == E_OK);
    assert(line("COMMIT extra") == E_OS_VALUE);
    assert(batch.completed_epoch == UINT64_C(1000));
    Ecu_HostBatchInitialize(&batch);
    assert(line("PROTOCOL HostBatchV2") == E_OK);
    batch.sequence = UINT64_MAX;
    assert(line("BEGIN 0") == E_OK);
    assert(line("COMMIT") == E_OS_LIMIT);
    assert(batch.sequence == UINT64_MAX && batch.batch_id == UINT64_C(0));
    batch.sequence = UINT64_MAX - UINT64_C(1);
    assert(line("BEGIN 0") == E_OK);
    assert(line("COMMIT") == E_OK);
    assert(Ecu_HostBatchComplete(&batch) == E_OK);
    assert(batch.sequence == UINT64_MAX);
    batch.batch_id = UINT64_MAX;
    assert(line("BEGIN 0") == E_OK);
    assert(line("COMMIT") == E_OS_LIMIT);
    assert(batch.batch_id == UINT64_MAX);
    {
        Ecu_BatchCommand command = ECU_BATCH_RX;
        const char invalid[] = {'R', 'X', '\0'};
        assert(Ecu_HostBatchParse(&batch, invalid, sizeof(invalid), &command) == E_OS_VALUE);
        assert(Ecu_HostBatchParse(&batch, "BEGIN 0", ECU_BATCH_LINE_CAPACITY + 1u, &command) ==
               E_OS_VALUE);
        assert(command == ECU_BATCH_RX);
    }
    Ecu_HostBatchInitialize(&batch);
    assert(line("PROTOCOL HostBatchV2") == E_OK);
    reject_invalid_arguments();
    reject_whitespace_in_open_batch();
    return (printf(
                "host_batch_codec boundaries=256/257,1000/1001 numeric/sequence/recovery=pass\n") <
            0)
               ? 93
               : 0;
}
