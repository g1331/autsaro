#ifndef AUTOSAR_EPIC4_OS_MAILBOX_H
#define AUTOSAR_EPIC4_OS_MAILBOX_H
#include "Os.h"
#define OS_INPUT_CAPACITY 256u
#define OS_INPUT_PAYLOAD 32u
#define OS_INPUT_INTERRUPT 30u
typedef struct {
    uint64_t ticket;
    uint8_t length;
    uint8_t data[OS_INPUT_PAYLOAD];
} Os_InputRecord;
/** Copy one input record from the single native bridge producer.
 * @param data Caller-owned bytes, copied before acceptance; never retained.
 * @param length Number of bytes, 1 through OS_INPUT_PAYLOAD.
 * @param ticket Accepted monotonic ticket; unchanged on rejection.
 * @return E_OK, E_OS_LIMIT (full/ticket exhausted), E_OS_ACCESS (other producer),
 * E_OS_CALLEVEL (Task/ISR), E_OS_STATE (not ready/closed/disabled),
 * E_OS_VALUE or E_OS_ILLEGAL_ADDRESS. No old record is discarded.
 */
StatusType Os_TargetPostInput(const uint8_t *data, uint8_t length, uint64_t *ticket);
/** Consume the oldest complete input on the configured owner Task.
 * @param record Output copy; unchanged when no record or on rejection.
 * @return E_OK, E_OS_NOFUNC (empty), E_OS_ACCESS (other Task),
 * E_OS_CALLEVEL, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Os_TargetTakeInput(Os_InputRecord *record);
/** Inspect published input count on the configured owner Task.
 * @param count Output count, unchanged on rejection.
 * @return E_OK, E_OS_ACCESS, E_OS_CALLEVEL, E_OS_STATE or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Os_TargetInputCount(unsigned *count);
#endif
