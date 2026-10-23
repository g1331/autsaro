/** @file Private native Task-entry unwind boundary, shared by the sole OS port. */
#ifndef AUTOSAR_OS_PORT_RESTART_H
#define AUTOSAR_OS_PORT_RESTART_H
#include "Os_Version.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/* Enter once on the registered native Task thread. Each restart discards the
 * outgoing C frames and invokes entry again on that same physical stack. */
OS_MEMMAP void Os_PortTaskLoop(void (*entry)(void *), void *argument) __attribute__((noreturn));

/* Called only after successful automotive activation settlement and rescheduling.
 * Invalid/unhandled native unwinding stops the OS; it cannot return success. */
OS_MEMMAP void Os_PortRestart(void) __attribute__((noreturn));
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
