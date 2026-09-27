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
/** @brief Transmit one configured diagnostic response segment via CanIf.
 * @param[in] dlc Number of valid segment bytes.
 * @param[in] data Eight-byte segment buffer.
 * @return Host transmission status.
 */
EcuStatus CanIf_TransmitDiagnostic(uint8_t dlc, const uint8_t data[8]);
/** @brief Confirm a successful Can Driver transmission to the configured upper layer.
 * @param[in] can_tx_pdu_id Software PDU handle retained by Can_Write.
 */
void CanIf_TxConfirmation(PduIdType can_tx_pdu_id);
/** @brief Record the controller mode reported by the Can Driver.
 * @param[in] controller_id Generated CanIf controller identifier, currently zero.
 * @param[in] controller_mode Mode reached by the virtual controller.
 */
void CanIf_ControllerModeIndication(uint8_t controller_id, Can_ControllerStateType controller_mode);
/** @brief Record a bus-off notification from the Can Driver.
 * @param[in] controller_id Generated CanIf controller identifier, currently zero.
 */
void CanIf_ControllerBusOff(uint8_t controller_id);
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
