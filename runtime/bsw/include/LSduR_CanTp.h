/** @file Standard configured LSduR upper interface. */
#ifndef LSDUR_CANTP_H
#define LSDUR_CANTP_H
#include "LSduR_Version.h"
#include "LSduR.h"
#define LSDUR_START_SEC_CODE
#include "LSduR_MemMap.h"
/** Forward a configured CanTp request to its lower handle (CP_SWS_LSduR_91009).
 * @param[in] TxPduId A configured CanTp source handle; distinct from the lower namespace.
 * @param[in] PduInfoPtr Non-NULL borrowed descriptor and payload satisfying the selected
 * destination contract; no metadata.
 * @return The lower result; E_NOT_OK when uninitialized, the descriptor is NULL or no route
 * matches. Preconditions: Initialized; otherwise E_NOT_OK. Context: task or upper-layer callback;
 * not inside an exclusive area. Reentrancy: reentrant for different PDU IDs; not reentrant for the
 * same PDU ID. Ownership: This router retains neither descriptor nor payload; destination ownership
 * and completion rules apply.
 */
LSDUR_CODE Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#define LSDUR_STOP_SEC_CODE
#include "LSduR_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "LSduR_CanTp.h -> LSduR_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "LSduR_CanTp.h -> LSduR_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
