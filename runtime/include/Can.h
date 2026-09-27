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
    CAN_BUS_OFF = 2, /**< Controller rejects frame traffic after a bus-off event. */
    CAN_SLEEP = 3    /**< Logical sleep state for the virtual controller. */
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
/** @brief De-initialize a stopped or sleeping host CAN controller. */
void Can_DeInit(void);
/** @brief Select the generated host baud-rate configuration, ID zero.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[in] baud_rate_config_id Generated baud-rate configuration identifier, currently zero.
 * @return E_OK if the configuration was selected, otherwise E_NOT_OK.
 */
Std_ReturnType Can_SetBaudrate(uint8_t controller, uint16_t baud_rate_config_id);
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
/** @brief Read the host controller's active or bus-off error state.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[out] error_state Current CAN error state.
 * @return E_OK when the state was read, otherwise E_NOT_OK.
 */
Std_ReturnType Can_GetControllerErrorState(uint8_t controller, Can_ErrorStateType *error_state);
/** @brief Query the unavailable host Rx hardware error counter.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[out] error_counter Unchanged because the virtual target has no hardware counter.
 * @return E_NOT_OK; the host target has no Rx error counter.
 */
Std_ReturnType Can_GetControllerRxErrorCounter(uint8_t controller, uint8_t *error_counter);
/** @brief Query the unavailable host Tx hardware error counter.
 * @param[in] controller Generated controller identifier, currently zero.
 * @param[out] error_counter Unchanged because the virtual target has no hardware counter.
 * @return E_NOT_OK; the host target has no Tx error counter.
 */
Std_ReturnType Can_GetControllerTxErrorCounter(uint8_t controller, uint8_t *error_counter);
/** @brief Increment the nested host interrupt-disable count.
 * @param[in] controller Generated controller identifier, currently zero.
 */
void Can_DisableControllerInterrupts(uint8_t controller);
/** @brief Decrement the nested host interrupt-disable count if nonzero.
 * @param[in] controller Generated controller identifier, currently zero.
 */
void Can_EnableControllerInterrupts(uint8_t controller);
/** @brief Report that the host target has no hardware wakeup source.
 * @param[in] controller Generated controller identifier, currently zero.
 * @return E_NOT_OK because no hardware wakeup event can be detected.
 */
Std_ReturnType Can_CheckWakeup(uint8_t controller);
/** @brief Queue one CAN L-SDU through hardware transmit handle zero without waiting for output.
 * @param[in] hth Generated hardware transmit handle, currently zero.
 * @param[in] pdu Caller-owned CAN L-SDU.
 * @return E_OK when accepted, CAN_BUSY if the handle is occupied, otherwise E_NOT_OK.
 */
Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu);
/** @brief Complete the pending host transmission through the configured output callback.
 * @return Host output status, or ECU_OK if no frame is pending.
 */
EcuStatus Can_HostFlush(void);

/** @brief Change the virtual controller state.
 * @param[in] mode New controller state.
 */
void Can_SetMode(CanMode mode);
/** @brief Read the virtual controller state.
 * @return Current controller state.
 */
CanMode Can_GetMode(void);
/** @brief Send a host frame while retaining its generated CanIf PDU handle.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
EcuStatus Can_TransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc, const uint8_t data[8]);
/** @brief Inject one received frame into the host CAN stack.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host receive status.
 */
EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#endif
