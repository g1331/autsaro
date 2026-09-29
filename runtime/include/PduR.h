/** @file
 * @brief Host PDU routing and transport buffer interface.
 */
#ifndef PDUR_H
#define PDUR_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"
#include "ComStack_Types.h"
#include "Std_Types.h"

/** @brief Route a configured Tx frame to LSduR.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @return Host routing status.
 */
EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8]);
/** @brief Dispatch a CanIf transmit confirmation to Com or CanTp.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] result Standard transmission result.
 */
void PduR_CanIfTxConfirmation(PduIdType pdu_id, Std_ReturnType result);
/** @brief Route a configured Rx frame to Com.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing status.
 */
EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);
/** @brief Load generated frame routing and reset transport buffers.
 * @param[in] config Generated ECU configuration.
 */
void PduR_Init(const EcuConfig *config);

/** @brief Reserve the PduR-owned buffer for an incoming N-SDU.
 * @param[in] length Expected complete N-SDU length in bytes.
 * @return Host buffer status.
 */
EcuStatus PduR_CanTpStartOfReception(size_t length);
/** @brief Append received transport data to the PduR-owned N-SDU.
 * @param[in] data Received segment bytes.
 * @param[in] length Number of segment bytes.
 * @return Host buffer status.
 */
EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length);
/** @brief Deliver the completed N-SDU to Dcm.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing status.
 */
EcuStatus PduR_CanTpRxIndication(uint64_t now_ms);
/** @brief Discard an incomplete received N-SDU. */
void PduR_CanTpRxAbort(void);
/** @brief Copy a segment from the PduR-owned response N-SDU.
 * @param[in] offset Byte offset into the complete response.
 * @param[out] destination Caller-provided segment buffer.
 * @param[in] length Number of bytes to copy.
 * @return Host buffer status.
 */
EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length);
/** @brief Forward transport completion to Dcm.
 * @param[in] status Transport completion status.
 * @param[in] now_ms Current host clock in milliseconds.
 */
void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms);
/** @brief Store and send one complete diagnostic response N-SDU.
 * @param[in] data Response payload bytes.
 * @param[in] length Response payload length in bytes.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing and transport status.
 */
EcuStatus PduR_DcmTransmit(const uint8_t *data, size_t length, uint64_t now_ms);

#ifdef ECU_TARGET_EPIC4
/** @brief Query the owner's physical diagnostic connection availability.
 * @return Nonzero when neither reception nor response owns its buffers.
 */
int PduR_TargetDiagnosticReady(void);
#endif

#endif
