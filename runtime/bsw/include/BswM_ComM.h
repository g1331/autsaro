/** @file ComM mode indication entry of BswM for the single configured channel. */
#ifndef BSWM_COMM_H
#define BSWM_COMM_H
#include "BswM_Version.h"
#include "BswM.h"
#define BSWM_START_SEC_CODE
#include "BswM_MemMap.h"
/** Arbitrate the indicated ComM mode and run the matching rule's action (SWS_BswM_00047).
 * @param[in] Network Configured channel; other channels are ignored.
 * @param[in] RequestedMode NO, SILENT or FULL; other values are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback; not inside an exclusive area.
 * Reentrancy: reentrant. An indication received while an action runs is postponed and
 * arbitrated after it in the same caller's context (SWS_BswM_00069); each postponed
 * indication arbitrates the latest indicated mode (SWS_BswM_00281).
 * Ownership: None.
 */
BSWM_CODE void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType RequestedMode);
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "BswM_ComM.h -> BswM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "BswM_ComM.h -> BswM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
