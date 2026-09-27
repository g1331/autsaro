/** @file
 * @brief Scheduled CAN Driver polling functions for the host target.
 */
#ifndef SCHM_CAN_H
#define SCHM_CAN_H

/** @brief Poll one pending host Rx frame and indicate it to CanIf. */
void Can_MainFunction_Read(void);

#endif
