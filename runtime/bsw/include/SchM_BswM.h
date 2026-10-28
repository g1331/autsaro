/** @file BswM exclusive area and main function for the selected all-IMMEDIATE
 * configuration. */
#ifndef SCHM_BSWM_H
#define SCHM_BSWM_H
#include "Rte_Version.h"
#include "SchM_BswM_Type.h"
/** Preserve the C ABI for C++ consumers without conditional directives in mapped sections. */
#ifdef __cplusplus
#define SCHM_BSWM_LINKAGE extern "C" /**< C ABI for C++ consumers. */
#else
#define SCHM_BSWM_LINKAGE /**< C declarations already have C linkage. */
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
SCHM_BSWM_LINKAGE BSWM_CODE void BswM_MainFunction(void);
#define BSWM_STOP_SEC_CODE
#include "BswM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_BswM.h -> BswM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_BswM.h -> BswM_MemMap.h: incompatible AUTOSAR release"
#endif
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
/** Enter the BswM state area (SWS_Rte_07250).
 * @par Return
 * None.
 * Preconditions: no exclusive area is held by this context.
 * Context: task or callback, including interrupt.
 * Reentrancy: not reentrant within one context; concurrent contexts serialize.
 * Ownership: this context holds the area until its matching Exit.
 */
SCHM_BSWM_LINKAGE RTE_Rte_CODE void SchM_Enter_BswM_BSWM_STATE(void);
/** Leave the BswM state area (SWS_Rte_07253).
 * @par Return
 * None.
 * Preconditions: this context holds BSWM_STATE.
 * Context: the same context as the matching Enter.
 * Reentrancy: not reentrant within one context.
 * Ownership: releases this context's area.
 */
SCHM_BSWM_LINKAGE RTE_Rte_CODE void SchM_Exit_BswM_BSWM_STATE(void);
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_BswM.h -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_BswM.h -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif

#endif
