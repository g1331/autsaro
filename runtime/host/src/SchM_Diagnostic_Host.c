/** @file Independent, non-recursive Dcm area with nesting detection (ADR 0018). */
#include "ComStack_Cfg.h"
#include "SchM_Dcm.h"
#include "Can_HostLock.h"
void SchM_Enter_Dcm_DCM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_DCM); }
void SchM_Exit_Dcm_DCM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_DCM); }
