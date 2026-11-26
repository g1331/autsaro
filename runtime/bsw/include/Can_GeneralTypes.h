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

/** SWS_Can_00416; CAN identifier with width and FD flags. */
typedef uint32 Can_IdType;

/** Can_IdType layout (SWS_Can_00416). Bit 29 is reserved. */
#define CAN_ID_EXTENDED_FLAG 0x80000000u
#define CAN_ID_FD_FLAG       0x40000000u
#define CAN_ID_EXTENDED_MAX  0x1fffffffu
#define CAN_ID_STANDARD_MAX  0x7ffu

/** Check an encoded Classical CAN identifier at a driver or host boundary.
 * @param[in] id Encoded CAN identifier.
 * @return TRUE for standard or extended Classical CAN, FALSE for other encodings.
 * Preconditions: None.
 * Context: Any.
 * Reentrancy: Reentrant.
 * Ownership: No shared state or retained references.
 */
static inline boolean Can_IsClassicalId(Can_IdType id)
{
    return ((id & CAN_ID_EXTENDED_FLAG) != 0u)
               ? ((id & ~CAN_ID_EXTENDED_FLAG) <= CAN_ID_EXTENDED_MAX)
               : (id <= CAN_ID_STANDARD_MAX);
}
#define CAN_CLASSICAL_MAX_LENGTH 8u  /**< Maximum Classical payload in bytes. */
#define CAN_FD_MAX_LENGTH        64u /**< Maximum FD payload in bytes. */
#define CAN_FD_MAX_DLC           15u /**< Largest raw CAN DLC. */

/** Check an encoded CAN identifier.
 * @param[in] id Encoded CAN identifier including the FD flag.
 * @return TRUE for the four supported formats with an in-range numeric identifier.
 * Preconditions: None.
 * Context: Any.
 * Reentrancy: Reentrant.
 * Ownership: No shared state or retained references.
 */
static inline boolean Can_IsValidId(Can_IdType id)
{
    return Can_IsClassicalId(id & ~CAN_ID_FD_FLAG);
}

/** Decode a raw CAN FD DLC into a wire payload length.
 * @param[in] dlc Raw DLC, 0 through CAN_FD_MAX_DLC.
 * @return Wire payload bytes, or zero for an out-of-range DLC.
 * Preconditions: None.
 * Context: Any.
 * Reentrancy: Reentrant.
 * Ownership: No shared state or retained references.
 */
static inline uint8 Can_FdDlcToLength(uint8 dlc)
{
    static const uint8 lengths[CAN_FD_MAX_DLC + 1u] = {
        0u, 1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 12u, 16u, 20u, 24u, 32u, 48u, 64u};
    return (dlc <= CAN_FD_MAX_DLC) ? lengths[dlc] : 0u;
}

/** Encode an FD request length, rounding up to the next available DLC.
 * @param[in] length Logical payload bytes.
 * @return Raw DLC, or UINT8_MAX when length exceeds CAN_FD_MAX_LENGTH.
 * Preconditions: None.
 * Context: Any.
 * Reentrancy: Reentrant.
 * Ownership: No shared state or retained references.
 */
static inline uint8 Can_FdLengthToDlc(uint8 length)
{
    uint8 dlc = length;
    if (length > CAN_FD_MAX_LENGTH) {
        dlc = (uint8)UINT8_MAX;
    } else if (length > CAN_CLASSICAL_MAX_LENGTH) {
        dlc = CAN_CLASSICAL_MAX_LENGTH + 1u;
        while ((dlc < CAN_FD_MAX_DLC) && (Can_FdDlcToLength(dlc) < length)) {
            ++dlc;
        }
    } else {
        /* The byte count is already the DLC. */
    }
    return dlc;
}

/** Check the byte count of an already encoded wire frame.
 * @param[in] id Encoded CAN identifier; only the FD flag is used.
 * @param[in] length Payload bytes on the wire.
 * @return TRUE if this frame format can carry exactly that byte count.
 * Preconditions: None.
 * Context: Any.
 * Reentrancy: Reentrant.
 * Ownership: No shared state or retained references.
 */
static inline boolean Can_IsWireLength(Can_IdType id, PduLengthType length)
{
    return (length <= CAN_CLASSICAL_MAX_LENGTH) ||
           (((id & CAN_ID_FD_FLAG) != 0u) && (length <= CAN_FD_MAX_LENGTH) &&
            (Can_FdDlcToLength(Can_FdLengthToDlc((uint8)length)) == length));
}

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
