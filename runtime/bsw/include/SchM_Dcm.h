/** @file Dcm exclusive area, main function and diagnostic session mode switch.
 */
#ifndef SCHM_DCM_H
#define SCHM_DCM_H
#include "Rte_Version.h"
#include "SchM_Dcm_Type.h"
#include "Rte_Dcm_Type.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(DCM_AR_RELEASE_MAJOR_VERSION) || !defined(DCM_AR_RELEASE_MINOR_VERSION)
#error "SchM_Dcm.h -> Rte_Dcm_Type.h: missing AUTOSAR release"
#elif (DCM_AR_RELEASE_MAJOR_VERSION != 4U) || (DCM_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_Dcm.h -> Rte_Dcm_Type.h: incompatible AUTOSAR release"
#endif
#include "Rte.h"
#ifdef __cplusplus
extern "C" {
#endif
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
/** Enter the Dcm state area (SWS_Rte_07250).
 * @par Return
 * None.
 * Preconditions: no exclusive area is held by this context.
 * Context: task or callback, including interrupt.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: this context holds the area until its matching Exit.
 */
RTE_Rte_CODE void SchM_Enter_Dcm_DCM_STATE(void);
/** Leave the Dcm state area (SWS_Rte_07253).
 * @par Return
 * None.
 * Preconditions: this context holds DCM_STATE.
 * Context: the same context as the matching Enter.
 * Reentrancy: not reentrant within one context.
 * Ownership: releases this context's area.
 */
RTE_Rte_CODE void SchM_Exit_Dcm_DCM_STATE(void);
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_Dcm.h -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_Dcm.h -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif

#define DCM_START_SEC_CODE
#include "Dcm_MemMap.h"
/** Process one configured main period (SWS_Dcm_00053).
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: scheduled ECU task; not inside an exclusive area.
 * Reentrancy: not reentrant. A call that overlaps the active executor or a mode
 * publication returns without work and does not advance the P2/S3 timers.
 * Ownership: application callbacks use call-local buffers, never live Rx/Tx
 * storage.
 */
DCM_CODE void Dcm_MainFunction(void);
#define DCM_STOP_SEC_CODE
#include "Dcm_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_Dcm.h -> Dcm_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_Dcm.h -> Dcm_MemMap.h: incompatible AUTOSAR release"
#endif
#define RTE_Dcm_START_SEC_CODE
#include "Rte_MemMap.h"
/** Publish the session mode through the selected immediate SchM interface.
 * @param[in] mode Configured session mode, distinct from the UDS session ID.
 * @return SCHM_E_OK after publishing the immediate mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant; the Dcm publisher orders updates.
 * Ownership: None.
 */
RTE_CODE Std_ReturnType
SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode);
/** Read the selected session mode.
 * @return Current mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
RTE_CODE Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void);
#define RTE_Dcm_STOP_SEC_CODE
#include "Rte_MemMap.h"
#ifdef __cplusplus
}
#endif
#endif
