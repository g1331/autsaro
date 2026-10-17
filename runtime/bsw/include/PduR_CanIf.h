/** @file Standard PduR interface callbacks, invoked through configured LSduR. */
#ifndef PDUR_CANIF_H
#define PDUR_CANIF_H
#include "PduR.h"
/** Synchronously route a matching CanIf reception (SWS_PduR_00362).
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
void PduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
/** Forward a lower result using the configured upper handle (SWS_PduR_00365).
 * @param[in] TxPduId A configured lower Tx handle.
 * @param[in] result E_OK for a successful frame, E_NOT_OK for failure.
 * @par Return
 * None.
 * Preconditions: Initialized and a matching route; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: None.
 */
void PduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result);
/** Ask the upper callback to fill the lower buffer (SWS_PduR_00369).
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
Std_ReturnType PduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
#endif
