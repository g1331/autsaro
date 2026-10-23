/** @file Host primitives (atomics, threads, events, exit) over Win32 or the Linux shim. */
#include "Os_Host.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP Os_Atomic32Type Os_HostAtomicLoad(volatile Os_Atomic32Type *cell)
{
    return InterlockedCompareExchange(cell, 0, 0);
}

OS_MEMMAP Os_Atomic32Type Os_HostAtomicExchange(volatile Os_Atomic32Type *cell,
                                                Os_Atomic32Type value)
{
    return InterlockedExchange(cell, value);
}

OS_MEMMAP Os_Atomic32Type Os_HostAtomicCompareExchange(volatile Os_Atomic32Type *cell,
                                                       Os_Atomic32Type desired,
                                                       Os_Atomic32Type expected)
{
    return InterlockedCompareExchange(cell, desired, expected);
}

OS_MEMMAP Os_HostThreadIdType Os_HostThreadIdentity(void) { return GetCurrentThreadId(); }

OS_MEMMAP uint64_t Os_HostMonotonicMs(void) { return GetTickCount64(); }

OS_MEMMAP void Os_HostSleepMs(uint32_t milliseconds) { Sleep(milliseconds); }

OS_MEMMAP Os_HostHandleType Os_HostOpenManualEvent(void)
{
    return CreateEventA(NULL, TRUE, FALSE, NULL);
}

OS_MEMMAP Os_HostHandleType Os_HostSpawnThread(Os_HostThreadEntryType entry, void *argument)
{
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostServices.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostServices.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#if defined(__linux__)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP Os_HostHandleType Os_HostSpawnObserverThread(Os_HostThreadEntryType entry, void *argument)
{
    return Os_HostCreateActor(262144u, entry, argument, 0u, NULL, 0);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP Os_HostHandleType Os_HostSpawnObserverThread(Os_HostThreadEntryType entry, void *argument)
{
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_HostWait(Os_HostHandleType handle, uint32_t milliseconds)
{
    return WaitForSingleObject(handle, milliseconds) == WAIT_OBJECT_0;
}

OS_MEMMAP int Os_HostClose(Os_HostHandleType handle) { return CloseHandle(handle) != 0; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_HostExit(uint32_t status) { ExitProcess(status); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
