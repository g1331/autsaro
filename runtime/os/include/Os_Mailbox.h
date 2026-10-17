/** @file Single-producer native input mailbox between the host bridge and an owner Task. */
#ifndef AUTOSAR_OS_MAILBOX_H
#define AUTOSAR_OS_MAILBOX_H
#include "Os.h"
#define OS_INPUT_CAPACITY  256u /**< Maximum simultaneously queued input records. */
#define OS_INPUT_PAYLOAD   32u  /**< Maximum bytes in one input record. */
#define OS_INPUT_INTERRUPT 30u  /**< Native vector reserved for mailbox notification. */
/** Owned mailbox record copied between the native producer and configured automotive owner Task. */
typedef struct {
    uint64 ticket;                /**< Accepted monotonic input ticket. */
    uint8 length;                 /**< Payload bytes, 1 through OS_INPUT_PAYLOAD. */
    uint8 data[OS_INPUT_PAYLOAD]; /**< Owned payload storage; only length bytes contain the
                                     submitted input. */
} Os_InputRecord;
/** Copy one input record from the single native bridge producer.
 * @param[in] data Caller-owned bytes, copied before acceptance; never retained.
 * @param[in] length Number of bytes, 1 through OS_INPUT_PAYLOAD.
 * @param[out] ticket Accepted monotonic ticket; unchanged on rejection.
 * @return E_OK, E_OS_LIMIT (full/ticket exhausted), E_OS_ACCESS (other producer),
 * E_OS_CALLEVEL (Task/ISR), E_OS_STATE (not ready/closed/disabled),
 * E_OS_VALUE or E_OS_ILLEGAL_ADDRESS. No old record is discarded.
 * Preconditions: ready mailbox configuration and the bound native producer; documented refusals
 * preserve the queue. Context: single native producer, outside OS/BSW critical or exclusive areas.
 * Reentrancy: non-reentrant for the producer; atomic publication coordinates with the owner Task.
 * Ownership: copies data before acceptance and writes ticket on success; neither pointer is
 * retained.
 */
StatusType Os_TargetPostInput(const uint8 *data, uint8 length, uint64 *ticket);
/** Consume the oldest complete input on the configured owner Task.
 * @param[out] record Output copy; unchanged when no record or on rejection.
 * @return E_OK, E_OS_NOFUNC (empty), E_OS_ACCESS (other Task),
 * E_OS_CALLEVEL, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: ready mailbox configuration and the configured owner Task; refusal preserves
 * output. Context: configured automotive owner Task, outside BSW exclusive areas. Reentrancy:
 * owner-only operation; producer/consumer publication is atomic. Ownership: caller owns output
 * storage, borrowed until return and written only on success.
 */
StatusType Os_TargetTakeInput(Os_InputRecord *record);
/** Inspect published input count on the configured owner Task.
 * @param[out] count Output count, unchanged on rejection.
 * @return E_OK, E_OS_ACCESS, E_OS_CALLEVEL, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: ready mailbox configuration and the configured owner Task; refusal preserves
 * output. Context: configured automotive owner Task, outside BSW exclusive areas. Reentrancy:
 * owner-only operation; producer/consumer publication is atomic. Ownership: caller owns output
 * storage, borrowed until return and written only on success.
 */
StatusType Os_TargetInputCount(uint32 *count);
#endif
