/** @file
 * @brief Host diagnostic PDU routing bridge.
 */
#ifndef LSDUR_H
#define LSDUR_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"
#include "ComStack_Types.h"
#include "Std_Types.h"

/** @brief Load the generated host diagnostic routing connection.
 * @param[in] config Generated diagnostic configuration.
 */
void LSduR_Init(const EcuDiagnosticConfig *config);
/** @brief Route a transport segment toward the virtual CAN controller.
 * @param[in] dlc Number of valid frame bytes.
 * @param[in] data Eight-byte frame buffer.
 * @return Host routing status.
 */
EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]);
/** @brief Forward a received diagnostic CAN frame to CanTp.
 * @param[in] dlc Number of valid frame bytes.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing status.
 */
EcuStatus LSduR_CanTpRxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms);
/** @brief Route a configured PDU frame toward CanIf.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @return Host routing status.
 */
EcuStatus LSduR_PduRTransmit(size_t frame_index, const uint8_t data[8]);
/** @brief Route one CanIf transmit confirmation to the owning upper layer.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] result Standard transmission result.
 */
void LSduR_CanIfTxConfirmation(PduIdType pdu_id, Std_ReturnType result);
/** @brief Forward a received configured frame from CanIf.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing status.
 */
EcuStatus LSduR_CanIfRxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);

#endif
