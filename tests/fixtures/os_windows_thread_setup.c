/** @file Actual Windows port setup with operation-specific refusal controls. */
#include "Os_Host.h"
#include <stdio.h>
#include <string.h>

const char *Os_ThreadSetupScenario;

static int refuses(HANDLE thread, const char *operation)
{
    const char *role = thread == GetCurrentThread() ? "dispatcher" : "task";
    char selected[64];
    const int length = snprintf(selected, sizeof(selected), "%s-%s", role, operation);
    if ((length <= 0) || ((size_t)length >= sizeof(selected))) {
        return 0;
    }
    if (strcmp(Os_ThreadSetupScenario, selected) == 0) {
        printf("native setup refused=%s\n", selected);
        (void)fflush(stdout);
        SetLastError(ERROR_ACCESS_DENIED);
        return 1;
    }
    return 0;
}

DWORD_PTR WINAPI Os_TestSetThreadAffinityMask(HANDLE thread, DWORD_PTR mask)
{
    return refuses(thread, "affinity") != 0 ? 0u : SetThreadAffinityMask(thread, mask);
}

BOOL WINAPI Os_TestSetThreadPriorityBoost(HANDLE thread, BOOL disabled)
{
    return refuses(thread, "boost") != 0 ? 0 : SetThreadPriorityBoost(thread, disabled);
}

BOOL WINAPI Os_TestSetThreadPriority(HANDLE thread, int priority)
{
    return refuses(thread, "priority") != 0 ? 0 : SetThreadPriority(thread, priority);
}
