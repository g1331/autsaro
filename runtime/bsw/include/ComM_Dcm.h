/** @file R24-11 Dcm active and inactive diagnostic notifications to ComM. */
#ifndef COMM_DCM_H
#define COMM_DCM_H
#include "ComM_Version.h"
#include "ComM.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
/** Keep the channel requested while diagnosis is active (SWS_ComM_00873).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: Dcm callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
COMM_CODE void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel);
/** Release the diagnostic demand (SWS_ComM_00874).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: Dcm callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
COMM_CODE void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel);
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM_Dcm.h -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM_Dcm.h -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
