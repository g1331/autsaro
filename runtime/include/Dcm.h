#ifndef DCM_H
#define DCM_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

void Dcm_Init(const EcuDiagnosticConfig *config);
EcuStatus Dcm_RxIndication(const uint8_t *request, size_t length, uint64_t now_ms);
void Dcm_TpTxConfirmation(EcuStatus status, uint64_t now_ms);
void Dcm_AdvanceTime(uint64_t now_ms);

#endif
