/** @file BswM-to-ComM communication-allowed callback for the configured channel. */
#ifndef COMM_BSWM_H
#define COMM_BSWM_H
#include "ComM_Version.h"
#include "ComM.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
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
COMM_CODE void ComM_CommunicationAllowed(NetworkHandleType Channel, boolean Allowed);
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM_BswM.h -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM_BswM.h -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
