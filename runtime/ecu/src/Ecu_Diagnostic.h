#ifndef ECU_DIAGNOSTIC_H
#define ECU_DIAGNOSTIC_H

#include "Ecu_Status.h"
#include <stdint.h>

/* Reset only during successful target preparation, before the owner starts. */
void Ecu_DiagnosticReset(void);
/* Dispatch only from the owner's diagnostic or output-confirmation phase. */
EcuStatus Ecu_DiagnosticProcess(uint64_t now_ms);

#endif
