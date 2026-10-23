/** @file ControlIdle modes and wakeup of a waiting Task by an ISR from the idle loop. */
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#ifdef __linux__
#include <time.h>
#endif

static const uint8 task_name_0[] = "owner"; /**< Process-lifetime Task name bytes. */

static const char *scenario; /**< Main-owned scenario selection before StartOS; no Init reset. */
static volatile LONG
    requested; /**< Atomic native/OS handoff requested; fixture-owned, no Init reset. */
static volatile LONG
    observed;           /**< Atomic native/OS handoff observed; fixture-owned, no Init reset. */
static unsigned wakes;  /**< Wakes observations; fixture-owned, no Init reset. */
static unsigned errors; /**< Observed checks/errors; fixture-owned, no Init reset. */
static StatusType error_status[5]; /**< Observed ErrorHook results; fixture-owned, no Init reset. */
static OSServiceIdType
    error_service[5]; /**< Observed ErrorHook services; fixture-owned, no Init reset. */
static IdleModeType
    rejected_mode; /**< Rejected idle-mode observation; fixture-owned, no Init reset. */
static uint64_t idle_started_ms; /**< Host observer idle started ms measurement; no Init reset. */
static uint64_t idle_cpu_ms;     /**< Host observer idle cpu ms measurement; no Init reset. */
static uint64_t idle_window_ms;  /**< Host observer idle window ms measurement; no Init reset. */
static void check(bool condition)
{
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
void ErrorHook(StatusType error)
{
    const OSServiceIdType service = OSErrorGetServiceId();
    check(errors < 5u && isOsStarted() == TRUE);
    if (service == 29u) {
        check(OSError_ControlIdle_CoreID() == UINT16_MAX);
        check(OSError_ControlIdle_IdleMode() == rejected_mode);
    } else {
        check(service == 54u && error == 9u);
    }
    error_status[errors] = error;
    error_service[errors++] = service;
}
/* Host CPU time consumed by the calling thread. */
static uint64_t thread_cpu_ms(void)
{
#ifdef _WIN32
    FILETIME created, exited, kernel, user;
    check(GetThreadTimes(GetCurrentThread(), &created, &exited, &kernel, &user) != 0);
    return ((((uint64_t)kernel.dwHighDateTime << 32) | kernel.dwLowDateTime) +
            (((uint64_t)user.dwHighDateTime << 32) | user.dwLowDateTime)) /
           10000u;
#else
    struct timespec now;
    check(clock_gettime(CLOCK_THREAD_CPUTIME_ID, &now) == 0);
    return ((uint64_t)now.tv_sec * 1000u) + ((uint64_t)now.tv_nsec / 1000000u);
#endif
}
void Os_TestIdleObserved(void)
{
    const bool yield = strcmp(scenario, "host-yield") == 0;
    if (InterlockedCompareExchange(&requested, 0, 0) == 0) {
        return;
    }
    if (InterlockedCompareExchange(&observed, 1, 0) == 0) {
        check(Os_StackCurrent()->role == 'I' && isOsStarted() == TRUE);
        if (!yield) {
            Os_PortPostInterrupt(5u);
        } else {
            idle_started_ms = GetTickCount64();
            idle_cpu_ms = thread_cpu_ms();
        }
    } else if (yield && idle_window_ms == 0u && (GetTickCount64() - idle_started_ms) >= 400u) {
        /* The Task waits throughout, so the window measures the idle loop alone. */
        idle_window_ms = GetTickCount64() - idle_started_ms;
        idle_cpu_ms = thread_cpu_ms() - idle_cpu_ms;
        Os_PortPostInterrupt(5u);
    }
}
static uint32_t interrupt(void)
{
    TaskType task = 42u;
    TaskStateType state = SUSPENDED;
    check(GetISRID() == 5u && isOsStarted() == TRUE);
    check(GetTaskID(&task) == E_OK && task == INVALID_TASK);
    check(GetTaskState(0u, &state) == E_OK && state == WAITING);
    check(InterlockedCompareExchange(&observed, 0, 0) == 1);
    if (strcmp(scenario, "isr") == 0) {
        check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OK);
    }
    check(SetEvent(0u, 1u) == E_OK);
    ++wakes;
    return 0u;
}
void StartupHook(void)
{
    check(isOsStarted() == TRUE);
    rejected_mode = 0u;
    check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OS_CALLEVEL);
    vPortSetInterruptHandler(5u, &interrupt);
}
void ShutdownHook(StatusType error)
{
    check(isOsStarted() == TRUE);
    printf("idle_state scenario=%s observed=%d wakes=%u errors=%u reason=%u\n",
           scenario,
           (int)InterlockedCompareExchange(&observed, 0, 0),
           wakes,
           errors,
           error);
    for (unsigned i = 0u; i < errors; ++i) {
        printf("idle_error status=%u service=%u\n", error_status[i], error_service[i]);
    }
    if (strcmp(scenario, "host-yield") == 0) {
        printf("idle_thread cpu_ms=%llu window_ms=%llu\n",
               (unsigned long long)idle_cpu_ms,
               (unsigned long long)idle_window_ms);
    }
    if (fflush(stdout) != 0) {
        ExitProcess(99u);
    }
}
static void owner(void)
{
    check(isOsStarted() == TRUE);
    if (strcmp(scenario, "invalid") == 0 || strcmp(scenario, "unconfigured") == 0) {
        rejected_mode = 99u;
        check(ControlIdle(UINT16_MAX, 99u) == E_OS_ID);
    } else if (strcmp(scenario, "masked") == 0) {
        DisableAllInterrupts();
        rejected_mode = 0u;
        check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == 9u);
        check(isOsStarted() == FALSE);
        EnableAllInterrupts();
        check(isOsStarted() == TRUE);
    }
    check(ControlIdle(UINT16_MAX, IDLE_NO_HALT) == E_OK);
    check(ControlIdle(0u, IDLE_NO_HALT) == E_OK);
    InterlockedExchange(&requested, 1);
    check(WaitEvent(1u) == E_OK && wakes == 1u);
    check(ClearEvent(1u) == E_OK);
    ShutdownOS(E_OK);
}
int main(int argc, char **argv)
{
    check(argc == 2);
    scenario = argv[1];
    check(isOsStarted() == FALSE);
    const Os_TaskConfigType task = {
        0u, task_name_0, owner, 10u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u};
    const Os_HookConfigType hooks = {&ErrorHook, NULL, NULL};
    const Os_TargetConfigType target = {&task,
                                        1u,
                                        262144u,
                                        NULL,
                                        0u,
                                        NULL,
                                        0u,
                                        0u,
                                        0u,
                                        0u,
                                        NULL,
                                        strcmp(scenario, "unconfigured") == 0 ? NULL : &hooks,
                                        NULL,
                                        0u};
    check(Os_TargetPrepare(&target) == E_OK && isOsStarted() == FALSE);
    StartOS(1u);
    return 99;
}
