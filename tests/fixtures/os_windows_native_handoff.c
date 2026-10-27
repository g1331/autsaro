/** @file Operation-specific failures in the real Windows kernel port. */
#include "Os_Host.h"
#include <stdio.h>
#include <string.h>

const char *Os_HandoffScenario;
static __thread int failed;
static __thread int supplied_timeout;

static int refuses(const char *operation)
{
    if (failed != 0) {
        printf("native handoff continued after failure\n");
        (void)fflush(stdout);
        ExitProcess(91u);
    }
    if (strcmp(Os_HandoffScenario, operation) == 0) {
        printf("native handoff refused=%s\n", operation);
        (void)fflush(stdout);
        failed = 1;
        SetLastError(ERROR_INVALID_HANDLE);
        return 1;
    }
    return 0;
}

DWORD WINAPI Os_TestWaitForSingleObject(HANDLE object, DWORD milliseconds)
{
    return refuses("wait") != 0 ? WAIT_FAILED : WaitForSingleObject(object, milliseconds);
}

DWORD WINAPI Os_TestWaitForMultipleObjects(DWORD count,
                                           const HANDLE *objects,
                                           BOOL all,
                                           DWORD milliseconds)
{
    if (refuses("wait-multiple") != 0) {
        return WAIT_FAILED;
    }
    if (refuses("abandoned") != 0) {
        return WAIT_ABANDONED_0;
    }
    if ((strcmp(Os_HandoffScenario, "timeout") == 0) && (supplied_timeout == 0)) {
        supplied_timeout = 1;
        puts("native handoff timeout");
        (void)fflush(stdout);
        return WAIT_TIMEOUT;
    }
    return WaitForMultipleObjects(count, objects, all, milliseconds);
}

BOOL WINAPI Os_TestReleaseMutex(HANDLE mutex)
{
    return refuses("release") != 0 ? 0 : ReleaseMutex(mutex);
}

BOOL Os_TestSignalEvent(HANDLE event)
{
    return refuses("signal") != 0 ? 0 : Os_HostSetEvent(event);
}

BOOL WINAPI Os_TestResetEvent(HANDLE event)
{
    return refuses("reset") != 0 ? 0 : ResetEvent(event);
}
