/** @file BswM exclusive area and main function for the selected all-IMMEDIATE
 * configuration. */
#ifndef SCHM_BSWM_H
#define SCHM_BSWM_H
#include "SchM_BswM_Type.h"
#ifdef __cplusplus
extern "C" {
#endif
#define BSWM_START_SEC_CODE
#include "BswM_MemMap.h"
/** Run deferred arbitration (SWS_BswM_00053); no DEFERRED input is selected.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: scheduled task; not inside an exclusive area.
 * Reentrancy: reentrant; it has no effect.
 * Ownership: None.
 */
void BswM_MainFunction(void);
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
/** Enter the BswM state area (SWS_Rte_07250).
 * @par Return
 * None.
 * Preconditions: no exclusive area is held by this context.
 * Context: task or callback, including interrupt.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: this context holds the area until its matching Exit.
 */
void SchM_Enter_BswM_BSWM_STATE(void);
/** Leave the BswM state area (SWS_Rte_07253).
 * @par Return
 * None.
 * Preconditions: this context holds BSWM_STATE.
 * Context: the same context as the matching Enter.
 * Reentrancy: not reentrant within one context.
 * Ownership: releases this context's area.
 */
void SchM_Exit_BswM_BSWM_STATE(void);
#ifdef __cplusplus
}
#endif
#endif
