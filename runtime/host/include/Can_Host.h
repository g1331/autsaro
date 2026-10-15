/** @file Explicit host injection and output adapter; no driver storage. */
#ifndef CAN_HOST_H
#define CAN_HOST_H
#include "Can.h"
/** @brief State of the host virtual CAN controller. */
typedef enum {
    CAN_STOPPED = 0, /**< Controller is stopped. */
    CAN_STARTED = 1, /**< Controller accepts frame traffic. */
    CAN_BUS_OFF = 2, /**< Controller rejects frame traffic after a bus-off event. */
    CAN_SLEEP = 3    /**< Logical sleep state for the virtual controller. */
} CanMode;

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
/** @brief Complete the pending host transmission through the configured output callback.
 * @return Host output status, or ECU_OK if no frame is pending.
 */
CAN_CODE EcuStatus Can_HostFlush(void);
/** @brief Capture the actual queued transmission token inside the host sink.
 * @return Nonzero lifetime token only during the current queued driver flush;
 * zero when the selected profile does not enable queued completion.
 */
CAN_CODE uint64_t Can_HostTransmitToken(void);
/** @brief Publish a physically completed, owner-verified queued output.
 * @param[in] handle Actual swPduHandle stored with the output ticket.
 * @param[in] token Token captured during that output's driver flush.
 * @return E_OK when queued for Write polling, CAN_BUSY if a previous event
 * needs draining, E_NOT_OK for disabled/uninitialized, stale or cancelled tokens.
 * The controlled owner must prove the actual output ticket/handle/token and
 * physical write/flush before calling; no payload is queued or synthesized here.
 */
CAN_CODE Std_ReturnType Can_HostCompleteTransmit(PduIdType handle, uint64_t token);

/** @brief Change the virtual controller state.
 * @param[in] mode New controller state.
 */
CAN_CODE void Can_SetMode(CanMode mode);
/** @brief Read the virtual controller state.
 * @return Current controller state.
 */
CAN_CODE CanMode Can_GetMode(void);
/** @brief Send a host frame while retaining its generated CanIf PDU handle.
 * @param[in] pdu_id Generated CanIf transmit PDU identifier.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @return Host transmission status.
 */
CAN_CODE EcuStatus Can_TransmitPdu(PduIdType pdu_id,
                                   uint32_t id,
                                   uint8_t dlc,
                                   const uint8_t data[8]);
/** @brief Inject one received frame into the host CAN stack.
 * @param[in] id Standard CAN identifier.
 * @param[in] dlc Number of valid payload bytes.
 * @param[in] data Eight-byte frame buffer.
 * @param[in] now_ms Current host clock in milliseconds; not forwarded, since
 * CanIf_RxIndication carries no reception time.
 * @return Driver admission status; CanIf filtering and length checks are
 * observed through Det and the upper-layer indication, not this result.
 */
CAN_CODE EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
