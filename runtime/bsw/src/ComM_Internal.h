/** @file Private body for the generated per-channel SchM main wrapper.
 * One call consumes one configured period. Returned state is the resulting
 * state, not a second scheduler or host clock. Wrapper identity is generated.
 */
#ifndef COMM_INTERNAL_H
#define COMM_INTERNAL_H
#include "ComM_Version.h"
#include "ComM.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
COMM_CODE ComM_StateType ComM_RunChannel(void);
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM_Internal.h -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM_Internal.h -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
