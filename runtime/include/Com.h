/** @file
 * @brief Host COM signal and PDU interface.
 */
#ifndef COM_H
#define COM_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"
#include "ComStack_Types.h"
#include "Std_Types.h"

/** @brief Load generated signal and frame configuration.
 * @param[in] config Generated ECU configuration.
 */
void Com_Init(const EcuConfig *config);
/** @brief Set a configured host signal value.
 * @param[in] id Generated signal identifier.
 * @param[in] value Unsigned value to store.
 * @return Host signal status.
 */
EcuStatus Com_SetSignal(uint16_t id, uint32_t value);
/** @brief Read a configured host signal and its validity flag.
 * @param[in] id Generated signal identifier.
 * @param[out] value Current unsigned value.
 * @param[out] valid Nonzero when a received signal has a valid value.
 * @return Host signal status.
 */
EcuStatus Com_GetSignal(uint16_t id, uint32_t *value, uint8_t *valid);
/** @brief Encode and send a configured Tx frame.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @return Host transmission status.
 */
EcuStatus Com_TriggerTransmit(size_t frame_index);
/** @brief Record confirmation of a configured transmitted I-PDU.
 * @param[in] tx_pdu_id Generated Tx I-PDU identifier.
 * @param[in] result Standard transmission result.
 */
void Com_TxConfirmation(PduIdType tx_pdu_id, Std_ReturnType result);
/** @brief Decode a configured Rx frame into its signals.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host receive status.
 */
EcuStatus Com_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);
/** @brief Update received-signal timeout state.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host timeout-processing status.
 */
EcuStatus Com_AdvanceTime(uint64_t now_ms);

#endif
