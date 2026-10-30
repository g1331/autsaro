/** @file Private driver operations; state remains in Can.c. */
#ifndef CAN_INTERNAL_H
#define CAN_INTERNAL_H
#include "Can_Version.h"
#include "Can_Host.h"
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
CAN_CODE EcuStatus Can_InternalHostFlush(void);
CAN_CODE uint64 Can_InternalHostTransmitToken(void);
CAN_CODE Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64 token);
CAN_CODE EcuStatus Can_InternalInject(uint32 id, uint8 dlc, const uint8 data[8], uint64 now_ms);
/* Private ingress for a frame already accepted by the platform, not host admission. */
CAN_CODE EcuStatus Can_InternalRxIndication(uint32 id,
                                            uint8 dlc,
                                            const uint8 data[8],
                                            uint64 now_ms);
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Can_Internal.h -> Can_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Can_Internal.h -> Can_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
