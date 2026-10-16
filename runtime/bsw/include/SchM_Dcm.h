/** @file Dcm exclusive area, main function and diagnostic session mode switch. */
#ifndef SCHM_DCM_H
#define SCHM_DCM_H
#include "SchM_Dcm_Type.h"
#ifdef __cplusplus
extern "C" {
#endif
#include "Rte_Dcm_Type.h"
#include "Rte.h"
/** Enter the Dcm state area (SWS_Rte_07250).
 * @return None.
 * Preconditions: no exclusive area is held by this context.
 * Context: task or callback, including interrupt.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: this context holds the area until its matching Exit.
 */
void SchM_Enter_Dcm_DCM_STATE(void);
/** Leave the Dcm state area (SWS_Rte_07251).
 * @return None.
 * Preconditions: this context holds DCM_STATE.
 * Context: the same context as the matching Enter.
 * Reentrancy: not reentrant within one context.
 * Ownership: releases this context's area.
 */
void SchM_Exit_Dcm_DCM_STATE(void);
/** Process one configured main period (SWS_Dcm_00053).
 * @return None.
 * Preconditions: initialized; otherwise no action.
 * Context: scheduled ECU task; not inside an exclusive area.
 * Reentrancy: not reentrant. A call that overlaps the active executor or a mode
 * publication returns without work and does not advance the P2/S3 timers.
 * Ownership: application callbacks use call-local buffers, never live Rx/Tx storage.
 */
#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
void Dcm_MainFunction(void);
#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
/** Publish the session mode through the selected immediate SchM interface.
 * @param[in] mode Configured session mode, distinct from the UDS session ID.
 * @return SCHM_E_OK after publishing the immediate mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant; the Dcm publisher orders updates.
 * Ownership: None.
 */
Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(
    Rte_ModeType_DcmDiagnosticSessionControl mode);
/** Read the selected session mode.
 * @return Current mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void);
#ifdef __cplusplus
}
#endif
#endif
