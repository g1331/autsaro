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

int main(void) {
    unsigned index;
    static const char *const malformed[] = {
        "RX 800 9 000000000000000000",
        "RX 2048 4 78563412",
        "RX -1 4 78563412",
        "RX 800 0 00",
        "RX 800 4 7856341",
        "RX 800 4 78563412ff",
        "RX 800 4 7856341z",
        "RX 800 4 78563412 tail",
        "RX 800 4",
        "RX 0x 4 78563412",
        "RX 800 4.0 78563412",
        "RX 18446744073709551616 4 78563412",
        "RX 800 -4 78563412",
        "UNKNOWN",
        "BEGIN 1",
    };
    Ecu_HostBatchInitialize(&batch);
    assert(line("BEGIN 0") == E_OK);
    for (index = 0u; index < 256u; ++index) {
        assert(line("RX 0x320 4 78563412") == E_OK);
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
        assert(line("RX 800 4 78563412") == E_OK);
    }
    assert(line("RX 800 4 78563412") == E_OS_LIMIT);
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
        assert(Ecu_HostBatchParse(&batch, "BEGIN 0", 129u, &command) == E_OS_VALUE);
        assert(command == ECU_BATCH_RX);
    }
    return (printf(
                "host_batch_codec boundaries=256/257,1000/1001 numeric/sequence/recovery=pass\n") <
            0)
               ? 93
               : 0;
}
