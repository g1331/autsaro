/** @file
 * @brief R24-11 Can Driver public types for the bounded host profile.
 */
#ifndef CAN_GENERALTYPES_H
#define CAN_GENERALTYPES_H

#include <stdint.h>
#include "ComStack_Types.h"

/** @brief Encoded CAN identifier; this host profile accepts only 11-bit Classic IDs. */
typedef uint32_t Can_IdType;
/** @brief Hardware transmit handle; the current host profile has one handle, zero. */
typedef uint16_t Can_HwHandleType;

/** @brief CAN L-SDU supplied by CanIf to Can_Write. */
typedef struct {
    PduIdType swPduHandle; /**< Software PDU handle for confirmation routing. */
    uint8_t length;        /**< Number of valid payload bytes. */
    Can_IdType id;         /**< Encoded CAN identifier. */
    uint8_t *sdu;          /**< Caller-owned payload buffer. */
} Can_PduType;

/** @brief Controller states exposed by the standard-facing host API. */
typedef enum {
    CAN_CS_UNINIT = 0x00,  /**< Driver is not initialized. */
    CAN_CS_STARTED = 0x01, /**< Controller accepts frame traffic. */
    CAN_CS_STOPPED = 0x02, /**< Controller is stopped. */
    CAN_CS_SLEEP = 0x03    /**< Logical sleep state of this host controller. */
} Can_ControllerStateType;

/** @brief CAN controller error states defined by R24-11. */
typedef enum {
    CAN_ERRORSTATE_ACTIVE = 0,  /**< Controller can communicate normally. */
    CAN_ERRORSTATE_PASSIVE = 1, /**< Controller does not send active error frames. */
    CAN_ERRORSTATE_BUSOFF = 2   /**< Controller does not participate on the bus. */
} Can_ErrorStateType;

/** @brief The single host transmit handle is still completing a previous request. */
#define CAN_BUSY 0x02u

#endif
