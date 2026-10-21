/** @file
 * @brief R24-11 Can Driver public types for the bounded host profile.
 */
#ifndef CAN_GENERALTYPES_H
#define CAN_GENERALTYPES_H
#include "Can_Version.h"

#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "Can_GeneralTypes.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can_GeneralTypes.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif

/** SWS_Can_00416; the host profile accepts only 11-bit standard identifiers. */
typedef uint32 Can_IdType;
/** SWS_Can_00429 */
typedef uint16 Can_HwHandleType;

/** SWS_CAN_00496 */
typedef struct {
    Can_IdType CanId;     /**< Received CAN identifier. */
    Can_HwHandleType Hoh; /**< Receive hardware object handle. */
    uint8 ControllerId;   /**< Abstract CanIf controller identifier. */
} Can_HwType;

/** SWS_Can_00415 */
typedef struct {
    PduIdType swPduHandle; /**< Software PDU handle for confirmation routing. */
    uint8 length;          /**< Number of valid payload bytes. */
    Can_IdType id;         /**< Encoded CAN identifier. */
    uint8 *sdu;            /**< Caller-owned payload buffer. */
} Can_PduType;

/** SWS_Can_91013 */
typedef enum {
    CAN_CS_UNINIT = 0x00,  /**< Driver is not initialized. */
    CAN_CS_STARTED = 0x01, /**< Controller accepts frame traffic. */
    CAN_CS_STOPPED = 0x02, /**< Controller is stopped. */
    CAN_CS_SLEEP = 0x03    /**< Logical sleep state of this host controller. */
} Can_ControllerStateType;

/** SWS_Can_91003; the virtual controller never reports CAN_ERRORSTATE_PASSIVE. */
typedef enum {
    CAN_ERRORSTATE_ACTIVE = 0,  /**< Controller can communicate normally. */
    CAN_ERRORSTATE_PASSIVE = 1, /**< Controller does not send active error frames. */
    CAN_ERRORSTATE_BUSOFF = 2   /**< Controller does not participate on the bus. */
} Can_ErrorStateType;

/** SWS_Can_00039: Can_Write found the transmit object occupied. */
#define CAN_BUSY 0x02u
/** SWS_Can_91020: an already received frame was overwritten before CanIf took it. */
#define CAN_E_DATALOST 0x01u

#endif
