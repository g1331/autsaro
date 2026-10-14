/** @file Host exclusive areas of the communication stack (ADR 0018). The driver, CanIf,
 * Det, PduR, LSduR and Com areas are strict: separate resources that nest with nothing.
 * The selected target lock provider implements them.
 */
#include "SchM_CanIf.h"
#include "SchM_Can.h"
#include "SchM_Det.h"
#include "SchM_Com.h"
#include "SchM_PduR.h"
#include "SchM_LSduR.h"
#include "Can_HostLock.h"
void SchM_Enter_Can_CAN_STATE(void) { Can_Lock(); }
void SchM_Exit_Can_CAN_STATE(void) { Can_Unlock(); }
void SchM_Enter_CanIf_CANIF_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_CANIF); }
void SchM_Exit_CanIf_CANIF_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_CANIF); }
void SchM_Enter_Det_DET_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_DET); }
void SchM_Exit_Det_DET_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_DET); }
void SchM_Enter_Com_COM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_COM); }
void SchM_Exit_Com_COM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_COM); }
void SchM_Enter_PduR_PDUR_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_PDUR); }
void SchM_Exit_PduR_PDUR_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_PDUR); }
void SchM_Enter_LSduR_LSDUR_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_LSDUR); }
void SchM_Exit_LSduR_LSDUR_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_LSDUR); }
