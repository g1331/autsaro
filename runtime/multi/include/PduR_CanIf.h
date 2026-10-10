/** @file Standard PduR interface callbacks, invoked through configured LSduR. */
#ifndef PDUR_CANIF_H
#define PDUR_CANIF_H
#include "PduR.h"
void PduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
void PduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result);
Std_ReturnType PduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
#endif
