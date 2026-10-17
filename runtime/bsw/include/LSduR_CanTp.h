/** @file Standard configured LSduR upper interface. */
#ifndef LSDUR_CANTP_H
#define LSDUR_CANTP_H
#include "LSduR.h"
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
Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#endif
