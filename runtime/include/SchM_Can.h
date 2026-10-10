/** @file
 * @brief Scheduled CAN Driver polling functions for the host target.
 */
#ifndef SCHM_CAN_H
#define SCHM_CAN_H

/** @brief Poll one pending host Rx frame and indicate it to CanIf. */
void Can_MainFunction_Read(void);

/** @brief Poll one completed host Tx request and confirm it to CanIf. */
void Can_MainFunction_Write(void);

/** @brief Notify CanIf of a completed host controller mode transition. */
void Can_MainFunction_Wakeup(void);

/** @brief Poll configured bus-off events and notify CanIf once per event. */
void Can_MainFunction_BusOff(void);

/** @brief Poll completed host controller mode transitions. */
void Can_MainFunction_Mode(void);

#endif
