/** @file Strict ComM and BswM exclusive areas on the host.
 * Each module enters only its own area and makes cross-module calls outside it.
 * Configured exclusive-area declarations/provenance belong to full generation.
 */
#include "SchM_ComM.h"
#include "SchM_BswM.h"
#include "Can_HostLock.h"
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
RTE_Rte_CODE void SchM_Enter_ComM_COMM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_COMM); }
RTE_Rte_CODE void SchM_Exit_ComM_COMM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_COMM); }
RTE_Rte_CODE void SchM_Enter_BswM_BSWM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_BSWM); }
RTE_Rte_CODE void SchM_Exit_BswM_BSWM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_BSWM); }
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_Mode_Host.c -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_Mode_Host.c -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif
