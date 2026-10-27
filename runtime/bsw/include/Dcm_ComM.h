/** @file R24-11 ComM notifications for the one configured Dcm channel. */
#ifndef DCM_COMM_H
#define DCM_COMM_H
#include "Dcm_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Dcm_ComM.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm_ComM.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
/** Disable reception and new response transmissions (SWS_Dcm_00356).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
DCM_CODE void Dcm_ComM_NoComModeEntered(uint8 NetworkId);
/** Allow reception and disable new response transmissions (SWS_Dcm_00358).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
DCM_CODE void Dcm_ComM_SilentComModeEntered(uint8 NetworkId);
/** Allow reception and new response transmissions (SWS_Dcm_00360).
 * @param[in] NetworkId Configured ComM channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
DCM_CODE void Dcm_ComM_FullComModeEntered(uint8 NetworkId);
#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Dcm_ComM.h -> Dcm_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Dcm_ComM.h -> Dcm_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
