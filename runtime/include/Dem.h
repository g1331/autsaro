/** @file
 * @brief Host diagnostic event and DTC interface.
 */
#ifndef DEM_H
#define DEM_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

EcuStatus Dem_Init(const EcuConfig *config, const char *nvm_path);
EcuStatus Dem_DisableDTCSetting(void);
EcuStatus Dem_EnableDTCSetting(void);
EcuStatus Dem_ReportPassed(uint16_t frame_index);
EcuStatus Dem_ReportFailed(uint16_t frame_index);
uint8_t Dem_FilterDtc(uint8_t mask, uint32_t *code, uint8_t *status);
uint8_t Dem_GetSupportedDtc(uint32_t *code, uint8_t *status);
EcuStatus Dem_ClearAll(void);

#endif
