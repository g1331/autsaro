/** @file Deliver suspend before entering a fortified native event wait. */
#include "Os.h"
#include "Os_Host.h"
#include "Os_Stack.h"
#include <poll.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static HANDLE event;
static unsigned armed;
static unsigned proceed;
static DWORD actor_id;
static OS_HOST_THREAD_LOCAL int waiting;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }
static void require(int accepted, int code)
{
    if (!accepted) {
        fprintf(stderr, "event_suspend_rejected stage=%d\n", code);
        _exit(code);
    }
}
int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
int __real___poll_chk(struct pollfd *fds, nfds_t count, int timeout, size_t bytes);
int __real_pthread_kill(pthread_t thread, int signal_number);

/* The observer must not drain TSan's queued signal before the real wait entry.
 * Keep only this handshake and its forwarding calls free of inserted safe points. */
__attribute__((no_sanitize_thread)) static void arm(void)
{
    if (waiting && __atomic_load_n(&armed, __ATOMIC_ACQUIRE) == 0) {
        __atomic_store_n(&armed, 1, __ATOMIC_RELEASE);
        while (__atomic_load_n(&proceed, __ATOMIC_ACQUIRE) == 0) {
            /* The sender observes kernel delivery before releasing this entry. */
        }
    }
}
__attribute__((no_sanitize_thread)) int __wrap_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    arm();
    return __real_poll(fds, count, timeout);
}
__attribute__((no_sanitize_thread)) int __wrap___poll_chk(struct pollfd *fds,
                                                          nfds_t count,
                                                          int timeout,
                                                          size_t bytes)
{
    arm();
    return __real___poll_chk(fds, count, timeout, bytes);
}
/* pthread_kill has queued the signal; a clear kernel pending bit witnesses its
 * delivery even when TSan has deferred the user handler until a blocking entry. */
static int pending(void)
{
    char name[128], line[256];
    require(snprintf(name,
                     sizeof(name),
                     "/proc/self/task/%lu/status",
                     (unsigned long)__atomic_load_n(&actor_id, __ATOMIC_ACQUIRE)) > 0,
            20);
    FILE *file = fopen(name, "r");
    require(file != NULL, 21);
    unsigned long long bits = ~0ULL;
    while (fgets(line, sizeof(line), file) != NULL) {
        if (sscanf(line, "SigPnd: %llx", &bits) == 1) {
            break;
        }
    }
    require(fclose(file) == 0, 22);
    return (bits & (1ULL << (SIGRTMIN - 1))) != 0;
}
int __wrap_pthread_kill(pthread_t thread, int signal_number)
{
    int result = __real_pthread_kill(thread, signal_number);
    if (signal_number == SIGRTMIN && __atomic_load_n(&armed, __ATOMIC_ACQUIRE) != 0) {
        require(result == 0, 23);
        uint64_t start = GetTickCount64();
        while (pending()) {
            require(GetTickCount64() - start < 2000, 24);
        }
        __atomic_store_n(&proceed, 1, __ATOMIC_RELEASE);
    }
    return result;
}
static DWORD actor(void *argument)
{
    (void)argument;
    __atomic_store_n(&actor_id, GetCurrentThreadId(), __ATOMIC_RELEASE);
    waiting = 1;
    require(WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0, 30);
    return 37;
}
int main(void)
{
    Os_StackPrepare();
    event = CreateEventA(NULL, 1, 0, NULL);
    require(event != NULL, 40);
    HANDLE thread = Os_HostCreateActor(262144, actor, NULL, 0, NULL, 0);
    require(thread != NULL, 41);
    uint64_t start = GetTickCount64();
    while (__atomic_load_n(&armed, __ATOMIC_ACQUIRE) == 0) {
        require(GetTickCount64() - start < 2000, 42);
        sched_yield();
    }
    require(SuspendThread(thread) == 0, 52);
    require(ResumeThread(thread) == 1, 53);
    require(Os_HostSetEvent(event) != 0, 54);
    require(WaitForSingleObject(thread, 2000) == WAIT_OBJECT_0, 55);
    require(CloseHandle(thread) != 0 && CloseHandle(event) != 0, 56);
    puts("event_suspend signal=delivered-before-poll acknowledgement=observed");
    return 0;
}
