/** @file Explicit CDD provider for one controlled-host CAN channel.
 * Full completion requires actual STARTED indication and ONLINE PDU mode.
 * Bus-off cannot be recovered by an ordinary ComM request. The normal host
 * CAN mode command remains the explicit recovery boundary.
 */
#ifndef ECU_HOSTBUSSM_H
#define ECU_HOSTBUSSM_H
#include "ComM.h"
#include "CanIf.h"
/** Generated immutable bus_config: the ComM channel and its CanIf controller. */
typedef struct {
    NetworkHandleType channel; /**< ComM channel (ComMChannelId). */
    uint8 controller;          /**< CanIf controller (CanIfCtrlId). */
} Ecu_HostBusSM_ConfigType;
/** Start the provider in requested NO.
 * @param[in] ConfigPtr Configuration whose controller CanIf knows; otherwise no action.
 * @par Return
 * None.
 * Preconditions: CanIf initialized.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant. A restart keeps a publication in flight.
 * Ownership: ConfigPtr is retained until the next Init or DeInit.
 */
void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr);
/** Stop the provider; later requests and events have no effect.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: None.
 */
void Ecu_HostBusSM_DeInit(void);
/** <Bus>SM_RequestComMode for ComM (SWS_ComM_00828). NO stops the controller, SILENT and
 * FULL start it; the mode is indicated to ComM when the controller completes.
 * @param[in] Channel Configured channel.
 * @param[in] ComMode NO, SILENT or FULL.
 * @return E_OK when applied or a transition is under way; E_NOT_OK before Init, for
 * another channel or mode, for SILENT/FULL during bus-off, or when CanIf refuses.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: ComM; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel, ComM_ModeType ComMode);
/** <Bus>SM_GetCurrentComMode for ComM (SWS_ComM_00828): NO unless STARTED, settled and
 * not OFFLINE; SILENT for TX_OFFLINE; otherwise FULL.
 * @param[in] Channel Configured channel.
 * @param[out] ComMode Written on E_OK.
 * @return E_OK, or E_NOT_OK before Init, for another channel, a NULL pointer, or when
 * CanIf cannot report the controller.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel, ComM_ModeType *ComMode);
/** Actual BswM user-callout action; controls directions, never COM groups.
 * @param[in] Channel Configured channel.
 * @param[in] ComMode NO, SILENT or FULL.
 * @return E_OK when CanIf applied the PDU mode, or for NO on a stopped controller;
 * E_NOT_OK otherwise, including SILENT/FULL on a stopped controller.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: BswM action; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel, ComM_ModeType ComMode);
/** CanIf controller mode indication (CanIfDispatchUserCtrlModeIndicationName): applies
 * the requested directions on STARTED and indicates the completed mode to ComM.
 * @param[in] ControllerId Configured controller; others are ignored.
 * @param[in] ControllerMode Indicated controller state.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: CanIf callback; not inside an exclusive area.
 * Reentrancy: reentrant; completed modes reach ComM in order, the latest of several.
 * Ownership: None.
 */
void Ecu_HostBusSM_ControllerModeIndication(uint8 ControllerId,
                                            Can_ControllerStateType ControllerMode);
/** CanIf bus-off indication (CanIfDispatchUserCtrlBusOffName): indicates NO for a stopped
 * controller; a late one after a restart restores the requested directions.
 * @param[in] ControllerId Configured controller; others are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: CanIf callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId);
#endif
