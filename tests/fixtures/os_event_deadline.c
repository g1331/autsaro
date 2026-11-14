/** @file Pause past a finite wait deadline immediately before its native blocking call. */
#include "Os.h"
#include "Os_Host.h"
#include "Os_LinuxNative.h"
#include "Os_Stack.h"
#include <poll.h>
#include <sched.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

enum { BUDGET_MS = 200, PAUSE_MS = 300, RESUME_BOUND_MS = 50 };
static HANDLE events[2];
static HANDLE gate;
static HANDLE mutex;
static unsigned armed;
static unsigned proceed;
static unsigned mode;
static DWORD wait_result;
static uint64_t finished_at;
static OS_HOST_THREAD_LOCAL unsigned waiting;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }
int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
int __real_Os_LinuxFutexWait(const volatile int32_t *address,
                             int32_t expected,
                             const struct timespec *interval);

static void require(int accepted, int stage)
{
    if (accepted == 0) {
        fprintf(stderr, "deadline_rejected stage=%d\n", stage);
        _exit(stage);
    }
}

static void before_native_wait(void)
{
    if (waiting != 0u && __atomic_exchange_n(&armed, 1u, __ATOMIC_SEQ_CST) == 0u) {
        while (__atomic_load_n(&proceed, __ATOMIC_ACQUIRE) == 0u) {
            (void)sched_yield();
        }
    }
}

int __wrap_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    before_native_wait();
    return __real_poll(fds, count, timeout);
}

int __wrap_Os_LinuxFutexWait(const volatile int32_t *address,
                             int32_t expected,
                             const struct timespec *interval)
{
    before_native_wait();
    return __real_Os_LinuxFutexWait(address, expected, interval);
}

static DWORD actor(void *argument)
{
    (void)argument;
    require(WaitForSingleObject(gate, INFINITE) == WAIT_OBJECT_0, 21);
    waiting = 1u;
    wait_result = mode == 2u   ? WaitForSingleObject(mutex, BUDGET_MS)
                  : mode == 1u ? WaitForMultipleObjects(2u, events, FALSE, BUDGET_MS)
                               : WaitForSingleObject(events[0], BUDGET_MS);
    finished_at = GetTickCount64();
    return 0u;
}

int main(int argc, char **argv)
{
    require(argc == 2, 22);
    mode = strcmp(argv[1], "single") == 0 ? 0u : strcmp(argv[1], "any") == 0 ? 1u : 2u;
    Os_StackPrepare();
    events[0] = CreateEventA(NULL, TRUE, FALSE, NULL);
    events[1] = CreateEventA(NULL, TRUE, FALSE, NULL);
    gate = CreateEventA(NULL, TRUE, FALSE, NULL);
    mutex = CreateMutexA(NULL, FALSE, NULL);
    require(events[0] != NULL && events[1] != NULL && gate != NULL && mutex != NULL, 23);
    HANDLE thread = Os_HostCreateActor(262144u, actor, NULL, 0u, NULL, 0);
    require(thread != NULL, 24);
    if (mode == 2u) {
        require(WaitForSingleObject(mutex, 0u) == WAIT_OBJECT_0, 25);
    }
    require(Os_HostSetEvent(gate) != 0, 26);
    const uint64_t started = GetTickCount64();
    while (__atomic_load_n(&armed, __ATOMIC_ACQUIRE) == 0u) {
        require(GetTickCount64() - started < 5000u, 27);
        (void)sched_yield();
    }
    require(SuspendThread(thread) == 0u, 28);
    const uint64_t paused_at = GetTickCount64();
    const struct timespec pause = {.tv_sec = 0, .tv_nsec = PAUSE_MS * 1000000L};
    require(nanosleep(&pause, NULL) == 0, 29);
    const uint64_t resumed_at = GetTickCount64();
    __atomic_store_n(&proceed, 1u, __ATOMIC_RELEASE);
    require(ResumeThread(thread) == 1u, 30);
    require(WaitForSingleObject(thread, 2000u) == WAIT_OBJECT_0, 31);
    const uint64_t after = finished_at - resumed_at;
    printf("deadline mode=%s result=%lu after_resume_ms=%llu paused_ms=%llu budget_ms=%u\n",
           argv[1],
           (unsigned long)wait_result,
           (unsigned long long)after,
           (unsigned long long)(resumed_at - paused_at),
           BUDGET_MS);
    require(fflush(stdout) == 0, 34);
    require(wait_result == WAIT_TIMEOUT, 53);
    require(after <= RESUME_BOUND_MS, 54);
    if (mode == 2u) {
        require(ReleaseMutex(mutex) != 0, 32);
    }
    require(CloseHandle(thread) != 0 && CloseHandle(mutex) != 0 && CloseHandle(gate) != 0 &&
                CloseHandle(events[0]) != 0 && CloseHandle(events[1]) != 0,
            33);
    return 0;
}
