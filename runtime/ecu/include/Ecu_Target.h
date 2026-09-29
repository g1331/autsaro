/** @file
 * @brief Controlled single-owner Windows ECU target boundary.
 */
#ifndef ECU_TARGET_H
#define ECU_TARGET_H

#include "ComStack_Types.h"
#include "Ecu_Status.h"
#include "Os_Target.h"
#include <stddef.h>
#include <stdint.h>

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
