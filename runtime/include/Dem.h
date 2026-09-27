/** @file
 * @brief Host diagnostic event and DTC interface.
 */
#ifndef DEM_H
#define DEM_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Initialize the configured host DTC and its persistent status.
 * @param[in] config Generated ECU configuration.
 * @param[in] nvm_path Host DTC storage path.
 * @return Host initialization status.
 */
EcuStatus Dem_Init(const EcuConfig *config, const char *nvm_path);
/** @brief Suppress status changes from subsequent monitor reports.
 * @return Host DTC-setting status.
 */
EcuStatus Dem_DisableDTCSetting(void);
/** @brief Resume status changes from monitor reports.
 * @return Host DTC-setting status.
 */
EcuStatus Dem_EnableDTCSetting(void);
/** @brief Report that the configured Rx-frame monitor passed.
 * @param[in] frame_index Index of the monitored frame.
 * @return Host DTC and persistence status.
 */
EcuStatus Dem_ReportPassed(uint16_t frame_index);
/** @brief Report that the configured Rx-frame monitor failed.
 * @param[in] frame_index Index of the monitored frame.
 * @return Host DTC and persistence status.
 */
EcuStatus Dem_ReportFailed(uint16_t frame_index);
/** @brief Read the configured DTC when its status matches a mask.
 * @param[in] mask Requested DTC status mask.
 * @param[out] code Matching 24-bit DTC number.
 * @param[out] status Current DTC status byte.
 * @return 1 when a DTC matches, otherwise 0.
 */
uint8_t Dem_FilterDtc(uint8_t mask, uint32_t *code, uint8_t *status);
/** @brief Read the configured DTC regardless of its current status.
 * @param[out] code Configured 24-bit DTC number.
 * @param[out] status Current DTC status byte.
 * @return 1 when a DTC is configured, otherwise 0.
 */
uint8_t Dem_GetSupportedDtc(uint32_t *code, uint8_t *status);
/** @brief Clear the configured DTC and persist its reset status.
 * @return Host DTC and persistence status.
 */
EcuStatus Dem_ClearAll(void);

#endif
