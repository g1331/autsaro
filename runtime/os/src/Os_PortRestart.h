/** @file Private native Task-entry unwind boundary, shared by the sole OS port. */
#ifndef AUTOSAR_OS_PORT_RESTART_H
#define AUTOSAR_OS_PORT_RESTART_H
#include "Os_Version.h"
#include "Os_PortTypes.h"

#ifndef _WIN32
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/* Initialize the provider's shared unwind register table before schedulable actors exist.
 * Returns 1 after a complete startup-stack walk, otherwise 0. */
OS_MEMMAP int Os_PortRestartPrepare(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/* Enter once on the registered native Task thread. Each restart discards the
 * outgoing C frames and invokes entry again on that same physical stack. */
OS_MEMMAP void Os_PortTaskLoop(void (*entry)(void *context), void *argument)
    __attribute__((noreturn));

/* Called only after successful automotive activation settlement and rescheduling.
 * Invalid/unhandled native unwinding stops the OS; it cannot return success. */
OS_MEMMAP void Os_PortRestart(void) __attribute__((noreturn));

/* Borrow the assembler continuation in the representation required by the native unwinder. */
OS_MEMMAP Os_PortRestartAddressType Os_PortRestartAddress(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_PortRestart.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_PortRestart.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
