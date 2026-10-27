/** @file RTE trusted-context lifecycle; Start follows BSW/SchM Init. */
#ifndef RTE_MAIN_H
#define RTE_MAIN_H
#include "Rte_Version.h"
#include "Rte.h"
#ifdef __cplusplus
extern "C" {
#endif
/** Immutable SchM partition selection for this generated ECU. */
typedef struct {
    uint16 partition; /**< Validated partition handle; no other handle is accepted. */
} SchM_ConfigType;
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
/** Start the RTE once for this ECU lifetime.
 * @return E_OK after start; RTE_E_LIMIT on an invalid lifecycle context or repeated allocation.
 *
 * Preconditions: BSW and SchM initialized; StartupHook context, otherwise RTE_E_LIMIT.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE Std_ReturnType Rte_Start(void);
/** Stop component execution without permitting another allocation.
 * @return E_OK after stop; RTE_E_LIMIT for an invalid lifecycle context.
 *
 * Preconditions: Trusted shutdown context; rejected calls preserve the started state.
 *
 * Context: ShutdownHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE Std_ReturnType Rte_Stop(void);
/** Initialize the selected SchM partition once.
 * @param[in] ConfigPtr Non-NULL configuration with the generated partition handle.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and no prior allocation; invalid or repeated calls have no
 * effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE void SchM_Init(const SchM_ConfigType *ConfigPtr);
/** Enable SchM processing after initialization.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and SchM initialized; otherwise no effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE void SchM_Start(void);
/** Enable configured timing after SchM start.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and SchM started; otherwise no effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE void SchM_StartTiming(void);
/** Retire SchM permanently for this ECU lifetime.
 * @par Return
 * None.
 *
 * Preconditions: Trusted shutdown context; unallocated state stays unchanged.
 *
 * Context: ShutdownHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE void SchM_Deinit(void);
/** Read whether SchM processing has started.
 * @return TRUE in started or timing state; FALSE otherwise.
 *
 * Preconditions: Lifecycle changes are serialized by the owner; no concurrent lifecycle writer.
 *
 * Context: StartupHook, ShutdownHook or ECU owner task; no concurrent lifecycle writer.
 *
 * Reentrancy: reentrant when lifecycle state is stable.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE boolean Ecu_SchMReady(void);
/** Read whether SchM timing has started.
 * @return TRUE only in timing state; FALSE otherwise.
 *
 * Preconditions: Lifecycle changes are serialized by the owner; no concurrent lifecycle writer.
 *
 * Context: StartupHook, ShutdownHook or ECU owner task; no concurrent lifecycle writer.
 *
 * Reentrancy: reentrant when lifecycle state is stable.
 *
 * Ownership: No pointer is retained.
 */
RTE_Rte_CODE boolean Ecu_SchMTimingActive(void);
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
#define RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED
#include "Rte_MemMap.h"
extern const SchM_ConfigType Ecu_SchMConfig RTE_Rte_CONFIG_DATA_PREBUILD_UNSPECIFIED;
#define RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Rte_Main.h -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Rte_Main.h -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif
#ifdef __cplusplus
}
#endif
#endif
