/** @file Configured Det runtime notification for controlled ECU receipts. */
#include "Det_Externals.h"
#include "Ecu_Target.h"
#define DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE
#include "Det_MemMap.h"
DET_CALLOUT_CODE Std_ReturnType Ecu_DetRuntimeCallout(uint16 ModuleId,
                                                      uint8 InstanceId,
                                                      uint8 ApiId,
                                                      uint8 ErrorId)
{
    (void)InstanceId;
    if ((ModuleId == 35u) && (ApiId == 0x06u) && ((ErrorId == 0xc0u) || (ErrorId == 0xd0u))) {
        Ecu_TargetReportTransportTimeout();
    }
    return E_OK;
}
#define DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE
#include "Det_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(DET_AR_RELEASE_MAJOR_VERSION) || !defined(DET_AR_RELEASE_MINOR_VERSION)
#error "Det_Callouts.c -> Det_Externals.h: missing AUTOSAR release"
#elif (DET_AR_RELEASE_MAJOR_VERSION != 4U) || (DET_AR_RELEASE_MINOR_VERSION != 10U)
#error "Det_Callouts.c -> Det_Externals.h: incompatible AUTOSAR release"
#endif
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Det_Callouts.c -> Det_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Det_Callouts.c -> Det_MemMap.h: incompatible AUTOSAR release"
#endif
