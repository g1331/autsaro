/** @file Generated from the selected source contract. */
#ifndef ECU_EXECUTION_H
#define ECU_EXECUTION_H
#include "Ecu_Config.h"
#include "Can_Host.h"
EcuStatus Ecu_ExecutionTransmit(
    Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);
uint64_t Ecu_ExecutionNow(void);
#endif
