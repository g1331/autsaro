/** @file Strict ComM and BswM exclusive areas on the host.
 * Each module enters only its own area and makes cross-module calls outside it.
 * Configured exclusive-area declarations/provenance belong to full generation.
 */
#include "SchM_ComM.h"
#include "SchM_BswM.h"
#include "Can_HostLock.h"
void SchM_Enter_ComM_COMM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_COMM); }
void SchM_Exit_ComM_COMM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_COMM); }
void SchM_Enter_BswM_BSWM_STATE(void) { Can_HostAreaEnter(CAN_HOST_AREA_BSWM); }
void SchM_Exit_BswM_BSWM_STATE(void) { Can_HostAreaExit(CAN_HOST_AREA_BSWM); }
