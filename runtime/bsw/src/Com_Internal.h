/** @file Private scheduler implementation shared by generated SchM instances. */
#ifndef COM_INTERNAL_H
#define COM_INTERNAL_H
#include "Com.h"
/** Owner supplies ordering only; this adapter has no clock or timer. */
void Ecu_ComReceptionBeforeMain(boolean pending);
void Ecu_ComMainFunctionRx(void);
void Ecu_ComMainFunctionTx(void);
#endif
