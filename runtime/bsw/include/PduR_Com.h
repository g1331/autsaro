/** @file Standard configured PduR upper transmission interface. */
#ifndef PDUR_COM_H
#define PDUR_COM_H
#include "PduR_Version.h"
#include "PduR.h"
/** Forward a configured Com request to its lower handle (SWS_PduR_00406).
 * @param[in] TxPduId A configured Com source handle; distinct from the lower namespace.
 * @param[in] PduInfoPtr Non-NULL borrowed descriptor and payload satisfying the selected
 * destination contract; no metadata.
 * @return The lower result; E_NOT_OK when uninitialized, the descriptor is NULL or no route
 * matches. Preconditions: Initialized; otherwise E_NOT_OK. Context: task or upper-layer callback;
 * not inside an exclusive area. Reentrancy: reentrant for different PDU IDs; not reentrant for the
 * same PDU ID. Ownership: This router retains neither descriptor nor payload; destination ownership
 * and completion rules apply.
 */
Std_ReturnType PduR_ComTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#endif
