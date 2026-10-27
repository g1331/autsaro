/** @file Host exclusive areas of the communication stack. The driver, CanIf,
 * Det, PduR, LSduR, Com and CanTp areas are strict: separate resources that nest with nothing.
 * The selected target lock provider implements them.
 */
#include "SchM_CanIf.h"
#include "SchM_Can.h"
#include "SchM_Det.h"
#include "SchM_Com.h"
#include "SchM_PduR.h"
#include "SchM_LSduR.h"
#include "SchM_CanTp.h"
#include "Can_HostLock.h"
#define RTE_Rte_START_SEC_CODE
#include "Rte_MemMap.h"
RTE_Rte_CODE void SchM_Enter_Can_CAN_STATE(void) { Can_Lock(); }
RTE_Rte_CODE void SchM_Exit_Can_CAN_STATE(void) { Can_Unlock(); }
RTE_Rte_CODE void SchM_Enter_CanIf_CANIF_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_CANIF); }
RTE_Rte_CODE void SchM_Exit_CanIf_CANIF_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_CANIF); }
RTE_Rte_CODE void SchM_Enter_Det_DET_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_DET); }
RTE_Rte_CODE void SchM_Exit_Det_DET_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_DET); }
RTE_Rte_CODE void SchM_Enter_Com_COM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_COM); }
RTE_Rte_CODE void SchM_Exit_Com_COM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_COM); }
RTE_Rte_CODE void SchM_Enter_PduR_PDUR_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_PDUR); }
RTE_Rte_CODE void SchM_Exit_PduR_PDUR_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_PDUR); }
RTE_Rte_CODE void SchM_Enter_LSduR_LSDUR_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_LSDUR); }
RTE_Rte_CODE void SchM_Exit_LSduR_LSDUR_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_LSDUR); }
RTE_Rte_CODE void SchM_Enter_CanTp_CANTP_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_CANTP); }
RTE_Rte_CODE void SchM_Exit_CanTp_CANTP_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_CANTP); }
#define RTE_Rte_STOP_SEC_CODE
#include "Rte_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "SchM_CanIf_Host.c -> Rte_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "SchM_CanIf_Host.c -> Rte_MemMap.h: incompatible AUTOSAR release"
#endif
