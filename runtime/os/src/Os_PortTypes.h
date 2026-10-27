/** @file Native vector callbacks and Task-entry continuation address representations. */
#ifndef AUTOSAR_OS_PORT_TYPES_H
#define AUTOSAR_OS_PORT_TYPES_H
#include "Os_Version.h"
#include "Platform_Types.h"
#include <stdint.h>
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PLATFORM_AR_RELEASE_MAJOR_VERSION) || !defined(PLATFORM_AR_RELEASE_MINOR_VERSION)
#error "Os_PortTypes.h -> Platform_Types.h: missing AUTOSAR release"
#elif (PLATFORM_AR_RELEASE_MAJOR_VERSION != 4U) || (PLATFORM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_PortTypes.h -> Platform_Types.h: incompatible AUTOSAR release"
#endif

/** Native interrupt callback result and argument ABI used by both controlled ports. */
typedef uint32 (*Os_InterruptHandlerType)(void);
/* The native unwinder consumes a PC representation, not a callable C function. */
#ifdef _WIN32
typedef void *Os_PortRestartAddressType;
#else
typedef uintptr_t Os_PortRestartAddressType;
#endif
#endif
