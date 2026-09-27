/** @file
 * @brief CAN interface for configured host frames.
 */
#ifndef CANIF_H
#define CANIF_H

#include <stddef.h>
#include <stdint.h>
#include "Can_GeneralTypes.h"
#include "ComStack_Types.h"
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Install the generated host frame configuration.
 * @param[in] config Generated ECU configuration.
 */
void CanIf_Init(const EcuConfig *config);
/** @brief Transmit a configured frame through the virtual CAN controller.
 * @param[in] frame_index Index into EcuConfig::frames.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
EcuStatus CanIf_Transmit(size_t frame_index, const uint8_t data[8]);
/** @brief Route a received CAN L-PDU through the R24-11 CanIf callback.
 * @param[in] mailbox Receive hardware object and CAN identifier.
 * @param[in] pdu_info Received payload and length.
 */
void CanIf_RxIndication(const Can_HwType *mailbox, const PduInfoType *pdu_info);
/** @brief Inject one host frame through the standard CanIf receive callback.
 * @param[in] id Received standard CAN identifier.
 * @param[in] dlc Received payload length.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host routing status.
 */
EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#endif
