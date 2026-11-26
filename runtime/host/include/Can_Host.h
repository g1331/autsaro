/** @file Explicit host injection and output adapter; no driver storage. */
#ifndef CAN_HOST_H
#define CAN_HOST_H
#include "Can.h"
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
/** @brief State of the host virtual CAN controller. */
typedef enum {
    CAN_STOPPED = 0, /**< Controller is stopped. */
    CAN_STARTED = 1, /**< Controller accepts frame traffic. */
    CAN_BUS_OFF = 2, /**< Controller rejects frame traffic after a bus-off event. */
    CAN_SLEEP = 3    /**< Logical sleep state for the virtual controller. */
} CanMode;

/** Set the virtual mode of a configured driver controller.
 * @param[in] controller Configured driver identifier.
 * @param[in] mode Requested host controller mode.
 * @par Return
 * None.
 * Preconditions: Driver initialized and supported transition; invalid requests have no effect.
 * Context: Host control, outside exclusive areas.
 * Reentrancy: Different controllers have independent state and indications.
 * Ownership: No pointer retained.
 */
CAN_CODE void Can_HostSetControllerMode(uint8 controller, CanMode mode);
/** Inject a wire frame on the specified driver controller's channel.
 * @param[in] controller Configured driver identifier.
 * @param[in] id Encoded CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Frame payload, borrowed only until it is copied.
 * @return Successful admission or host controller, frame or slot status.
 * Preconditions: Initialized started controller and valid wire frame.
 * Context: Host reception, outside exclusive areas.
 * Reentrancy: Reentrant for different controllers.
 * Ownership: Copies the payload before returning and retains no pointer.
 */
CAN_CODE EcuStatus Can_HostInjectChannel(uint8 controller, uint32 id, uint8 dlc, const uint8 *data);
/** Capture the queued token during this controller's output callback.
 * @param[in] controller Driver identifier supplied with the actual output.
 * @return Nonzero token for the current queued transmission, zero otherwise.
 * Preconditions: A queued output callback is active for this controller.
 * Context: Host sink callback, outside exclusive areas.
 * Reentrancy: Reentrant read serialized by CAN_STATE.
 * Ownership: Returns the token by value.
 */
CAN_CODE uint64 Can_HostTransmitTokenFor(uint8 controller);
/** Report physical completion using the controller and token stored with the output.
 * @param[in] controller Actual driver identifier from the output.
 * @param[in] handle Actual software PDU handle from the same output.
 * @param[in] token Token captured during that output's driver flush.
 * @return E_OK, CAN_BUSY for an occupied confirmation slot, or E_NOT_OK for an invalid token.
 * Preconditions: Confirmed physical write and matching output ticket and identity.
 * Context: Queued completion owner, outside exclusive areas.
 * Reentrancy: Reentrant for different controllers.
 * Ownership: Copies scalar identity into the controller's matching confirmation slot.
 */
CAN_CODE Std_ReturnType Can_HostCompleteTransmitFor(uint8 controller,
                                                    PduIdType handle,
                                                    uint64 token);

/* Private driver operations used by the typed host forwarding definitions. */
/** @brief Driver implementation: Change the virtual controller state.
 * @param[in] mode New controller state.
 * @par Return
 * None.
 * Preconditions: Driver initialized and supported transition; otherwise no state change.
 * Context: Host control, outside exclusive areas.
 * Reentrancy: State serialized by CAN_STATE; indications serialized outside the area.
 * Ownership: No pointer retained; may publish mode/bus-off callbacks after unlocking.
 */
CAN_CODE void Can_InternalSetMode(CanMode mode);
/** @brief Driver implementation: Read the virtual controller state.
 * @return Current controller state.
 * Preconditions: None; before initialization returns CAN_STOPPED.
 * Context: Any caller, outside exclusive areas.
 * Reentrancy: Reentrant read serialized by CAN_STATE; do not call while holding another area.
 * Ownership: Returns a value; no pointer retained.
 */
CAN_CODE CanMode Can_InternalGetMode(void);
/** @brief Driver implementation: Send a host frame while retaining its generated CanIf PDU handle.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] id Encoded CAN identifier.
 * @param[in] dlc Number of valid payload bytes, 0..64.
 * @param[in] data Nonnull caller-owned payload; copied before return.
 * @return Host transmission status.
 * Preconditions: Initialized started driver, valid identifier/DLC and nonnull data; else refuses.
 * Context: Host transmit owner, outside exclusive areas.
 * Reentrancy: Driver serializes admission; pending/in-flight frame returns ECU_ERR_CAN_BUSY.
 * Ownership: Copies payload; physical sink borrows the driver snapshot until its return.
 */
CAN_CODE EcuStatus Can_InternalTransmitPdu(PduIdType pdu_id,
                                           uint32 id,
                                           uint8 dlc,
                                           const uint8 *data);

/** @brief Complete the pending host transmission through the configured output callback.
 * @return Host output status, or ECU_OK if no frame is pending.
 * Preconditions: Driver initialized; otherwise ECU_ERR_CONFIG; empty queue succeeds.
 * Context: Owner/output dispatch, outside exclusive areas.
 * Reentrancy: Driver state serialized by CAN_STATE; an in-flight frame is not flushed twice.
 * Ownership: Snapshots driver payload; sink borrows it only during callback outside CAN_STATE.
 */
CAN_CODE EcuStatus Can_HostFlush(void);
/** @brief Capture the actual queued transmission token inside the host sink.
 * @return Nonzero lifetime token only during the current queued driver flush;
 * zero when the selected profile does not enable queued completion.
 * Preconditions: Current queued driver sink invocation for a meaningful nonzero token.
 * Context: Host sink callback, outside exclusive areas.
 * Reentrancy: Reentrant read serialized by CAN_STATE.
 * Ownership: Returns scalar token; retains no caller storage.
 */
CAN_CODE uint64_t Can_HostTransmitToken(void);
/** @brief Publish a physically completed, owner-verified queued output.
 * @param[in] handle Actual swPduHandle stored with the output ticket.
 * @param[in] token Token captured during that output's driver flush.
 * @return E_OK when queued for Write polling, CAN_BUSY if a previous event
 * needs draining, E_NOT_OK for disabled/uninitialized, stale or cancelled tokens.
 * The controlled owner must prove the actual output ticket/handle/token and
 * physical write/flush before calling; no payload is queued or synthesized here.
 * Preconditions: Caller proved physical output and exact ticket/handle/token; driver initialized.
 * Context: Queued completion owner, outside exclusive areas.
 * Reentrancy: State serialized by CAN_STATE; previous pending confirmation returns CAN_BUSY.
 * Ownership: Stores handle/token only; caller remains responsible for matching actual output.
 */
CAN_CODE Std_ReturnType Can_HostCompleteTransmit(PduIdType handle, uint64_t token);

/** @brief Change the virtual controller state.
 * @param[in] mode New controller state.
 * @par Return
 * None.
 * Preconditions: Driver initialized and supported transition; otherwise no state change.
 * Context: Host control, outside exclusive areas.
 * Reentrancy: State serialized by CAN_STATE; indications serialized outside the area.
 * Ownership: No pointer retained; may publish mode/bus-off callbacks after unlocking.
 */
CAN_CODE static inline void Can_SetMode(CanMode mode) { Can_InternalSetMode(mode); }
/** @brief Read the virtual controller state.
 * @return Current controller state.
 * Preconditions: None; before initialization returns CAN_STOPPED.
 * Context: Any caller, outside exclusive areas.
 * Reentrancy: Reentrant read serialized by CAN_STATE; do not call while holding another area.
 * Ownership: Returns a value; no pointer retained.
 */
CAN_CODE static inline CanMode Can_GetMode(void) { return Can_InternalGetMode(); }
/** @brief Send a host frame while retaining its generated CanIf PDU handle.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] id Encoded CAN identifier.
 * @param[in] dlc Number of valid payload bytes, 0..64.
 * @param[in] data Nonnull caller-owned payload; copied before return.
 * @return Host transmission status.
 * Preconditions: Initialized started driver, valid identifier/DLC and nonnull data; else refuses.
 * Context: Host transmit owner, outside exclusive areas.
 * Reentrancy: Driver serializes admission; pending/in-flight frame returns ECU_ERR_CAN_BUSY.
 * Ownership: Copies payload; physical sink borrows the driver snapshot until its return.
 */
CAN_CODE static inline EcuStatus Can_TransmitPdu(PduIdType pdu_id,
                                                 uint32_t id,
                                                 uint8_t dlc,
                                                 const uint8_t *data)
{
    return Can_InternalTransmitPdu(pdu_id, id, dlc, data);
}
/** @brief Inject one received frame into the host CAN stack.
 * @param[in] id Encoded CAN identifier.
 * @param[in] dlc Number of valid payload bytes, 0..64.
 * @param[in] data Nonnull caller-owned payload; copied before return.
 * @param[in] now_ms Unused timestamp; does not advance logical time or
 * enter the CanIf receive indication.
 * @return Driver admission status; CanIf filtering and length checks are
 * observed through Det and the upper-layer indication, not this result.
 * Preconditions: Initialized started driver, encoded CAN ID, legal wire payload length and nonnull
 * data. Context: Host input control, outside exclusive areas. Reentrancy: Driver serializes
 * admission; a pending/in-flight receive returns ECU_ERR_CAN_BUSY. Ownership: Copies payload before
 * acceptance; upper indication borrows snapshot during callback.
 */
CAN_CODE EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t *data, uint64_t now_ms);

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
