/** @file R24-11 ComM notifications for the one configured Dcm channel. */
#ifndef DCM_COMM_H
#define DCM_COMM_H
#include "Std_Types.h"
/** Disable reception and new response transmissions (SWS_Dcm_00356).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void Dcm_ComM_NoComModeEntered(uint8 NetworkId);
/** Allow reception and disable new response transmissions (SWS_Dcm_00358).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void Dcm_ComM_SilentComModeEntered(uint8 NetworkId);
/** Allow reception and new response transmissions (SWS_Dcm_00360).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void Dcm_ComM_FullComModeEntered(uint8 NetworkId);
#endif
