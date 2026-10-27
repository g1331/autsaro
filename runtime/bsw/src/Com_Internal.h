/** @file Private scheduler implementation shared by generated SchM instances. */
#ifndef COM_INTERNAL_H
#define COM_INTERNAL_H
#include "Com_Version.h"
#include "Com.h"
#define COM_START_SEC_CODE
#include "Com_MemMap.h"
/** Owner supplies ordering only; this adapter has no clock or timer. */
COM_CODE void Ecu_ComReceptionBeforeMain(boolean pending);
COM_CODE void Ecu_ComMainFunctionRx(void);
COM_CODE void Ecu_ComMainFunctionTx(void);
#define COM_STOP_SEC_CODE
#include "Com_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Com_Internal.h -> Com_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com_Internal.h -> Com_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
