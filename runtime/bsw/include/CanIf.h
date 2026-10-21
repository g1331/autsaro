/** @file Standard CanIf for one Classical CAN controller and immutable routes. */
#ifndef CANIF_H
#define CANIF_H
#include "CanIf_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "CanIf.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "CanIf.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
#include "Can_GeneralTypes.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CAN_AR_RELEASE_MAJOR_VERSION) || !defined(CAN_AR_RELEASE_MINOR_VERSION)
#error "CanIf.h -> Can_GeneralTypes.h: missing AUTOSAR release"
#elif (CAN_AR_RELEASE_MAJOR_VERSION != 4U) || (CAN_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.h -> Can_GeneralTypes.h: incompatible AUTOSAR release"
#endif
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
#define CANIF_MODULE_ID         60u
#define CANIF_SID_TRANSMIT      0x49u /**< SWS_CANIF_00005: service ID. */
#define CANIF_SID_RX_INDICATION 0x14u /**< SWS_CANIF_00006: service ID. */
/* Runtime errors reported to Det (SWS_CANIF_91007). */
#define CANIF_E_INVALID_DATA_LENGTH   61u /**< Received DLC fails the data length check. */
#define CANIF_E_DATA_LENGTH_MISMATCH  62u /**< Transmit length exceeds the CAN frame. */
#define CANIF_E_STOPPED               70u /**< Transmit requested on an offline PDU channel. */
#define CANIF_E_TXPDU_LENGTH_EXCEEDED 90u /**< Transmit length exceeds the configured PDU. */
/** Channel activity (SWS_CANIF_00137); TX_OFFLINE_ACTIVE is not selected. */
typedef enum {
    CANIF_OFFLINE = 0,    /**< Neither transmit nor receive is forwarded. */
    CANIF_TX_OFFLINE = 1, /**< Receive is enabled; transmit and confirmations are suppressed. */
    CANIF_TX_OFFLINE_ACTIVE =
        2,           /**< Reserved standard value; SetPduMode rejects it in this profile. */
    CANIF_ONLINE = 3 /**< Transmit and receive are enabled. */
} CanIf_PduModeType;
/** One software-filter route, generated from CanIfRxPduCfg and its referenced PDU. */
typedef struct {
    Can_IdType can_id;    /**< 11-bit acceptance ID (CanIfRxPduCanId). */
    PduIdType pdu;        /**< CanIf Rx handle (CanIfRxPduId), passed to LSduR. */
    uint8 minimum_length; /**< Minimum accepted DLC, bytes 0..8 (CanIfRxPduDataLength). */
} CanIf_RxPduConfigType;
/** One fixed-ID transmit route, generated from CanIfTxPduCfg and its referenced PDU. */
typedef struct {
    Can_IdType can_id;    /**< 11-bit transmitted ID (CanIfTxPduCanId). */
    PduIdType pdu;        /**< CanIf Tx handle (CanIfTxPduId), used for confirmations. */
    uint8 maximum_length; /**< Maximum payload bytes, 1..8, from the referenced PDU length. */
} CanIf_TxPduConfigType;
/** Generated dispatch callback; called after releasing CANIF_STATE with the abstract controller ID.
 */
typedef void (*CanIf_ModeNotificationType)(uint8 controller, Can_ControllerStateType mode);
/** Generated bus-off dispatch callback; called outside CANIF_STATE with the abstract controller ID.
 */
typedef void (*CanIf_BusOffNotificationType)(uint8 controller);
/** Implementation-owned configuration (SWS_CANIF_00144). Route counts and callback pointers are
 * derived from the validated plan, not independent ECUC parameters. All storage is immutable while
 * published. */
typedef struct {
    const CanIf_RxPduConfigType *receive;  /**< Rx routes; NULL only when receive_count is zero. */
    uint16 receive_count;                  /**< Rx route count, 0..32. */
    const CanIf_TxPduConfigType *transmit; /**< Tx routes; NULL only when transmit_count is zero. */
    uint16 transmit_count;                 /**< Tx route count, 0..32. */
    CanIf_ModeNotificationType
        controller_mode; /**< Required CanIfDispatchUserCtrlModeIndication callback. */
    CanIf_BusOffNotificationType bus_off; /**< Required CanIfDispatchUserCtrlBusOff callback. */
    uint8 controller;                     /**< Abstract controller ID (CanIfCtrlId). */
    uint8 driver_controller;              /**< Mapped driver ID (CanControllerId). */
    Can_HwHandleType receive_hoh; /**< Driver receive object (CanObjectId via CanIfHrhIdSymRef). */
    Can_HwHandleType
        transmit_hoh; /**< Driver transmit object (CanObjectId via CanIfHthIdSymRef). */
} CanIf_ConfigType;
/** Initialize the one controller and reset all pending results (SWS_CANIF_00001).
 * @param[in] ConfigPtr Generated immutable configuration; invalid or NULL sets CanIf to UNINIT.
 * @par Return
 * None.
 * Preconditions: None; a repeated call silently ends the previous lifetime.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: Configuration, route tables and callbacks remain valid until DeInit or the next Init.
 */
void CanIf_Init(const CanIf_ConfigType *ConfigPtr);
/** Release configuration and silently end pending requests (SWS_CANIF_91002).
 * @par Return
 * None.
 * Preconditions: None; calling while uninitialized has no further effect.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: The configuration pointer is released; no completion is published for discarded
 * requests.
 */
void CanIf_DeInit(void);
/** Request one Classical CAN frame without CanIf buffering (SWS_CANIF_00005).
 * @param[in] TxPduId A configured Tx L-PDU handle.
 * @param[in] PduInfoPtr Non-NULL caller-owned descriptor and payload, 0 to the configured maximum
 * of at most 8 bytes; metadata is unused.
 * @return E_OK when the driver accepts; E_NOT_OK for invalid inputs, a stopped/offline channel or
 * driver refusal, including CAN_BUSY. Preconditions: Initialized, controller STARTED and PDU mode
 * ONLINE; otherwise E_NOT_OK. Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: Neither descriptor nor payload is retained; the driver copies accepted data before
 * return. Completion may precede return.
 */
Std_ReturnType CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
/** Request the mapped driver mode transition (SWS_CANIF_00003).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[in] ControllerMode CAN_CS_STARTED, CAN_CS_STOPPED or CAN_CS_SLEEP, subject to the driver
 * state machine.
 * @return E_OK if the driver accepts; E_NOT_OK otherwise. Completion is reported by
 * ControllerModeIndication. Preconditions: CanIf initialized; otherwise E_NOT_OK. Context: task or
 * upper-layer callback; not inside an exclusive area. Reentrancy: reentrant for different
 * controllers; not for the same controller. Ownership: None.
 */
Std_ReturnType CanIf_SetControllerMode(uint8 ControllerId, Can_ControllerStateType ControllerMode);
/** Read the mapped driver mode (SWS_CANIF_00229).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[out] ControllerModePtr Non-NULL caller storage; written only when the driver returns E_OK.
 * @return E_OK with the queried value; E_NOT_OK for an unknown controller, NULL output or
 * unavailable driver value. Preconditions: CanIf initialized; otherwise E_NOT_OK with output
 * unchanged. Context: task or upper-layer callback; not inside an exclusive area. Reentrancy: not
 * reentrant for the same controller; only one controller is configured. Ownership: Output storage
 * is borrowed for this call and never retained; refusal preserves it.
 */
Std_ReturnType CanIf_GetControllerMode(uint8 ControllerId,
                                       Can_ControllerStateType *ControllerModePtr);
/** Read the mapped driver error state (SWS_CANIF_91001).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[out] ErrorStatePtr Non-NULL caller storage; written only when the driver returns E_OK.
 * @return E_OK with the queried value; E_NOT_OK for an unknown controller, NULL output or
 * unavailable driver value. Preconditions: CanIf initialized; otherwise E_NOT_OK with output
 * unchanged. Context: task or upper-layer callback; not inside an exclusive area. Reentrancy: not
 * reentrant for the same controller; only one controller is configured. Ownership: Output storage
 * is borrowed for this call and never retained; refusal preserves it.
 */
Std_ReturnType CanIf_GetControllerErrorState(uint8 ControllerId, Can_ErrorStateType *ErrorStatePtr);
/** Read the mapped driver receive error count.
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[out] RxErrorCounterPtr Non-NULL caller storage; written only when the driver returns E_OK.
 * @return E_OK with the queried value; E_NOT_OK for an unknown controller, NULL output or
 * unavailable driver value. Preconditions: CanIf initialized; otherwise E_NOT_OK with output
 * unchanged. Context: task or upper-layer callback; not inside an exclusive area. Reentrancy: not
 * reentrant for the same controller; only one controller is configured. Ownership: Output storage
 * is borrowed for this call and never retained; refusal preserves it.
 */
Std_ReturnType CanIf_GetControllerRxErrorCounter(uint8 ControllerId, uint8 *RxErrorCounterPtr);
/** Read the mapped driver transmit error count.
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[out] TxErrorCounterPtr Non-NULL caller storage; written only when the driver returns E_OK.
 * @return E_OK with the queried value; E_NOT_OK for an unknown controller, NULL output or
 * unavailable driver value. Preconditions: CanIf initialized; otherwise E_NOT_OK with output
 * unchanged. Context: task or upper-layer callback; not inside an exclusive area. Reentrancy: not
 * reentrant for the same controller; only one controller is configured. Ownership: Output storage
 * is borrowed for this call and never retained; refusal preserves it.
 */
Std_ReturnType CanIf_GetControllerTxErrorCounter(uint8 ControllerId, uint8 *TxErrorCounterPtr);
/** Change PDU gating on the started controller (SWS_CANIF_00008).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[in] PduModeRequest OFFLINE, TX_OFFLINE or ONLINE; TX_OFFLINE_ACTIVE is not selected.
 * @return E_OK when committed in this lifetime; E_NOT_OK for an invalid mode/controller,
 * uninitialized or non-started driver. Preconditions: Initialized and actual driver STARTED;
 * otherwise E_NOT_OK. Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: None.
 */
Std_ReturnType CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest);
/** Read this controller's current PDU gating mode (SWS_CANIF_00009).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[out] PduModePtr Non-NULL caller storage; written only on E_OK.
 * @return E_OK with the current mode; E_NOT_OK preserves output.
 * Preconditions: Initialized and a valid controller; otherwise E_NOT_OK.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different channels; not for the same channel.
 * Ownership: The output pointer is borrowed only for this call.
 */
Std_ReturnType CanIf_GetPduMode(uint8 ControllerId, CanIf_PduModeType *PduModePtr);
/** Filter a received frame and synchronously route accepted data (SWS_CANIF_00006).
 * @param[in] Mailbox Non-NULL descriptor of the configured CanIf controller/HRH and 11-bit CAN ID.
 * @param[in] PduInfoPtr Non-NULL received descriptor and payload, 0 to 8 bytes; metadata is unused.
 * @par Return
 * None.
 * Preconditions: Initialized and receive-enabled; invalid, unknown or short frames are not
 * forwarded. Context: lower-layer callback, including interrupt context; not inside an exclusive
 * area. Reentrancy: reentrant. Ownership: The descriptors and frame are borrowed until return;
 * accepted data is forwarded outside CANIF_STATE, never retained.
 */
void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr);
/** Settle at most one accepted transmission (SWS_CANIF_00007).
 * @param[in] CanTxPduId Configured Tx L-PDU; unknown, duplicate or already-cancelled completions
 * are ignored.
 * @par Return
 * None.
 * Preconditions: Initialized; otherwise no action. A disabled Tx path consumes the result without
 * forwarding. Context: lower-layer callback, including interrupt context; not inside an exclusive
 * area. Reentrancy: reentrant. Ownership: No storage is retained; any upper confirmation runs
 * outside CANIF_STATE.
 */
void CanIf_TxConfirmation(PduIdType CanTxPduId);
/** Publish a driver state transition and cancel stopped requests (SWS_CANIF_00699).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @param[in] ControllerMode Completed STARTED, STOPPED or SLEEP transition; other modes are
 * ignored.
 * @par Return
 * None.
 * Preconditions: Initialized and configured controller; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: No storage is retained; mode and cancelled-request callbacks run outside CANIF_STATE.
 */
void CanIf_ControllerModeIndication(uint8 ControllerId, Can_ControllerStateType ControllerMode);
/** Apply bus-off gating and publish the driver event (SWS_CANIF_00218).
 * @param[in] ControllerId The single configured abstract CanIf controller; other IDs are rejected.
 * @par Return
 * None.
 * Preconditions: Initialized and configured controller; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: No storage is retained; upper callbacks run outside CANIF_STATE.
 */
void CanIf_ControllerBusOff(uint8 ControllerId);
#endif
