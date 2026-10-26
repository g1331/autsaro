/** @file EcuM wake-up indication for the configured CDD channel. */
#ifndef COMM_ECUM_H
#define COMM_ECUM_H
#include "ComM_Version.h"
#include "ComM.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
/** Turn a wake-up in COMM_NO_COM_NO_PENDING_REQUEST into a pending request that lasts
 * until the channel enters FULL (SWS_ComM_00275, SWS_ComM_00893).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action. Other states ignore it.
 * Context: EcuM callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
COMM_CODE void ComM_EcuM_WakeUpIndication(NetworkHandleType Channel);
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM_EcuM.h -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM_EcuM.h -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
