/** @file Standard configured PduR upper transmission interface. */
#ifndef PDUR_DCM_H
#define PDUR_DCM_H
#include "PduR_Version.h"
#include "PduR.h"
#define PDUR_START_SEC_CODE
#include "PduR_MemMap.h"
/** Forward a configured Dcm request to its lower handle (SWS_PduR_00406).
 * @param[in] TxPduId A configured Dcm source handle; distinct from the lower namespace.
 * @param[in] PduInfoPtr Non-NULL borrowed descriptor and payload satisfying the selected
 * destination contract; no metadata.
 * @return The lower result; E_NOT_OK when uninitialized, the descriptor is NULL or no route
 * matches. Preconditions: Initialized; otherwise E_NOT_OK. Context: task or upper-layer callback;
 * not inside an exclusive area. Reentrancy: reentrant for different PDU IDs; not reentrant for the
 * same PDU ID. Ownership: This router retains neither descriptor nor payload; destination ownership
 * and completion rules apply.
 */
PDUR_CODE Std_ReturnType PduR_DcmTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#define PDUR_STOP_SEC_CODE
#include "PduR_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "PduR_Dcm.h -> PduR_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR_Dcm.h -> PduR_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
