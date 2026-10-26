/** @file Private receive ordering adapter for the controlled ECU owner. */
#ifndef CANTP_INTERNAL_H
#define CANTP_INTERNAL_H
#include "CanTp_Version.h"
#include "CanTp.h"
#define CANTP_START_SEC_CODE
#include "CanTp_MemMap.h"
/** The serialized ECU owner brackets Can_MainFunction_Read with this ordering hint.
 * TRUE means that epoch's main function is still pending; FALSE ends the receive call.
 * It supplies ordering only, without a clock or a change to standard CanTp callbacks.
 */
CANTP_CODE void Ecu_CanTpReceptionBeforeMain(boolean pending);
#define CANTP_STOP_SEC_CODE
#include "CanTp_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "CanTp_Internal.h -> CanTp_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanTp_Internal.h -> CanTp_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
