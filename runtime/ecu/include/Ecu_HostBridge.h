/** @file
 * @brief HostBatchV1 execution and receipt on the sole native producer.
 * The codec (Ecu_HostBatch.h) stages a batch; this bridge admits it, drives
 * the target owner tick by tick and publishes outputs and the final receipt.
 */
#ifndef ECU_HOST_BRIDGE_H
#define ECU_HOST_BRIDGE_H

#include "Ecu_HostBatch.h"
#include "Ecu_Target.h"

#define ECU_BATCH_WATCHDOG_MS 5000u /**< Maximum native progress wait, host milliseconds. */

/** Native output operation: nonzero means the actual write/flush succeeded.
 * A NULL output is the final completed/error batch receipt.
 * @param[in] output Immutable copied frame, or NULL for final receipt.
 * @param[in] batch Native staging/sequence and actual completed epoch.
 * @param[in] status Receipt status: E_OK for COMMIT_OK, E_OS_VALUE for COMMIT_ERROR.
 * @param[in,out] context Caller-owned sink context, retained only during execution.
 * @return Nonzero for successful physical host output, zero for failure.
 */
typedef int (*Ecu_HostSink)(const Ecu_OutputRecord *output,
                            const Ecu_HostBatch *batch,
                            StatusType status,
                            void *context);

/** Execute one previously decoded complete batch on its sole native producer.
 * @param[in,out] batch Valid initialized staging in executing state.
 * @param[in] sink Actual output/receipt writer; called outside automotive tasks.
 * @param[in,out] context Caller-owned callback context.
 * @return E_OK once the batch has executed and its receipt went to the sink,
 * whatever the receipt status. Any other status is an admission refusal: no
 * automotive state changed, the staged inputs are discarded and the sink was
 * not called. E_OS_ILLEGAL_ADDRESS and E_OS_STATE for a nonexecuting batch
 * leave the staging untouched. Output and watchdog faults close the process
 * through the target fault control instead of returning, and never roll back.
 * Preconditions: Initialized executing staging and actual output sink; invalid admission refuses.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same target or staging object.
 * Ownership: Borrows staging, sink and context through synchronous execution only; no pointer
 * retained.
 */
StatusType Ecu_HostBatchExecute(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context);

#endif
