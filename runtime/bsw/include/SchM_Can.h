/** @file
 * @brief Scheduled CAN Driver polling functions for the host target.
 */
#ifndef SCHM_CAN_H
#define SCHM_CAN_H
#include "SchM_Can_Type.h"
#ifdef __cplusplus
extern "C" {
#endif

#define CAN_START_SEC_CODE
#include "Can_MemMap.h"

/** Poll received frames and route accepted data to CanIf.
 * @par Return
 * None.
 * Preconditions: Driver initialized; otherwise no indication.
 * Context: Scheduled ECU task, outside exclusive areas.
 * Reentrancy: not reentrant.
 * Ownership: Callbacks run outside CAN_STATE; no pointer is retained.
 */
CAN_CODE void Can_MainFunction_Read(void);

/** Publish a pending transmit confirmation to CanIf.
 * @par Return
 * None.
 * Preconditions: Driver initialized; otherwise no indication.
 * Context: Scheduled ECU task, outside exclusive areas.
 * Reentrancy: not reentrant.
 * Ownership: Callbacks run outside CAN_STATE; no pointer is retained.
 */
CAN_CODE void Can_MainFunction_Write(void);

/** Publish a pending controller mode transition to CanIf.
 * @par Return
 * None.
 * Preconditions: Driver initialized; otherwise no indication.
 * Context: Scheduled ECU task, outside exclusive areas.
 * Reentrancy: not reentrant.
 * Ownership: Callbacks run outside CAN_STATE; no pointer is retained.
 */
CAN_CODE void Can_MainFunction_Wakeup(void);

/** Publish a pending bus-off event once to CanIf.
 * @par Return
 * None.
 * Preconditions: Driver initialized; otherwise no indication.
 * Context: Scheduled ECU task, outside exclusive areas.
 * Reentrancy: not reentrant.
 * Ownership: Callbacks run outside CAN_STATE; no pointer is retained.
 */
CAN_CODE void Can_MainFunction_BusOff(void);

/** Publish a pending controller mode transition to CanIf.
 * @par Return
 * None.
 * Preconditions: Driver initialized; otherwise no indication.
 * Context: Scheduled ECU task, outside exclusive areas.
 * Reentrancy: not reentrant.
 * Ownership: Callbacks run outside CAN_STATE; no pointer is retained.
 */
CAN_CODE void Can_MainFunction_Mode(void);

#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"

/** Enter the driver state area (SWS_Rte_07250).
 * @par Return
 * None.
 * Preconditions: No exclusive area is held by this context.
 * Context: Task or callback, including interrupt context.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: Held by this context until its matching Exit.
 */
void SchM_Enter_Can_CAN_STATE(void);
/** Exit the driver state area (SWS_Rte_07253).
 * @par Return
 * None.
 * Preconditions: This context holds CAN_STATE.
 * Context: Task or callback, including interrupt context.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: Releases this context's area.
 */
void SchM_Exit_Can_CAN_STATE(void);
#ifdef __cplusplus
}
#endif
#endif
