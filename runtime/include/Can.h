/** @file
 * @brief Host virtual CAN controller interface.
 */
#ifndef CAN_H
#define CAN_H

#include <stdint.h>
#include "Can_GeneralTypes.h"
#include "Ecu_Status.h"
#include "Std_Types.h"

/** @brief State of the host virtual CAN controller. */
typedef enum {
    CAN_STOPPED = 0, /**< Controller is stopped. */
    CAN_STARTED = 1, /**< Controller accepts frame traffic. */
    CAN_BUS_OFF = 2  /**< Controller rejects frame traffic after a bus-off event. */
} CanMode;

/** @brief Host callback that emits one CAN frame.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid bytes in data.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
typedef EcuStatus (*CanTxSink)(uint32_t id, uint8_t dlc, const uint8_t data[8]);

/** @brief Hardware-specific Can Driver initialization data for the host target. */
typedef struct {
    CanTxSink sink; /**< Host frame output callback. */
} Can_ConfigType;

/** @brief Initialize the host Can Driver in the stopped state.
 * @param[in] config Host controller configuration.
 */
void Can_Init(const Can_ConfigType *config);
/** @brief Request a controller state transition on controller zero.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[in] transition Requested standard controller state.
 * @return E_OK if the transition was accepted, otherwise E_NOT_OK.
 */
Std_ReturnType Can_SetControllerMode(uint8_t controller, Can_ControllerStateType transition);
/** @brief Read the standard state of controller zero.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[out] mode Current standard controller state.
 * @return E_OK when the state was read, otherwise E_NOT_OK.
 */
Std_ReturnType Can_GetControllerMode(uint8_t controller, Can_ControllerStateType *mode);
/** @brief Submit one CAN L-SDU through hardware transmit handle zero.
 * @param[in] hth Generated hardware transmit handle, currently zero.
 * @param[in] pdu Caller-owned CAN L-SDU.
 * @return E_OK when the host sink accepts the frame, otherwise E_NOT_OK.
 */
Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu);

/** @brief Change the virtual controller state.
 * @param[in] mode New controller state.
 */
void Can_SetMode(CanMode mode);
/** @brief Read the virtual controller state.
 * @return Current controller state.
 */
CanMode Can_GetMode(void);
/** @brief Send one configured Classical CAN frame through the host sink.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
EcuStatus Can_Transmit(uint32_t id, uint8_t dlc, const uint8_t data[8]);
/** @brief Inject one received frame into the host CAN stack.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host receive status.
 */
EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#endif
