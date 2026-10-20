/** @file R24-11 Can Driver services for one polled Classical CAN controller on the host. */
#ifndef CAN_H
#define CAN_H
#include "Can_GeneralTypes.h"
#include "Can_CfgTypes.h"
#include "Std_Types.h"

/* Published information (SWS_BSW_00059): module ID and software version are provided below;
 * vendor ID and AUTOSAR release macros are not provided. */
#define CAN_MODULE_ID              80u   /**< SWS_BSW_00256, Appendix A.2: CAN Driver. */
#define CAN_SID_MAIN_FUNCTION_READ 0x08u /**< SWS_Can_00226: service ID. */
#define CAN_SW_MAJOR_VERSION       0u
#define CAN_SW_MINOR_VERSION       1u
#define CAN_SW_PATCH_VERSION       0u

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
/** Initialize the driver and set the controller to STOPPED (SWS_Can_00259).
 * @param[in] Config Host controller configuration; NULL or a missing synchronous sink is ignored.
 * @par Return
 * None.
 * Preconditions: driver UNINIT; a repeated call leaves the initialized state unchanged.
 * Context: task, outside any Can callback; not inside the CAN_STATE area.
 * Reentrancy: non reentrant.
 * Ownership: the configuration must stay valid until Can_DeInit.
 */
CAN_CODE void Can_Init(const Can_ConfigType *Config);
/** De-initialize the driver and return the controller to UNINIT (SWS_Can_91010).
 * @par Return
 * None.
 * Preconditions: driver initialized and controller not STARTED; otherwise the call is ignored.
 * Context: task; not inside the CAN_STATE area.
 * Reentrancy: non reentrant.
 * Ownership: pending transmissions and indications are cancelled.
 */
CAN_CODE void Can_DeInit(void);
/** Request a software transition of the controller state machine (SWS_Can_00017).
 * @param[in] Controller Driver controller identifier from CanControllerId.
 * @param[in] Transition CAN_CS_STARTED from STOPPED, CAN_CS_STOPPED from any initialized state,
 * CAN_CS_SLEEP from STOPPED or SLEEP.
 * @return E_OK when accepted; the completed state is indicated by Can_MainFunction_Mode.
 * E_NOT_OK for an invalid controller, transition or state, or while an earlier mode or
 * bus-off indication is not yet delivered.
 * Preconditions: driver initialized; otherwise E_NOT_OK.
 * Context: task, including CanIf callbacks; not inside the CAN_STATE area.
 * Reentrancy: non reentrant.
 * Ownership: none.
 */
CAN_CODE Std_ReturnType Can_SetControllerMode(uint8 Controller, Can_ControllerStateType Transition);
/** Read the controller state (SWS_Can_91015).
 * @param[in] Controller Driver controller identifier from CanControllerId.
 * @param[out] ControllerModePtr Receives STARTED, STOPPED or SLEEP.
 * @return E_OK when read; E_NOT_OK for an invalid controller or a NULL pointer.
 * Preconditions: driver initialized; otherwise E_NOT_OK and the output is unchanged.
 * Context: task or callback; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: the output is written only on E_OK.
 */
CAN_CODE Std_ReturnType Can_GetControllerMode(uint8 Controller,
                                              Can_ControllerStateType *ControllerModePtr);
/** Read the controller error state (SWS_Can_91008).
 * @param[in] ControllerId Driver controller identifier from CanControllerId.
 * @param[out] ErrorStatePtr Receives ACTIVE or BUSOFF; the virtual controller has no error-passive
 * state.
 * @return E_OK when read; E_NOT_OK for an invalid controller or a NULL pointer.
 * Preconditions: driver initialized; otherwise E_NOT_OK and the output is unchanged.
 * Context: task or callback; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: the output is written only on E_OK.
 */
CAN_CODE Std_ReturnType Can_GetControllerErrorState(uint8 ControllerId,
                                                    Can_ErrorStateType *ErrorStatePtr);
/** Query the Rx error counter, which the virtual controller does not have (SWS_Can_00511).
 * @param[in] ControllerId Driver controller identifier.
 * @param[out] RxErrorCounterPtr Unchanged.
 * @return E_NOT_OK: the counter is not available.
 * Preconditions: none.
 * Context: any; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: the output is never written.
 */
CAN_CODE Std_ReturnType Can_GetControllerRxErrorCounter(uint8 ControllerId,
                                                        uint8 *RxErrorCounterPtr);
/** Query the Tx error counter, which the virtual controller does not have (SWS_Can_00516).
 * @param[in] ControllerId Driver controller identifier.
 * @param[out] TxErrorCounterPtr Unchanged.
 * @return E_NOT_OK: the counter is not available.
 * Preconditions: none.
 * Context: any; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: the output is never written.
 */
CAN_CODE Std_ReturnType Can_GetControllerTxErrorCounter(uint8 ControllerId,
                                                        uint8 *TxErrorCounterPtr);
/** Count one nested disable of the controller interrupts; the polled controller uses none.
 * @param[in] Controller Driver controller identifier; other values are ignored.
 * @par Return
 * None.
 * Preconditions: driver initialized; otherwise the call is ignored.
 * Context: any; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: none.
 */
CAN_CODE void Can_DisableControllerInterrupts(uint8 Controller);
/** Undo one counted disable; no action when none is outstanding (SWS_Can_00208).
 * @param[in] Controller Driver controller identifier; other values are ignored.
 * @par Return
 * None.
 * Preconditions: driver initialized; otherwise the call is ignored.
 * Context: any; not inside the CAN_STATE area.
 * Reentrancy: reentrant.
 * Ownership: none.
 */
CAN_CODE void Can_EnableControllerInterrupts(uint8 Controller);
/** Check for a bus wakeup; the selected controller has no wakeup support.
 * @param[in] Controller Driver controller identifier.
 * @return E_NOT_OK: no wakeup can be detected.
 * Preconditions: none.
 * Context: any; not inside the CAN_STATE area.
 * Reentrancy: non reentrant.
 * Ownership: none.
 */
CAN_CODE Std_ReturnType Can_CheckWakeup(uint8 Controller);
/** Copy one L-PDU into the transmit object without waiting for output (SWS_Can_00212).
 * @param[in] Hth Configured hardware transmit handle.
 * @param[in] PduInfo Caller-owned L-SDU of 0 to 8 bytes; the CAN FD bit of the identifier is
 * ignored because the controller has no FD configuration (SWS_Can_00486).
 * @return E_OK when copied; CAN_BUSY while the transmit object holds an earlier request;
 * E_NOT_OK for an invalid handle, pointer, identifier, length or controller state.
 * Preconditions: driver initialized and controller STARTED; otherwise E_NOT_OK.
 * Context: task or callback; not inside the CAN_STATE area.
 * Reentrancy: reentrant; a concurrent call for the occupied handle returns CAN_BUSY.
 * Ownership: the SDU is copied before return and not kept.
 */
CAN_CODE Std_ReturnType Can_Write(Can_HwHandleType Hth, const Can_PduType *PduInfo);
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
