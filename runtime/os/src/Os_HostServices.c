#include "Os_Host.h"
#include <stdio.h>
#if defined(_WIN32)
#include <fcntl.h>
#include <io.h>
#endif

Os_Atomic32 Os_HostAtomicLoad(volatile Os_Atomic32 *cell) {
    return InterlockedCompareExchange(cell, 0, 0);
}
Os_Atomic32 Os_HostAtomicExchange(volatile Os_Atomic32 *cell, Os_Atomic32 value) {
    return InterlockedExchange(cell, value);
}
Os_Atomic32 Os_HostAtomicCompareExchange(volatile Os_Atomic32 *cell, Os_Atomic32 desired,
                                         Os_Atomic32 expected) {
    return InterlockedCompareExchange(cell, desired, expected);
}
Os_HostThreadId Os_HostThreadIdentity(void) { return GetCurrentThreadId(); }
uint64_t Os_HostMonotonicMs(void) { return GetTickCount64(); }
void Os_HostSleepMs(uint32_t milliseconds) { Sleep(milliseconds); }
Os_HostHandle Os_HostOpenManualEvent(void) { return CreateEventA(NULL, TRUE, FALSE, NULL); }
Os_HostHandle Os_HostSpawnThread(Os_HostThreadEntry entry, void *argument) {
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
}
Os_HostHandle Os_HostSpawnObserverThread(Os_HostThreadEntry entry, void *argument) {
#if defined(__linux__)
    return Os_HostCreateActor(262144u, entry, argument, 0u, NULL, 0);
#else
    return CreateThread(NULL, 262144u, entry, argument, 0u, NULL);
#endif
}
int Os_HostWait(Os_HostHandle handle, uint32_t milliseconds) {
    return WaitForSingleObject(handle, milliseconds) == WAIT_OBJECT_0;
}
int Os_HostClose(Os_HostHandle handle) { return CloseHandle(handle) != 0; }
int Os_HostSetBinaryStandardStreams(void) {
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
