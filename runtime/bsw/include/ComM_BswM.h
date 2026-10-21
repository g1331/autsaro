/** @file BswM-to-ComM communication-allowed callback for the configured channel. */
#ifndef COMM_BSWM_H
#define COMM_BSWM_H
#include "ComM_Version.h"
#include "ComM.h"
/** Store CommunicationAllowed; only COMM_NO_COM_REQUEST_PENDING evaluates it
 * (SWS_ComM_00871, SWS_ComM_00896).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @param[in] Allowed TRUE permits a pending request to enter FULL.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action. Init resets the flag to FALSE.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
void ComM_CommunicationAllowed(NetworkHandleType Channel, boolean Allowed);
#endif
