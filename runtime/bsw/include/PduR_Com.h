/** @file Standard configured PduR upper transmission interface. */
#ifndef PDUR_COM_H
#define PDUR_COM_H
#include "PduR.h"
Std_ReturnType PduR_ComTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#endif
