/** @file
 * @brief R24-11 Can Driver public types for the bounded host profile.
 */
#ifndef CAN_GENERALTYPES_H
#define CAN_GENERALTYPES_H

#include "ComStack_Types.h"

/** SWS_Can_00416; the host profile accepts only 11-bit standard identifiers. */
typedef uint32 Can_IdType;
/** SWS_Can_00429 */
typedef uint16 Can_HwHandleType;

/** SWS_CAN_00496 */
typedef struct {
    Can_IdType CanId;
    Can_HwHandleType Hoh;
    uint8 ControllerId;
} Can_HwType;

/** SWS_Can_00415 */
typedef struct {
    PduIdType swPduHandle;
    uint8 length;
    Can_IdType id;
    uint8 *sdu;
} Can_PduType;

/** SWS_Can_91013 */
typedef enum {
    CAN_CS_UNINIT = 0x00,
    CAN_CS_STARTED = 0x01,
    CAN_CS_STOPPED = 0x02,
    CAN_CS_SLEEP = 0x03
} Can_ControllerStateType;

/** SWS_Can_91003; the virtual controller never reports CAN_ERRORSTATE_PASSIVE. */
typedef enum {
    CAN_ERRORSTATE_ACTIVE = 0,
    CAN_ERRORSTATE_PASSIVE = 1,
    CAN_ERRORSTATE_BUSOFF = 2
} Can_ErrorStateType;

/** SWS_Can_00039 */
#define CAN_BUSY 0x02u
/** SWS_Can_91020 */
#define CAN_E_DATALOST 0x01u

#endif
