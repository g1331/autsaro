/** @file Independent, non-recursive Dcm area with nesting detection. */
#include "ComStack_Cfg.h"
#include "SchM_Dcm.h"
#include "Can_HostLock.h"
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
RTE_Rte_CODE void SchM_Enter_Dcm_DCM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_DCM); }
RTE_Rte_CODE void SchM_Exit_Dcm_DCM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_DCM); }
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_Diagnostic_Host.c -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_Diagnostic_Host.c -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif
