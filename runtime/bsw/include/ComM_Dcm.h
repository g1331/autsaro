/** @file R24-11 Dcm active and inactive diagnostic notifications to ComM. */
#ifndef COMM_DCM_H
#define COMM_DCM_H
#include "ComM.h"
/** Keep the channel requested while diagnosis is active (SWS_ComM_00873).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: Dcm callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel);
/** Release the diagnostic demand (SWS_ComM_00874).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: Dcm callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel);
#endif
