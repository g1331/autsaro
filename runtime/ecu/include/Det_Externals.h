/** @file Generated from the selected source contract. */
#ifndef DET_EXTERNALS_H
#define DET_EXTERNALS_H
#include "Det_Version.h"
#include "Std_Types.h"
#define DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE
#include "Det_MemMap.h"
/** Forward the selected CanTp timeout to the ECU receipt.
 * @param[in] ModuleId Reporting module, 0..65535.
 * @param[in] InstanceId Reporting instance, 0..255.
 * @param[in] ApiId Reporting API, 0..255.
 * @param[in] ErrorId Runtime error, 0..255.
 * @return E_OK; other reports have no receipt side effect.
 * Preconditions: Selected CanTp timeouts are reported by the ECU owner.
 * Context: Called by Det outside DET_STATE.
 * Reentrancy: Filtering is stateless; receipt publication uses the ECU owner.
 * Ownership: No parameter is retained.
 */
DET_CALLOUT_CODE Std_ReturnType Ecu_DetRuntimeCallout(uint16 ModuleId,
                                                      uint8 InstanceId,
                                                      uint8 ApiId,
                                                      uint8 ErrorId);
#define DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE
#include "Det_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Det_Externals.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Det_Externals.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Det_Externals.h -> Det_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Det_Externals.h -> Det_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
