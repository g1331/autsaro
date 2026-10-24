/** @file Actual Linux facilities observe fresh errno even after successful calls leave a value. */
#include "Os_HostLinux.h"
#if defined(TRUE) || defined(FALSE)
#error Native declarations must not own the AUTOSAR boolean constants
#endif
#include "Os_Host.h"
#include "Os_Stack.h"
#include "Os.h"
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>

static const char *scenario;
static unsigned bad_errno;
static unsigned observed;
static HANDLE gate;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

static void entered(unsigned bit)
{
    if (errno != 0) {
        (void)__atomic_fetch_or(&bad_errno, bit, __ATOMIC_RELAXED);
    }
    (void)__atomic_fetch_or(&observed, bit, __ATOMIC_RELAXED);
}

int __real_clock_gettime(clockid_t clock, struct timespec *value);
int __wrap_clock_gettime(clockid_t clock, struct timespec *value)
{
    entered(1u);
    const int result = __real_clock_gettime(clock, value);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

long __real_sysconf(int name);
long __wrap_sysconf(int name)
{
    entered(2u);
    const long result = __real_sysconf(name);
    if (result >= 0) {
        errno = EAGAIN;
    }
    return result;
}

void *__real_mmap(void *address, size_t length, int protection, int flags, int fd, off_t offset);
void *__wrap_mmap(void *address, size_t length, int protection, int flags, int fd, off_t offset)
{
    entered(4u);
    void *result = __real_mmap(address, length, protection, flags, fd, offset);
    if (result != MAP_FAILED) {
        errno = EAGAIN;
    }
    return result;
}

int __real_mprotect(void *address, size_t length, int protection);
int __wrap_mprotect(void *address, size_t length, int protection)
{
    entered(8u);
    if ((strcmp(scenario, "protect-error") == 0) ||
        (strcmp(scenario, "protect-rollback-error") == 0)) {
        errno = EACCES;
        return -1;
    }
    const int result = __real_mprotect(address, length, protection);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_munmap(void *address, size_t length);
int __wrap_munmap(void *address, size_t length)
{
    entered(16u);
    if ((strcmp(scenario, "protect-rollback-error") == 0) ||
        (strcmp(scenario, "create-rollback-error") == 0)) {
        (void)fprintf(stderr, "rollback_munmap_failed stage=%s\n", scenario);
        errno = ENOMEM;
        return -1;
    }
    const int result = __real_munmap(address, length);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_eventfd(unsigned value, int flags);
int __wrap_eventfd(unsigned value, int flags)
{
    entered(32u);
    const int result = __real_eventfd(value, flags);
    if (result >= 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_nanosleep(const struct timespec *duration, struct timespec *remaining);
int __wrap_nanosleep(const struct timespec *duration, struct timespec *remaining)
{
    static unsigned sleeps;
    entered(64u);
    if ((strcmp(scenario, "sleep-interrupted") == 0) &&
        (__atomic_fetch_add(&sleeps, 1u, __ATOMIC_RELAXED) == 0u)) {
        if (remaining != NULL) {
            *remaining = *duration;
        }
        errno = EINTR;
        return -1;
    }
    const int result = __real_nanosleep(duration, remaining);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_sigaddset(sigset_t *set, int signal_number);
int __wrap_sigaddset(sigset_t *set, int signal_number)
{
    entered(128u);
    const int result = __real_sigaddset(set, signal_number);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_sigaction(int signal_number, const struct sigaction *action, struct sigaction *previous);
int __wrap_sigaction(int signal_number, const struct sigaction *action, struct sigaction *previous)
{
    entered(256u);
    const int result = __real_sigaction(signal_number, action, previous);
    if (result == 0) {
        errno = EAGAIN;
    }
    return result;
}

int __real_pthread_create(pthread_t *thread,
                          const pthread_attr_t *attributes,
                          void *(*entry)(void *),
                          void *argument);
int __wrap_pthread_create(pthread_t *thread,
                          const pthread_attr_t *attributes,
                          void *(*entry)(void *),
                          void *argument)
{
    if ((strcmp(scenario, "create-error") == 0) ||
        (strcmp(scenario, "create-rollback-error") == 0)) {
        return EAGAIN;
    }
    return __real_pthread_create(thread, attributes, entry, argument);
}

static DWORD WINAPI actor(void *argument)
{
    (void)argument;
    return (WaitForSingleObject(gate, INFINITE) == WAIT_OBJECT_0) ? 42u : 43u;
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        return 1;
    }
    scenario = argv[1];
    errno = EBUSY;
    Os_StackPrepare();
    if (GetTickCount64() == UINT64_MAX) {
        return 2;
    }
    Sleep(0u);
    HANDLE event = CreateEventA(NULL, 0, 0, NULL);
    if ((event == NULL) || (CloseHandle(event) == 0)) {
        return 3;
    }
    HANDLE mutex = CreateMutexA(NULL, 0, NULL);
    if ((mutex == NULL) || (WaitForSingleObject(mutex, 0u) != WAIT_OBJECT_0) ||
        (ReleaseMutex(mutex) == 0) || (CloseHandle(mutex) == 0)) {
        return 3;
    }
    gate = CreateEventA(NULL, 0, 0, NULL);
    if (gate == NULL) {
        return 3;
    }
    HANDLE thread =
        Os_HostCreateActor(262144u, actor, NULL, STACK_SIZE_PARAM_IS_A_RESERVATION, NULL, 0);
    if ((strcmp(scenario, "protect-error") == 0) || (strcmp(scenario, "create-error") == 0) ||
        (strcmp(scenario, "protect-rollback-error") == 0) ||
        (strcmp(scenario, "create-rollback-error") == 0)) {
        if (thread != NULL) {
            return 4;
        }
    } else {
        DWORD code = 0u;
        const HANDLE pending[] = {gate, thread};
        if ((thread == NULL) || (WaitForSingleObject(thread, 1u) != WAIT_TIMEOUT) ||
            (WaitForMultipleObjects(2u, pending, 0, 1u) != WAIT_TIMEOUT) ||
            (Os_HostSetEvent(gate) == 0) || (WaitForSingleObject(thread, 5000u) != WAIT_OBJECT_0) ||
            (GetExitCodeThread(thread, &code) == 0) || (code != 42u) ||
            (CloseHandle(thread) == 0)) {
            return 5;
        }
    }
    if (CloseHandle(gate) == 0) {
        return 3;
    }
    if ((__atomic_load_n(&bad_errno, __ATOMIC_RELAXED) != 0u) ||
        (__atomic_load_n(&observed, __ATOMIC_RELAXED) != 511u)) {
        (void)fprintf(stderr,
                      "errno_bad=%u observed=%u\n",
                      __atomic_load_n(&bad_errno, __ATOMIC_RELAXED),
                      __atomic_load_n(&observed, __ATOMIC_RELAXED));
        return 6;
    }
    puts("native errno initialized across signal, clock, sleep, mapping, event and cleanup calls");
    return 0;
}
