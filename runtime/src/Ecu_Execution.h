#ifndef ECU_EXECUTION_H
#define ECU_EXECUTION_H

#include "Can.h"
#include "Ecu_Config.h"

/* Each validated profile links exactly one adapter. Policy remains in Ecu_Policy. */
EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc,
                                const uint8_t data[8]);
uint64_t Ecu_ExecutionNow(void);
EcuStatus Ecu_DiagnosticAdmit(const EcuDiagnosticConfig *config, const uint8_t *request,
                              size_t length, uint64_t now_ms);
unsigned Ecu_DiagnosticPending(void);

#endif
