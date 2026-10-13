/** @file Standard configured CanIf callbacks to LSduR. */
#ifndef LSDUR_CANIF_H
#define LSDUR_CANIF_H
#include "LSduR.h"
void LSduR_CanIfRxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
void LSduR_CanIfTxConfirmation(PduIdType TxPduId, Std_ReturnType result);
Std_ReturnType LSduR_CanIfTriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
#endif
