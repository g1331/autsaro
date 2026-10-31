/** @file Process-lifetime COM lower-refusal records and sole native stderr reader. */
#ifndef ECU_COM_REJECTION_H
#define ECU_COM_REJECTION_H
#include "ComStack_Types.h"
#include "Std_Types.h"

#define ECU_COM_REJECTION_CAPACITY 4096u /**< Append-only process-lifetime slots; never reused. */

/** Copy one actual lower refusal into independently published immutable storage.
 * @param[in] id Original COM PDU handle.
 * @param[in] result Original non-E_OK lower result.
 * @par Return
 * None.
 * Preconditions: static zero initialization before actors start; no runtime reset.
 * Context: Task/ISR callback outside COM_STATE; no locks, allocation, waiting for writers or I/O.
 * Reentrancy: concurrent writers supported; capacity exhaustion latches incomplete.
 * Ownership: only scalar values retained; no caller storage retained.
 */
void Ecu_ComRecordRejection(PduIdType id, Std_ReturnType result);

/** Output newly published records once, skipping preempted writers without waiting.
 * @param[in] final TRUE on the normal completed-tick/batch boundary before shutdown; FALSE to poll.
 * @return 1 after complete output; 0 on output failure, requiring the caller's native failure path.
 * Preconditions: one native reader per process; final called once, then no further automotive work.
 * Context: native control only, outside exclusive areas; stderr I/O may block.
 * Reentrancy: not reentrant; reader owns delivered flags and stderr record serialization.
 * Ownership: records never reused; flags set only after all record bytes are written. Final emits
 * an END count; pending publications or capacity loss emit INCOMPLETE. Successful process exit is
 * also required for a complete diagnostic stream; a partial/failed stream is never complete.
 */
int Ecu_ComDrainRejections(boolean final);
#endif
