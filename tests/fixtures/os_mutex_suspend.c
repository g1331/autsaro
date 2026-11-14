/** @file Suspend and stop around raw mutex waits without losing requests or deadlines. */
#include "Os.h"
#include "Os_Host.h"
#include "Os_LinuxNative.h"
#include "Os_Stack.h"
#include <asm/unistd.h>
#include <errno.h>
#include <poll.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

enum { BEFORE, AFTER, DURING, ELAPSED, SIGNAL_ERROR, STOP, STOP_AFTER, STOP_HELD };
static int mode;
static HANDLE mutex;
static HANDLE held_mutex;
static HANDLE start_event;
static HANDLE finish_event;
static unsigned registered_entry;
static unsigned request_started;
static unsigned armed;
static unsigned enter_native;
static unsigned self_signal;
static unsigned completed;
static unsigned event_waiting;
static unsigned stop_ack;
static unsigned offset_seconds;
static DWORD actor_id;
static const volatile int32_t *wait_address;
static int first_result;
static OS_HOST_THREAD_LOCAL int waiting;
static OS_HOST_THREAD_LOCAL unsigned native_calls;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }
int __real_Os_LinuxFutexWait(const volatile int32_t *address,
                             int32_t expected,
                             const struct timespec *duration);
void __real_Os_LinuxFutexWake(const volatile int32_t *address);
int __real_pthread_kill(pthread_t thread, int signal_number);
int __real_clock_gettime(clockid_t clock, struct timespec *value);
int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
int __real___poll_chk(struct pollfd *fds, nfds_t count, int timeout, size_t bytes);

static void require(int accepted, int stage)
{
    if (accepted == 0) {
        fprintf(stderr, "mutex_suspend_rejected stage=%d\n", stage);
        _exit(stage);
    }
}

/* These observers must not consume queued TSan signals at an inserted safe point. */
__attribute__((no_sanitize_thread)) void fixture_before_mutex_registration(
    volatile int32_t *address)
{
    if (waiting == 1) {
        __atomic_store_n(&wait_address, address, __ATOMIC_RELEASE);
        if ((mode == BEFORE) || (mode == ELAPSED) || (mode == SIGNAL_ERROR) || (mode == STOP) ||
            (mode == STOP_HELD)) {
            __atomic_store_n(&registered_entry, 1u, __ATOMIC_RELEASE);
            while (__atomic_load_n(&request_started, __ATOMIC_ACQUIRE) == 0u) {
                /* The sender publishes the request before this subscription. */
            }
        }
    }
}

__attribute__((no_sanitize_thread)) int __wrap_Os_LinuxFutexWait(const volatile int32_t *address,
                                                                 int32_t expected,
                                                                 const struct timespec *duration)
{
    const int target_wait = waiting == 1 && (duration == NULL || duration->tv_sec > 0);
    unsigned call = 0u;
    if (target_wait != 0) {
        call = ++native_calls;
        if (mode == ELAPSED && __atomic_load_n(&offset_seconds, __ATOMIC_ACQUIRE) != 0u) {
            require(0, 64);
        }
        if (__atomic_load_n(&armed, __ATOMIC_ACQUIRE) == 0u &&
            __atomic_load_n(&self_signal, __ATOMIC_ACQUIRE) == 0u) {
            __atomic_store_n(&armed, 1u, __ATOMIC_RELEASE);
            if (mode != DURING) {
                while (__atomic_load_n(&enter_native, __ATOMIC_ACQUIRE) == 0u) {
                    /* Signal delivery precedes the actual raw syscall. */
                }
            }
        }
    }
    const int result = __real_Os_LinuxFutexWait(address, expected, duration);
    if (call == 1u) {
        __atomic_store_n(&first_result, result, __ATOMIC_RELEASE);
    }
    return result;
}

__attribute__((no_sanitize_thread)) void __wrap_Os_LinuxFutexWake(const volatile int32_t *address)
{
    __real_Os_LinuxFutexWake(address);
    if (mode == STOP_HELD && waiting == 1 &&
        __atomic_load_n(address, __ATOMIC_ACQUIRE) == INT32_MAX) {
        __atomic_store_n(&stop_ack, 1u, __ATOMIC_RELEASE);
    }
}

__attribute__((no_sanitize_thread)) int __wrap_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    if (waiting == 2) {
        __atomic_store_n(&event_waiting, 1u, __ATOMIC_RELEASE);
    }
    return __real_poll(fds, count, timeout);
}

__attribute__((no_sanitize_thread)) int __wrap___poll_chk(struct pollfd *fds,
                                                          nfds_t count,
                                                          int timeout,
                                                          size_t bytes)
{
    if (waiting == 2) {
        __atomic_store_n(&event_waiting, 1u, __ATOMIC_RELEASE);
    }
    return __real___poll_chk(fds, count, timeout, bytes);
}

static int signal_pending(int signal_number)
{
    char path[128];
    require(snprintf(path,
                     sizeof(path),
                     "/proc/self/task/%lu/status",
                     (unsigned long)__atomic_load_n(&actor_id, __ATOMIC_ACQUIRE)) > 0,
            20);
    FILE *file = fopen(path, "r");
    require(file != NULL, 21);
    char line[256];
    unsigned long long bits = ~0ULL;
    while (fgets(line, sizeof(line), file) != NULL) {
        if (sscanf(line, "SigPnd: %llx", &bits) == 1) {
            break;
        }
    }
    require(fclose(file) == 0, 22);
    return (bits & (1ULL << (signal_number - 1))) != 0;
}

int __wrap_pthread_kill(pthread_t thread, int signal_number)
{
    if (pthread_equal(thread, pthread_self()) != 0) {
        if (mode == STOP_HELD) {
            require(signal_number == SIGRTMIN + 1, 68);
        }
        __atomic_store_n(&self_signal, 1u, __ATOMIC_RELEASE);
        return mode == SIGNAL_ERROR ? EAGAIN : __real_pthread_kill(thread, signal_number);
    }
    static unsigned calls;
    if (++calls == 1u) {
        if (mode == BEFORE || mode == SIGNAL_ERROR || mode == STOP || mode == STOP_HELD) {
            __atomic_store_n(&request_started, 1u, __ATOMIC_RELEASE);
            const uint64_t started = GetTickCount64();
            while (__atomic_load_n(&self_signal, __ATOMIC_ACQUIRE) == 0u &&
                   __atomic_load_n(&armed, __ATOMIC_ACQUIRE) == 0u) {
                require(GetTickCount64() - started < 2000u, 23);
                (void)sched_yield();
            }
        }
        if (mode == STOP_HELD) {
            const uint64_t started = GetTickCount64();
            while (__atomic_load_n(&stop_ack, __ATOMIC_ACQUIRE) == 0u) {
                require(GetTickCount64() - started < 2000u, 23);
                (void)sched_yield();
            }
        }
        const int result = __real_pthread_kill(thread, signal_number);
        require(result == 0, 24);
        if (__atomic_load_n(&self_signal, __ATOMIC_ACQUIRE) == 0u) {
            const uint64_t started = GetTickCount64();
            while (signal_pending(signal_number) != 0) {
                require(GetTickCount64() - started < 2000u, 25);
            }
        }
        __atomic_store_n(&request_started, 1u, __ATOMIC_RELEASE);
        __atomic_store_n(&enter_native, 1u, __ATOMIC_RELEASE);
        return result;
    }
    return __real_pthread_kill(thread, signal_number);
}

int __wrap_clock_gettime(clockid_t clock, struct timespec *value)
{
    const int result = __real_clock_gettime(clock, value);
    if (result == 0 && clock == CLOCK_MONOTONIC) {
        value->tv_sec += (time_t)__atomic_load_n(&offset_seconds, __ATOMIC_ACQUIRE);
    }
    return result;
}

static int sleeping_on_mutex(void)
{
    char path[128];
    require(snprintf(path,
                     sizeof(path),
                     "/proc/self/task/%lu/syscall",
                     (unsigned long)__atomic_load_n(&actor_id, __ATOMIC_ACQUIRE)) > 0,
            26);
    FILE *file = fopen(path, "r");
    require(file != NULL, 27);
    long number;
    unsigned long address;
    const int fields = fscanf(file, "%ld %lx", &number, &address);
    require(fclose(file) == 0, 28);
    return fields == 2 && number == __NR_futex &&
           address == (uintptr_t)__atomic_load_n(&wait_address, __ATOMIC_ACQUIRE);
}

static void await_flag(unsigned *flag)
{
    const uint64_t started = GetTickCount64();
    while (__atomic_load_n(flag, __ATOMIC_ACQUIRE) == 0u) {
        require(GetTickCount64() - started < 5000u, 29);
        (void)sched_yield();
    }
}

static DWORD actor(void *argument)
{
    (void)argument;
    __atomic_store_n(&actor_id, GetCurrentThreadId(), __ATOMIC_RELEASE);
    require(WaitForSingleObject(start_event, INFINITE) == WAIT_OBJECT_0, 30);
    if (mode == STOP_HELD) {
        require(WaitForSingleObject(held_mutex, 0u) == WAIT_OBJECT_0, 66);
    }
    waiting = 1;
    const DWORD result =
        WaitForSingleObject(mutex, mode == ELAPSED || mode == SIGNAL_ERROR ? 2000u : INFINITE);
    require(result == (mode == ELAPSED        ? WAIT_TIMEOUT
                       : mode == SIGNAL_ERROR ? WAIT_FAILED
                                              : WAIT_OBJECT_0),
            62);
    if (result == WAIT_OBJECT_0) {
        require(ReleaseMutex(mutex) != 0, 31);
    }
    waiting = 2;
    __atomic_store_n(&completed, 1u, __ATOMIC_RELEASE);
    require(WaitForSingleObject(finish_event, INFINITE) == WAIT_OBJECT_0, 32);
    return 37u;
}

/* A cached generation address remains valid after the static slot changes kind. */
static void reuse_mutex_cell(void)
{
    HANDLE previous = mutex;
    require(CloseHandle(mutex) != 0, 56);
    mutex = CreateEventA(NULL, 1, 0, NULL);
    require(mutex == previous, 57);
    require(WaitForSingleObject(mutex, 0u) == WAIT_TIMEOUT, 58);
}

static void exercise_stop(HANDLE thread)
{
    require(Os_HostStopActor(thread, 2000u) != 0, 52);
    require(ReleaseMutex(mutex) != 0, 34);
    reuse_mutex_cell();
    require(Os_HostStopActor(thread, 2000u) != 0, 53);
    require(__atomic_load_n(&completed, __ATOMIC_ACQUIRE) == 0u, 60);
}

static void exercise_suspend(HANDLE thread)
{
    require(SuspendThread(thread) == 0u, 52);
    if (mode == ELAPSED) {
        __atomic_store_n(&offset_seconds, 5u, __ATOMIC_RELEASE);
    }
    require(ResumeThread(thread) == 1u, 53);
    if (mode != ELAPSED && mode != SIGNAL_ERROR) {
        require(ReleaseMutex(mutex) != 0, 35);
    }
    await_flag(&completed);
    if (mode == ELAPSED || mode == SIGNAL_ERROR) {
        require(ReleaseMutex(mutex) != 0, 36);
    }
    await_flag(&event_waiting);
    reuse_mutex_cell();
    require(SuspendThread(thread) == 0u, 54);
    require(ResumeThread(thread) == 1u, 55);
    require(Os_HostSetEvent(finish_event) != 0, 37);
    require(WaitForSingleObject(thread, 2000u) == WAIT_OBJECT_0, 38);
    if (mode == DURING) {
        require(__atomic_load_n(&first_result, __ATOMIC_ACQUIRE) == -EINTR, 59);
    }
}

static void exercise(HANDLE thread)
{
    if (mode == BEFORE || mode == ELAPSED || mode == SIGNAL_ERROR || mode == STOP ||
        mode == STOP_HELD) {
        await_flag(&registered_entry);
    } else {
        await_flag(&armed);
    }
    if (mode == DURING) {
        const uint64_t started = GetTickCount64();
        while (sleeping_on_mutex() == 0) {
            require(GetTickCount64() - started < 2000u, 33);
            (void)sched_yield();
        }
    }
    if (mode == STOP || mode == STOP_AFTER || mode == STOP_HELD) {
        exercise_stop(thread);
    } else {
        exercise_suspend(thread);
    }
    require(WaitForSingleObject(mutex, 0u) == WAIT_TIMEOUT, 61);
    require(Os_HostSetEvent(mutex) != 0, 63);
    require(WaitForSingleObject(mutex, 0u) == WAIT_OBJECT_0, 65);
}

int main(int argc, char **argv)
{
    require(argc == 2, 40);
    mode = strcmp(argv[1], "before-register") == 0    ? BEFORE
           : strcmp(argv[1], "before-wait") == 0      ? AFTER
           : strcmp(argv[1], "during-wait") == 0      ? DURING
           : strcmp(argv[1], "elapsed") == 0          ? ELAPSED
           : strcmp(argv[1], "signal-error") == 0     ? SIGNAL_ERROR
           : strcmp(argv[1], "stop") == 0             ? STOP
           : strcmp(argv[1], "stop-before-wait") == 0 ? STOP_AFTER
           : strcmp(argv[1], "stop-held") == 0        ? STOP_HELD
                                                      : -1;
    require(mode >= BEFORE && mode <= STOP_HELD, 46);
    Os_StackPrepare();
    mutex = CreateMutexA(NULL, 0, NULL);
    held_mutex = CreateMutexA(NULL, 0, NULL);
    start_event = CreateEventA(NULL, 1, 0, NULL);
    finish_event = CreateEventA(NULL, 1, 0, NULL);
    require(mutex != NULL && held_mutex != NULL && start_event != NULL && finish_event != NULL, 41);
    HANDLE thread = Os_HostCreateActor(262144u, actor, NULL, 0u, NULL, 1);
    require(thread != NULL, 42);
    require(WaitForSingleObject(mutex, 0u) == WAIT_OBJECT_0, 43);
    require(Os_HostSetEvent(start_event) != 0, 44);
    exercise(thread);
    require(CloseHandle(thread) != 0 && CloseHandle(mutex) != 0 && CloseHandle(start_event) != 0 &&
                CloseHandle(finish_event) != 0,
            45);
    if (mode != STOP_HELD) {
        require(CloseHandle(held_mutex) != 0, 67);
    }
    printf("mutex_suspend scenario=%s requests=observed cache=reused\n", argv[1]);
    return 0;
}
