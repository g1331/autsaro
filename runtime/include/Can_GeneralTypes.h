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
    CAN_CS_SLEEP = 0x03    /**< Standard sleep state; unsupported by this host controller. */
} Can_ControllerStateType;

#endif
