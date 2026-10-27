/** @file Standard configured CanIf callbacks to LSduR. */
#ifndef LSDUR_CANIF_H
#define LSDUR_CANIF_H
#include "LSduR_Version.h"
#include "LSduR.h"
#define LSDUR_START_SEC_CODE
#include "LSduR_MemMap.h"
/** Synchronously route a matching CanIf reception (CP_SWS_LSduR_91011).
 * @param[in] RxPduId A configured lower Rx handle.
 * @param[in] PduInfoPtr Non-NULL borrowed receive descriptor and payload; the upper callback
 * validates its length.
 * @par Return
 * None.
 * Preconditions: Initialized and a matching route; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: The descriptor and payload are forwarded before return; this router retains neither.
 */
LSDUR_CODE void LSduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
/** Forward a lower result using the configured upper handle (CP_SWS_LSduR_91013).
 * @param[in] TxPduId A configured lower Tx handle.
 * @param[in] result E_OK for a successful frame, E_NOT_OK for failure.
 * @par Return
 * None.
 * Preconditions: Initialized and a matching route; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: None.
 */
LSDUR_CODE void LSduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result);
/** Ask the upper callback to fill the lower buffer (CP_SWS_LSduR_91014).
 * @param[in] TxPduId A configured lower Tx handle.
 * @param[in,out] PduInfoPtr Non-NULL borrowed descriptor; input SduLength is capacity, output
 * length and bytes are supplied by the upper callback.
 * @return The upper result; E_NOT_OK for NULL info, no route, no trigger callback or an
 * uninitialized router. Preconditions: Initialized and a route with a trigger callback; otherwise
 * E_NOT_OK with the descriptor unchanged. Context: lower-layer callback, including interrupt
 * context; not inside an exclusive area. Reentrancy: reentrant for different PDU IDs; not reentrant
 * for the same PDU ID. Ownership: No pointer is retained; the destination callback defines writes
 * on success or refusal.
 */
LSDUR_CODE Std_ReturnType LSduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
#define LSDUR_STOP_SEC_CODE
#include "LSduR_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "LSduR_CanIf.h -> LSduR_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "LSduR_CanIf.h -> LSduR_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
