/** @file R24-11 CAN service view for the selected host configuration. */
#ifndef CAN_H
#define CAN_H
#include "Can_GeneralTypes.h"
#include "Can_CfgTypes.h"
#include "Std_Types.h"

/* Software identity follows core/Cargo.toml; assigned vendor identity is absent. */
#define CAN_MODULE_ID 80u
#define CAN_SW_MAJOR_VERSION 0u
#define CAN_SW_MINOR_VERSION 1u
#define CAN_SW_PATCH_VERSION 0u

#ifndef CAN_VERSION_INFO_API
#define CAN_VERSION_INFO_API 0
#endif
#if CAN_VERSION_INFO_API != 0
#error "Can_GetVersionInfo is unsupported: no assigned vendor identity"
#endif

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
/** @brief Initialize the host Can Driver in the stopped state; ignore invalid or repeated calls.
 * @param[in] config Host controller configuration.
 */
CAN_CODE void Can_Init(const Can_ConfigType *config);
/** @brief De-initialize a stopped or sleeping host CAN controller. */
CAN_CODE void Can_DeInit(void);
/** @brief Select the generated host baud-rate configuration, ID zero.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[in] baud_rate_config_id Generated baud-rate configuration identifier, currently zero.
 * @return E_OK if the configuration was selected, otherwise E_NOT_OK.
 */
CAN_CODE Std_ReturnType Can_SetBaudrate(uint8_t controller, uint16_t baud_rate_config_id);
/** @brief Request a transition of the configured controller.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[in] transition Requested standard controller state.
 * @return E_OK if the transition was accepted, otherwise E_NOT_OK.
 */
CAN_CODE Std_ReturnType Can_SetControllerMode(uint8_t controller,
                                              Can_ControllerStateType transition);
/** @brief Read the standard state of the configured controller.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[out] mode Current standard controller state.
 * @return E_OK when the state was read, otherwise E_NOT_OK.
 */
CAN_CODE Std_ReturnType Can_GetControllerMode(uint8_t controller, Can_ControllerStateType *mode);
/** @brief Read the host controller's active or bus-off error state.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[out] error_state Current CAN error state.
 * @return E_OK when the state was read, otherwise E_NOT_OK.
 */
CAN_CODE Std_ReturnType Can_GetControllerErrorState(uint8_t controller,
                                                    Can_ErrorStateType *error_state);
/** @brief Query the unavailable host Rx hardware error counter.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[out] error_counter Unchanged because the virtual target has no hardware counter.
 * @return E_NOT_OK; the host target has no Rx error counter.
 */
CAN_CODE Std_ReturnType Can_GetControllerRxErrorCounter(uint8_t controller, uint8_t *error_counter);
/** @brief Query the unavailable host Tx hardware error counter.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @param[out] error_counter Unchanged because the virtual target has no hardware counter.
 * @return E_NOT_OK; the host target has no Tx error counter.
 */
CAN_CODE Std_ReturnType Can_GetControllerTxErrorCounter(uint8_t controller, uint8_t *error_counter);
/** @brief Increment the nested host interrupt-disable count.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 */
CAN_CODE void Can_DisableControllerInterrupts(uint8_t controller);
/** @brief Decrement the nested host interrupt-disable count if nonzero.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 */
CAN_CODE void Can_EnableControllerInterrupts(uint8_t controller);
/** @brief Report that the host target has no hardware wakeup source.
 * @param[in] controller Configured driver controller identifier; legacy profile uses zero.
 * @return E_NOT_OK because no hardware wakeup event can be detected.
 */
CAN_CODE Std_ReturnType Can_CheckWakeup(uint8_t controller);
/** @brief Queue one CAN L-SDU through the configured transmit handle without waiting for output.
 * @param[in] hth Configured hardware transmit handle; legacy profile uses zero.
 * @param[in] pdu Caller-owned CAN L-SDU.
 * @return E_OK when accepted, CAN_BUSY if the handle is occupied, otherwise E_NOT_OK.
 */
CAN_CODE Std_ReturnType Can_Write(Can_HwHandleType hth, const Can_PduType *pdu);
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
