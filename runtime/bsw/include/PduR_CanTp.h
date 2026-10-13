/** @file Selected unbuffered CanTp-to-Dcm transport routes. */
#ifndef PDUR_CANTP_H
#define PDUR_CANTP_H
#include "ComStack_Types.h"
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id, const PduInfoType *info,
                                             PduLengthType length, PduLengthType *available);
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id, const PduInfoType *info,
                                       PduLengthType *available);
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result);
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id, const PduInfoType *info,
                                       const RetryInfoType *retry, PduLengthType *available);
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result);
#endif
