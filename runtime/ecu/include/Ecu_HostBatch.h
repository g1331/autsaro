/** @file
 * @brief HostBatchV1 bounded native staging; no automotive state access.
 */
#ifndef ECU_HOST_BATCH_H
#define ECU_HOST_BATCH_H

#include "Os.h"
#include "Ecu_Status.h"
#include <stddef.h>
#include <stdint.h>

#define ECU_BATCH_CAPACITY 256u
#define ECU_BATCH_MAX_SPAN UINT64_C(1000)
#define ECU_BATCH_LINE_CAPACITY 128u
#define ECU_BATCH_WATCHDOG_MS 5000u

typedef struct {
    uint32_t id;
    uint8_t dlc;
    uint8_t data[8];
} Ecu_BatchFrame;

typedef enum { ECU_BATCH_BEGIN, ECU_BATCH_RX, ECU_BATCH_COMMIT } Ecu_BatchCommand;

/** One native producer owns this staging state. Initialize before use.
 * Completed epoch changes only through Ecu_HostBatchComplete after execution.
 */
typedef struct {
    uint64_t completed_epoch;
    uint64_t epoch;
    uint64_t batch_id;
    uint64_t sequence;
    uint64_t first_input_sequence;
    uint16_t count;
    uint8_t open;
    uint8_t rejected;
    uint8_t executing;
    uint8_t input_sequences_reserved;
    EcuStatus input_status;
    EcuStatus transport_status;
    uint64_t transport_epoch;
    uint16_t transport_count;
    Ecu_BatchFrame frames[ECU_BATCH_CAPACITY];
} Ecu_HostBatch;

/** Initialize caller-owned native staging with epoch/id/sequence zero.
 * @param batch Nonnull caller-owned staging.
 */
void Ecu_HostBatchInitialize(Ecu_HostBatch *batch);
/** Decode one exact ASCII command without executing automotive work.
 * @param batch Single native producer's initialized staging.
 * @param line Complete caller-owned line, without CR/LF terminators.
 * @param length Exact byte count; embedded NUL/control bytes refuse.
 * @param command Decoded command, unchanged on refusal.
 * @return E_OK or E_OS_VALUE/LIMIT/STATE/ILLEGAL_ADDRESS. Any malformed
 * active batch stays rejected until COMMIT discards it; no prefix executes.
 */
StatusType Ecu_HostBatchParse(Ecu_HostBatch *batch, const char *line, size_t length,
                              Ecu_BatchCommand *command);
/** Publish successful execution after a real complete/quiescent boundary.
 * @param batch Executing single-producer staging.
 * @return E_OK or E_OS_STATE/ILLEGAL_ADDRESS; refuses a nonexecuting batch.
 */
StatusType Ecu_HostBatchComplete(Ecu_HostBatch *batch);

#endif
