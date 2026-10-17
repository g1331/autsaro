/** @file
 * @brief HostBatchV1 bounded native staging; no automotive state access.
 */
#ifndef ECU_HOST_BATCH_H
#define ECU_HOST_BATCH_H

#include "Os.h"
#include "Ecu_Status.h"
#include <stddef.h>
#include <stdint.h>

#define ECU_BATCH_CAPACITY      256u           /**< Maximum staged input frames. */
#define ECU_BATCH_MAX_SPAN      UINT64_C(1000) /**< Maximum batch time advance, ms. */
#define ECU_BATCH_LINE_CAPACITY 128u           /**< Maximum command length in bytes. */

/** One copied Classical CAN input staged before automotive admission. */
typedef struct {
    uint32_t id;     /**< 11-bit Classical CAN identifier. */
    uint8_t dlc;     /**< Payload length, 0..8; the ASCII codec accepts 1..8. */
    uint8_t data[8]; /**< Copied payload; bytes beyond dlc are ignored. */
} Ecu_BatchFrame;

/** HostBatchV1 command accepted by the bounded ASCII codec. */
typedef enum {
    ECU_BATCH_BEGIN, /**< Open a batch at an explicit logical epoch. */
    ECU_BATCH_RX,    /**< Append one copied input frame. */
    ECU_BATCH_COMMIT /**< Close staging and request whole-batch execution. */
} Ecu_BatchCommand;

/** One native producer owns this staging state. Initialize before use.
 * Completed epoch changes only through Ecu_HostBatchComplete after execution;
 * Ecu_HostBatchExecute (Ecu_HostBridge.h) owns execution and the receipt.
 */
typedef struct {
    uint64_t completed_epoch;         /**< Last completed logical epoch, ms; initially zero. */
    uint64_t epoch;                   /**< Requested current batch epoch, ms. */
    uint64_t batch_id;                /**< Completed batch count; initially zero. */
    uint64_t sequence;                /**< Last reserved input/output/receipt sequence. */
    uint64_t first_input_sequence;    /**< First reserved input sequence; zero for no inputs. */
    uint16_t count;                   /**< Staged frame count, 0..256. */
    uint8_t open;                     /**< Nonzero between accepted BEGIN and COMMIT. */
    uint8_t rejected;                 /**< Nonzero rejects the open batch until COMMIT. */
    uint8_t executing;                /**< Nonzero after valid COMMIT until completion/refusal. */
    uint8_t input_sequences_reserved; /**< Nonzero when bridge reserved all input sequences. */
    EcuStatus input_status;           /**< First actual input refusal, or ECU_OK. */
    EcuStatus transport_status;       /**< First observed transport failure, or ECU_OK. */
    uint64_t transport_epoch;         /**< Logical epoch of first transport failure, ms. */
    uint16_t transport_count; /**< Observed transport failure count; overflow closes target. */
    Ecu_BatchFrame frames[ECU_BATCH_CAPACITY]; /**< Native-owned copied frame storage. */
} Ecu_HostBatch;

/** Initialize caller-owned native staging with epoch/id/sequence zero.
 * @param[out] batch Caller-owned staging; NULL is ignored.
 * @par Return
 * None.
 * Preconditions: None; NULL is ignored.
 * Context: Native producer, outside exclusive areas.
 * Reentrancy: Not concurrent with another operation on the same staging object.
 * Ownership: Writes the complete object during the call; retains no pointer.
 */
void Ecu_HostBatchInitialize(Ecu_HostBatch *batch);
/** Decode one exact ASCII command without executing automotive work.
 * @param[in,out] batch Single native producer's initialized staging.
 * @param[in] line Complete caller-owned line, without CR/LF terminators.
 * @param[in] length Exact byte count; embedded NUL/control bytes refuse.
 * @param[out] command Decoded command, unchanged on refusal.
 * @return E_OK or E_OS_VALUE/LIMIT/STATE/ILLEGAL_ADDRESS. Any malformed
 * active batch stays rejected until COMMIT discards it; no prefix executes.
 * Preconditions: Initialized staging, not executing; null pointers or invalid input refuse.
 * Context: Sole native producer, outside exclusive areas.
 * Reentrancy: Not reentrant for the same staging object; independent objects may run concurrently.
 * Ownership: Borrows line until return; copies accepted frames; writes command only on success.
 */
StatusType Ecu_HostBatchParse(Ecu_HostBatch *batch,
                              const char *line,
                              size_t length,
                              Ecu_BatchCommand *command);
/** Publish successful execution after a real complete/quiescent boundary.
 * @param[in,out] batch Executing single-producer staging.
 * @return E_OK or E_OS_STATE/ILLEGAL_ADDRESS; refuses a nonexecuting batch.
 * Preconditions: Caller proved real execution/quiescence; staging executing, else refuses.
 * Context: Sole native producer, outside exclusive areas.
 * Reentrancy: Not reentrant for the same staging object.
 * Ownership: Updates completion epoch/identity/sequence on success; retains no pointer.
 */
StatusType Ecu_HostBatchComplete(Ecu_HostBatch *batch);

#endif
