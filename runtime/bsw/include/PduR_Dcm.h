/** @file Standard configured PduR upper transmission interface. */
#ifndef PDUR_DCM_H
#define PDUR_DCM_H
#include "PduR.h"
Std_ReturnType PduR_DcmTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
#endif
