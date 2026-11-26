/** @file Explicit CDD provider for configured controlled-host CAN channels.
 * Full completion requires actual STARTED indication and ONLINE PDU mode.
 * Bus-off cannot be recovered by an ordinary ComM request. The normal host
 * CAN mode command remains the explicit recovery boundary.
 */
#ifndef ECU_HOSTBUSSM_H
#define ECU_HOSTBUSSM_H
#include "ComM.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMM_AR_RELEASE_MAJOR_VERSION) || !defined(COMM_AR_RELEASE_MINOR_VERSION)
#error "Ecu_HostBusSM.h -> ComM.h: missing AUTOSAR release"
#elif (COMM_AR_RELEASE_MAJOR_VERSION != 4U) || (COMM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Ecu_HostBusSM.h -> ComM.h: incompatible AUTOSAR release"
#endif
#include "CanIf.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CANIF_AR_RELEASE_MAJOR_VERSION) || !defined(CANIF_AR_RELEASE_MINOR_VERSION)
#error "Ecu_HostBusSM.h -> CanIf.h: missing AUTOSAR release"
#elif (CANIF_AR_RELEASE_MAJOR_VERSION != 4U) || (CANIF_AR_RELEASE_MINOR_VERSION != 10U)
#error "Ecu_HostBusSM.h -> CanIf.h: incompatible AUTOSAR release"
#endif
#define ECU_HOSTBUSSM_START_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
/** One immutable ComM channel to CanIf controller mapping. */
typedef struct {
    NetworkHandleType channel; /**< ComM channel (ComMChannelId). */
    uint8 controller;          /**< CanIf controller (CanIfCtrlId). */
} Ecu_HostBusSM_ChannelConfigType;
/** Generated immutable BusSM channels. */
typedef struct {
    const Ecu_HostBusSM_ChannelConfigType *channels; /**< Channel-to-controller mappings. */
    uint16 channel_count;                            /**< Number of mappings. */
} Ecu_HostBusSM_ConfigType;
/** Start each configured channel in requested NO.
 * @param[in] ConfigPtr Configuration whose controllers CanIf knows; otherwise no action.
 * @par Return
 * None.
 * Preconditions: CanIf initialized.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant. A restart keeps a publication in flight.
 * Ownership: ConfigPtr is retained until the next Init or DeInit.
 */
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr);
/** Stop the provider; later requests and events have no effect.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_DeInit(void);
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
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel,
                                                               ComM_ModeType ComMode);
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
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel,
                                                                  ComM_ModeType *ComMode);
/** Apply the requested CAN directions for a BusSM operation.
 * @param[in] Channel Configured channel.
 * @param[in] ComMode NO, SILENT or FULL.
 * @return E_OK when CanIf applied the PDU mode, or for NO on a stopped controller;
 * E_NOT_OK otherwise, including SILENT/FULL on a stopped controller.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel,
                                                          ComM_ModeType ComMode);
/** Confirm the actual mode of a ComM indication without changing CAN directions.
 * @param[in] Channel Indicated channel.
 * @param[in] ComMode Indicated NO, SILENT or FULL mode.
 * @return E_OK if the actual mode can be read and matches the indication; E_NOT_OK otherwise.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: BswM user callout; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ConfirmMode(NetworkHandleType Channel,
                                                            ComM_ModeType ComMode);
/** Schedule a fresh observation after a BswM action fails.
 * @param[in] Channel Channel to observe again.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: BswM failure callout; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_QueueModePublication(NetworkHandleType Channel);
/** Publish a queued actual mode to ComM, once per observation if readable.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: ECU cycle or BusSM completion; not inside an exclusive area.
 * Reentrancy: reentrant; the active publisher handles new observations.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_PublishPendingMode(void);
/** Confirm the actual mode for an explicitly bound channel and CanIf controller.
 * @param[in] Channel Indicated channel.
 * @param[in] ControllerId Expected CanIf controller for Channel.
 * @param[in] ComMode Indicated NO, SILENT or FULL mode.
 * @return E_OK if the binding and actual mode match. E_NOT_OK otherwise.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: BswM user callout; outside exclusive areas.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
ECU_HOSTBUSSM_CODE Std_ReturnType Ecu_HostBusSM_ConfirmControllerMode(NetworkHandleType Channel,
                                                                      uint8 ControllerId,
                                                                      ComM_ModeType ComMode);
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
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerModeIndication(
    uint8 ControllerId, Can_ControllerStateType ControllerMode);
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
ECU_HOSTBUSSM_CODE void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId);
#define ECU_HOSTBUSSM_STOP_SEC_CODE
#include "Ecu_HostBusSM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Ecu_HostBusSM.h -> Ecu_HostBusSM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Ecu_HostBusSM.h -> Ecu_HostBusSM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
