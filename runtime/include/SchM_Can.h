/** @file
 * @brief Scheduled CAN Driver polling functions for the host target.
 */
#ifndef SCHM_CAN_H
#define SCHM_CAN_H

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

/** @brief Poll one pending host Rx frame and indicate it to CanIf. */
CAN_CODE void Can_MainFunction_Read(void);

/** @brief Poll one completed host Tx request and confirm it to CanIf. */
CAN_CODE void Can_MainFunction_Write(void);

/** @brief Notify CanIf of a completed host controller mode transition. */
CAN_CODE void Can_MainFunction_Wakeup(void);

/** @brief Poll configured bus-off events and notify CanIf once per event. */
CAN_CODE void Can_MainFunction_BusOff(void);

/** @brief Poll completed host controller mode transitions. */
CAN_CODE void Can_MainFunction_Mode(void);

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
#endif
