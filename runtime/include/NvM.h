#ifndef NVM_H
#define NVM_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/* Host-virtual, single-byte Dem block; not a generic AUTOSAR NvM implementation. */
EcuStatus NvM_Init(const EcuConfig *config, const char *path, uint8_t *status);
EcuStatus NvM_Write(uint8_t status);

#endif
