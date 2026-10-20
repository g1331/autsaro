/** @file Host primitives (atomics, threads, events, exit) over Win32 or the Linux shim. */
#include "Os_Host.h"
#include <stdio.h>
#if defined(_WIN32)
#include <fcntl.h>
#include <io.h>
#endif

Os_Atomic32Type Os_HostAtomicLoad(volatile Os_Atomic32Type *cell)
{
    return InterlockedCompareExchange(cell, 0, 0);
}
Os_Atomic32Type Os_HostAtomicExchange(volatile Os_Atomic32Type *cell, Os_Atomic32Type value)
{
    return InterlockedExchange(cell, value);
}
Os_Atomic32Type Os_HostAtomicCompareExchange(volatile Os_Atomic32Type *cell,
                                             Os_Atomic32Type desired,
                                             Os_Atomic32Type expected)
{
    return InterlockedCompareExchange(cell, desired, expected);
}
Os_HostThreadIdType Os_HostThreadIdentity(void) { return GetCurrentThreadId(); }
uint64_t Os_HostMonotonicMs(void) { return GetTickCount64(); }
void Os_HostSleepMs(uint32_t milliseconds) { Sleep(milliseconds); }
Os_HostHandleType Os_HostOpenManualEvent(void) { return CreateEventA(NULL, TRUE, FALSE, NULL); }
Os_HostHandleType Os_HostSpawnThread(Os_HostThreadEntryType entry, void *argument)
{
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
}
Os_HostHandleType Os_HostSpawnObserverThread(Os_HostThreadEntryType entry, void *argument)
{
#if defined(__linux__)
    return Os_HostCreateActor(262144u, entry, argument, 0u, NULL, 0);
#else
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
#endif
}
int Os_HostWait(Os_HostHandleType handle, uint32_t milliseconds)
{
    return WaitForSingleObject(handle, milliseconds) == WAIT_OBJECT_0;
}
int Os_HostClose(Os_HostHandleType handle) { return CloseHandle(handle) != 0; }
int Os_HostSetBinaryStandardStreams(void)
{
#if defined(_WIN32)
    const int input = _setmode(_fileno(stdin), _O_BINARY);
    const int output = _setmode(_fileno(stdout), _O_BINARY);
    return (input != -1) && (output != -1);
#else
    /* POSIX streams already preserve every byte, including CR and NUL. */
    return 1;
#endif
}
void Os_HostExit(uint32_t status) { ExitProcess(status); }
