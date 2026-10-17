/** @file
 * @brief Controlled single-owner ECU target boundary (Windows and Linux hosts).
 */
#ifndef ECU_TARGET_H
#define ECU_TARGET_H

#include "ComStack_Types.h"
#include "Ecu_Status.h"
#include "Ecu_HostBatch.h"
#include "Std_Types.h"
#include "Os_Target.h"
#include <stddef.h>
#include <stdint.h>

#define ECU_TARGET_UNPREPARED      0u   /**< Process-local target not prepared. */
#define ECU_TARGET_INITIALIZING    1u   /**< Preparation/startup in progress. */
#define ECU_TARGET_READY           2u   /**< Bootstrap and OS release complete. */
#define ECU_TARGET_FAILED          3u   /**< Terminal fault state. */
#define ECU_TARGET_CLOSED          4u   /**< Orderly terminal shutdown. */
#define ECU_TARGET_OUTPUT_CAPACITY 256u /**< Slots per copied output/failure queue. */

/** One immutable copied output; acknowledgment includes both identity fields. */
typedef struct {
    uint64_t ticket; /**< Monotonic nonzero output or batch identity. */
    uint64_t epoch;  /**< Owner logical time in milliseconds. */
    uint32_t can_id; /**< Classical CAN identifier, 0..0x7ff. */
    PduIdType pdu;   /**< Software PDU handle required for acknowledgment. */
    uint8_t dlc;     /**< Valid payload bytes, 0..8. */
    uint8_t data[8]; /**< Copied Classical CAN payload; unused bytes zero. */
} Ecu_OutputRecord;

/** Immutable owner-published transport failure; ordinary timeouts recover. */
typedef struct {
    uint64_t epoch;   /**< Owner logical time in milliseconds. */
    EcuStatus status; /**< Actual transport failure at this epoch. */
} Ecu_ProtocolRecord;

/** Copy and retire the next transport failure on the sole native consumer.
 * @param[out] record Caller-owned storage, unchanged on empty/context refusal.
 * @return E_OK, E_OS_NOFUNC or standard context/pointer errors.
 * Queue overflow closes the target; records are never silently discarded.
 * Preconditions: Target READY; null storage or invalid native context refuses.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies the record only on success; no caller pointer is retained.
 */
StatusType Ecu_TargetTakeProtocolFailure(Ecu_ProtocolRecord *record);
/** Record a transport timeout at the current epoch from the owner context.
 * A call outside the owner or with a full queue closes the target.
 * @par Return
 * None.
 * Preconditions: Caller owns ECU state; invalid context or full queue closes the target.
 * Context: ECU initialization thread or generated owner task, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Publishes an immutable copied failure; no pointer is retained.
 */
void Ecu_TargetReportTransportTimeout(void);

/** Completion published at the real owner waiting/empty boundary. */
typedef struct {
    uint64_t ticket;        /**< Monotonic nonzero output or batch identity. */
    uint64_t epoch;         /**< Owner logical time in milliseconds. */
    uint16_t input_count;   /**< Number of frames admitted, 0..256. */
    EcuStatus input_status; /**< First input refusal, or ECU_OK. */
} Ecu_BatchCompletion;

/** Validate every immutable frame before any automotive time/state change.
 * @param[in] frames Caller-owned array; NULL only for count zero.
 * @param[in] count Zero through 256.
 * @return E_OK or standard pointer/value/capacity refusal.
 * Preconditions: None; invalid pointers, shape or count refuse before automotive work.
 * Context: Any caller, outside exclusive areas.
 * Reentrancy: Reentrant; reads immutable generated configuration.
 * Ownership: Array is borrowed only until return; never retained.
 */
StatusType Ecu_TargetValidateFrames(const Ecu_BatchFrame *frames, uint16_t count);
/** Native fault control for failed host output or its watchdog.
 * @param[in] reason Standard non-success shutdown reason.
 * Does not return; requests existing shutdown and bounds failed host cleanup.
 * @par Return
 * None.
 * Preconditions: Fault control after a native output/watchdog failure; E_OK maps to E_OS_STATE.
 * Context: Native control or its watchdog thread, outside exclusive areas.
 * Reentrancy: Concurrent native faults request the same process shutdown.
 * Ownership: No pointer is retained; requests shutdown and terminates without returning.
 */
void Ecu_TargetAbortNative(StatusType reason);

/** Prepare the generated process-local configuration once, before StartOS.
 * @return Standard configuration/state status; creates no OS threads.
 * Preconditions: Process-local target unprepared; repeated calls return E_OS_STATE.
 * Context: Native main before StartOS, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Uses immutable generated configuration for process lifetime; no caller pointer.
 */
StatusType Ecu_TargetPrepare(void);
/** Read initialization/ready/failure publication from native control.
 * @return READY only when both initialization and OS release are complete.
 * Preconditions: None.
 * Context: Native control or automotive owner, including inside exclusive areas.
 * Reentrancy: Reentrant; reads atomic lifecycle publications.
 * Ownership: No pointer is retained or output written.
 */
uint8_t Ecu_TargetState(void);
/** Validate the current initialization thread or generated owner Task.
 * @return Nonzero only for the sole BSW/SWC owner.
 * Preconditions: None.
 * Context: Current native or automotive actor, including inside exclusive areas.
 * Reentrancy: Reentrant; checks current actor identity.
 * Ownership: No pointer is retained or output written.
 */
int Ecu_TargetIsOwner(void);
/** Enforce the single-owner proof at an internal BSW/SchM boundary.
 * @par Return
 * None.
 * Preconditions: Caller must be initialization thread or ECU owner; otherwise closes target.
 * Context: Internal BSW/SchM boundary, including inside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: No pointer is retained.
 */
void Ecu_TargetAssertOwner(void);
/** Check trusted startup (TRUE) or shutdown (FALSE) OS hook context.
 * @param[in] starting TRUE requires StartupHook; FALSE requires ShutdownHook.
 * @return E_OK for the matching hook context; E_NOT_OK without changing lifecycle state.
 * Preconditions: None; a nonmatching startup/shutdown hook refuses.
 * Context: Current actor, including inside exclusive areas.
 * Reentrancy: Reentrant; reads actor-local hook context.
 * Ownership: No pointer is retained or output written.
 */
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting);
/** Read the owner's explicitly delivered logical epoch.
 * @return Logical milliseconds; caller must own the ECU state.
 * Preconditions: Caller owns ECU state; otherwise closes target.
 * Context: ECU owner or initialization thread, including inside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: No pointer is retained or output written.
 */
uint64_t Ecu_TargetNow(void);
/** Copy a frame from native control into the existing OS input mailbox.
 * @param[in] at Last completed logical epoch or its successor, never below an
 * already accepted frame epoch. Frames refuse while a tick is unconfirmed.
 * @param[in] id Standard Classical CAN identifier.
 * @param[in] dlc Length, 1 through 8.
 * @param[in] data Caller-owned frame copied before acceptance.
 * @param[out] ticket Accepted mailbox ticket; unchanged on refusal.
 * @return Standard mailbox/input status; never discards older input.
 * Preconditions: Target READY and no unconfirmed tick; invalid epoch/shape/context refuses.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies payload into mailbox before acceptance; writes ticket only on success.
 */
StatusType Ecu_TargetPostFrame(
    uint64_t at, uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t *ticket);
/** Copy one complete batch before notifying the automotive owner.
 * @param[in] at Last completed logical epoch or its successor.
 * @param[in] frames Caller-owned frames; NULL allowed only for count zero.
 * @param[in] count Zero through 256; whole validation precedes notification.
 * @param[out] ticket Accepted batch ticket, unchanged on refusal.
 * @return Standard value/state/capacity/context status. One batch in flight.
 * Preconditions: Target READY and no in-flight batch; epoch/count/shape/context must validate.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies all frames before notification; writes ticket only on acceptance.
 */
StatusType Ecu_TargetPostBatch(uint64_t at,
                               const Ecu_BatchFrame *frames,
                               uint16_t count,
                               uint64_t *ticket);
/** Read a completed batch after the owner is waiting with no pending IO.
 * @param[in] ticket Accepted batch identity.
 * @param[out] result Complete immutable record; unchanged until completion.
 * @return E_OK, E_OS_NOFUNC, E_OS_ID or standard context/pointer errors.
 * Preconditions: Target READY; accepted ticket required; pending completion returns E_OS_NOFUNC.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies result only on success and releases the batch slot; retains no pointer.
 */
StatusType Ecu_TargetBatchCompletion(uint64_t ticket, Ecu_BatchCompletion *result);
/** Copy the next published output from the sole native control consumer.
 * @param[out] record Caller-owned output, unchanged when empty or rejected.
 * @return E_OK, E_OS_NOFUNC, E_OS_ACCESS, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: Target READY; nonnull record and valid native context required.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies one immutable record on success; caller owns its copy after return.
 */
StatusType Ecu_TargetTakeOutput(Ecu_OutputRecord *record);
/** Submit a successful native output confirmation, in output order.
 * @param[in] ticket Previously taken output ticket.
 * @param[in] pdu Matching software PDU handle.
 * @return Standard mailbox/status result; duplicates and mismatches refuse.
 * Preconditions: Physical output succeeded; matching taken ticket/PDU in output order required.
 * Context: Sole bound native control thread, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Queues identity only; mailbox refusal leaves the taken output retryable.
 */
StatusType Ecu_TargetConfirmOutput(uint64_t ticket, PduIdType pdu);
/** Publish a copy on the owner; called by the target Can Driver variant.
 * @param[in] pdu Software PDU handle retained by Can_Write.
 * @param[in] id CAN identifier.
 * @param[in] dlc Payload length, 0 through 8.
 * @param[in] data Payload copied before publication.
 * @return ECU_OK for enqueue only; confirmation remains a separate message.
 * Preconditions: Owner required (else ECU_ERR_IO); bad shape/full queue/invalid token closes
 * target. Context: ECU initialization thread or generated owner task, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Copies payload and driver token before publication; no caller pointer retained.
 */
EcuStatus Ecu_TargetEnqueueTransmit(PduIdType pdu, uint32_t id, uint8_t dlc, const uint8_t data[8]);
/** Generated sole extended Task entry.
 * @par Return
 * None.
 * Preconditions: Generated OS configuration activated this sole extended task.
 * Context: Generated ECU owner task, outside exclusive areas.
 * Reentrancy: Not reentrant on the same process-local target.
 * Ownership: Consumes copied mailbox input and publishes output records; no caller storage.
 */
void Ecu_TargetTask(void);

#endif
