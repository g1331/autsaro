/** @file
 * @brief Bounded host DoCAN transport interface.
 */
#ifndef CANTP_H
#define CANTP_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"
#include "ComStack_Types.h"
#include "Std_Types.h"

/** @brief Reset one physical, normal-addressed 11-bit DoCAN connection.
 * @param[in] config Generated diagnostic connection.
 */
void CanTp_Init(const EcuDiagnosticConfig *config);
/** @brief Accept a CAN transport segment from LSduR.
 * @param[in] dlc Received Classical CAN payload length.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host transport status.
 */
EcuStatus CanTp_RxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms);
/** @brief Begin sending the PduR-owned diagnostic response payload.
 * @param[in] length Total response payload length in bytes.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host transport status.
 */
EcuStatus CanTp_Transmit(size_t length, uint64_t now_ms);
/** @brief Record confirmation of one transmitted diagnostic CAN frame.
 * @param[in] tx_pdu_id Generated diagnostic transmit PDU identifier.
 * @param[in] result Standard CAN frame transmission result.
 */
void CanTp_TxConfirmation(PduIdType tx_pdu_id, Std_ReturnType result);
/** @brief Advance transport timers and pending segment transmission.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host transport status.
 */
EcuStatus CanTp_AdvanceTime(uint64_t now_ms);

#endif
