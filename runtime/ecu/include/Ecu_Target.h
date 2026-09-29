/** @file
 * @brief Controlled single-owner Windows ECU target boundary.
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

/** The sole application's committed snapshot and most recent standard status.
 * Read/write statuses describe the last periodic attempt; failed writes retain
 * the previously committed value and epoch. Only the owner may inspect it.
 */
typedef struct {
    uint32_t value;
    uint64_t epoch;
    Std_ReturnType read_status;
    Std_ReturnType write_status;
} Ecu_ApplicationState;
/** Inspect one coherent application state on the automotive owner.
 * @param result Nonnull caller storage, unchanged on refusal.
 * @return E_OK on the owner, E_NOT_OK for other contexts or a null output.
 */
Std_ReturnType Ecu_ApplicationInspect(Ecu_ApplicationState *result);

#define ECU_TARGET_UNPREPARED 0u
#define ECU_TARGET_INITIALIZING 1u
#define ECU_TARGET_READY 2u
#define ECU_TARGET_FAILED 3u
#define ECU_TARGET_CLOSED 4u
#define ECU_TARGET_OUTPUT_CAPACITY 256u

/** One immutable copied output; acknowledgment includes both identity fields. */
typedef struct {
    uint64_t ticket;
    uint64_t epoch;
    uint32_t can_id;
    PduIdType pdu;
    uint8_t dlc;
    uint8_t data[8];
} Ecu_OutputRecord;

/** Immutable owner-published transport failure; ordinary timeouts recover. */
typedef struct {
    uint64_t epoch;
    EcuStatus status;
} Ecu_ProtocolRecord;

/** Copy and retire the next transport failure on the sole native consumer.
 * @param record Caller-owned storage, unchanged on empty/context refusal.
 * @return E_OK, E_OS_NOFUNC or standard context/pointer errors.
 * Queue overflow closes the target; records are never silently discarded.
 */
StatusType Ecu_TargetTakeProtocolFailure(Ecu_ProtocolRecord *record);

/** Completion published at the real owner waiting/empty boundary. */
typedef struct {
    uint64_t ticket;
    uint64_t epoch;
    uint16_t input_count;
    EcuStatus input_status;
} Ecu_BatchCompletion;

/** Native output operation: nonzero means the actual write/flush succeeded.
 * A NULL output is the final completed/error batch receipt.
 * @param output Immutable copied frame, or NULL for final receipt.
 * @param batch Native staging/sequence and actual completed epoch.
 * @param status Actual execution status.
 * @param context Caller-owned sink context, retained only during execution.
 * @return Nonzero for successful physical host output, zero for failure.
 */
typedef int (*Ecu_HostSink)(const Ecu_OutputRecord *output, const Ecu_HostBatch *batch,
                            StatusType status, void *context);

/** Execute one previously decoded complete batch on its sole native producer.
 * @param batch Valid initialized staging in executing state.
 * @param sink Actual output/receipt writer; called outside automotive tasks.
 * @param context Caller-owned callback context.
 * @return Standard admission/execution status. Output/watchdog faults close
 * the process through the existing target fault control and never roll back.
 */
StatusType Ecu_HostBatchExecute(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context);

/** Validate every immutable frame before any automotive time/state change.
 * @param frames Caller-owned array; NULL only for count zero.
 * @param count Zero through256.
 * @return E_OK or standard pointer/value/capacity refusal.
 */
StatusType Ecu_TargetValidateFrames(const Ecu_BatchFrame *frames, uint16_t count);
/** Native fault control for failed host output or its watchdog.
 * @param reason Standard non-success shutdown reason.
 * Does not return; requests existing shutdown and bounds failed host cleanup.
 */
void Ecu_TargetAbortNative(StatusType reason);

/** Prepare the generated process-local configuration once, before StartOS.
 * @return Standard configuration/state status; creates no OS threads.
 */
StatusType Ecu_TargetPrepare(void);
/** Read initialization/ready/failure publication from native control.
 * @return READY only when both initialization and OS release are complete.
 */
uint8_t Ecu_TargetState(void);
/** Validate the current initialization thread or generated owner Task.
 * @return Nonzero only for the sole BSW/SWC owner.
 */
int Ecu_TargetIsOwner(void);
/** Enforce the single-owner proof at an internal BSW/SchM boundary. */
void Ecu_TargetAssertOwner(void);
/** Read the owner's explicitly delivered logical epoch.
 * @return Logical milliseconds; caller must own the ECU state.
 */
uint64_t Ecu_TargetNow(void);
/** Copy a frame from native control into the existing OS input mailbox.
 * @param at Last completed logical epoch or its successor, never below an
 * already accepted frame epoch. Frames refuse while a tick is unconfirmed.
 * @param id Standard Classical CAN identifier.
 * @param dlc Length, 1 through 8.
 * @param data Caller-owned frame copied before acceptance.
 * @param ticket Accepted mailbox ticket; unchanged on refusal.
 * @return Standard mailbox/input status; never discards older input.
 */
StatusType Ecu_TargetPostFrame(uint64_t at, uint32_t id, uint8_t dlc, const uint8_t data[8],
                               uint64_t *ticket);
/** Copy one complete batch before notifying the automotive owner.
 * @param at Last completed logical epoch or its successor.
 * @param frames Caller-owned frames; NULL allowed only for count zero.
 * @param count Zero through256; whole validation precedes notification.
 * @param ticket Accepted batch ticket, unchanged on refusal.
 * @return Standard value/state/capacity/context status. One batch in flight.
 */
StatusType Ecu_TargetPostBatch(uint64_t at, const Ecu_BatchFrame *frames, uint16_t count,
                               uint64_t *ticket);
/** Read a completed batch after the owner is waiting with no pending IO.
 * @param ticket Accepted batch identity.
 * @param result Complete immutable record; unchanged until completion.
 * @return E_OK, E_OS_NOFUNC, E_OS_ID or standard context/pointer errors.
 */
StatusType Ecu_TargetBatchCompletion(uint64_t ticket, Ecu_BatchCompletion *result);
/** Target-only backend waiting callback; executes in its existing critical
 * transition, without host IO or advancing automotive time.
 * @param id Actual waiting Task.
 * @param pending Remaining event bits.
 * @param predicate Actual WaitEvent predicate.
 */
void Ecu_TargetOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);
/** Copy the next published output from the sole native control consumer.
 * @param record Caller-owned output, unchanged when empty or rejected.
 * @return E_OK, E_OS_NOFUNC, E_OS_ACCESS, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Ecu_TargetTakeOutput(Ecu_OutputRecord *record);
/** Submit a successful native output confirmation, in output order.
 * @param ticket Previously taken output ticket.
 * @param pdu Matching software PDU handle.
 * @return Standard mailbox/status result; duplicates and mismatches refuse.
 */
StatusType Ecu_TargetConfirmOutput(uint64_t ticket, PduIdType pdu);
/** Publish a copy on the owner; called by the target Can Driver variant.
 * @param pdu Software PDU handle retained by Can_Write.
 * @param id CAN identifier.
 * @param dlc Payload length, 1 through 8.
 * @param data Payload copied before publication.
 * @return ECU_OK for enqueue only; confirmation remains a separate message.
 */
EcuStatus Ecu_TargetEnqueueTransmit(PduIdType pdu, uint32_t id, uint8_t dlc, const uint8_t data[8]);
/** Generated sole extended Task entry. */
void Ecu_TargetTask(void);
/** Generated RTE/application initialization; executed only by StartupHook.
 * @return E_OK after explicit initialization, otherwise E_NOT_OK.
 */
uint8_t Ecu_TargetInitializeRte(void);
/** Record a valid signal receive for standard RTE status mapping.
 * @param at Explicit receive epoch, supplied by the owner.
 */
void Ecu_TargetRecordReceive(uint64_t at);
/** Query initial/valid/expired receive state on the owner.
 * @return Standard RTE status defined by generated Rte.h.
 */
uint8_t Ecu_TargetReceiveStatus(void);

#endif
