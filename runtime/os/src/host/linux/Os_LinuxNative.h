/** @file Private typed Linux kernel and native execution-stack boundary. */
#ifndef AUTOSAR_OS_LINUX_NATIVE_H
#define AUTOSAR_OS_LINUX_NATIVE_H
#include "Os_Host.h"
#include <asm-generic/signal-defs.h>

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_LinuxFutexWait(const volatile int32_t *cell,
                                int32_t expected,
                                const struct timespec *interval);
/** Wake count is not an error; a negative kernel result stops the native process. */
OS_MEMMAP void Os_LinuxFutexWake(const volatile int32_t *cell);
OS_MEMMAP DWORD Os_LinuxThreadId(void);
OS_MEMMAP int Os_LinuxAltStack(const stack_t *stack, stack_t *previous);
/** Project the fixed x64 signal control prefix and legacy FP into owned storage; supports
 * self-copy. */
OS_MEMMAP void Os_LinuxCaptureContext(CONTEXT *destination, const void *source);
OS_MEMMAP uintptr_t Os_LinuxSignalSp(const void *context);
OS_MEMMAP int Os_LinuxMappedStack(uintptr_t *low, uintptr_t *high, size_t *guard);
OS_MEMMAP DWORD Os_LinuxCallOnStack(void *top, LPTHREAD_START_ROUTINE entry, void *argument);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_LinuxNative.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_LinuxNative.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
