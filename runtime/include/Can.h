/** @file
 * @brief Host virtual CAN controller interface.
 */
#ifndef CAN_H
#define CAN_H

#include <stdint.h>
#include "Ecu_Status.h"

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

/** @brief Reset the virtual controller and install its transmit callback.
 * @param[in] sink Host frame output callback.
 */
void Can_Init(CanTxSink sink);
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
