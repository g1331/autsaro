/** @file Real mutex unlock before or during futex wait, with recursive ownership and masks. */
#include "Os.h"
#include "Os_Host.h"
#include "Os_LinuxNative.h"
#include <asm/unistd.h>
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static HANDLE target;
static pthread_mutex_t gate = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static int go;
static int contended;
static int proceed;
static int mode_before;
static int mode_error;
static int observed_wait;
static int observed_wake;
static int wait_result;
static DWORD worker_result;
static DWORD worker_id;
static const volatile int32_t *wait_address;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }
int __real_pthread_mutex_trylock(pthread_mutex_t *mutex);
int __real_Os_LinuxFutexWait(const volatile int32_t *address,
                             int32_t expected,
                             const struct timespec *interval);
void __real_Os_LinuxFutexWake(const volatile int32_t *address);

static void require(int accepted, int code)
{
    if (accepted == 0) {
        fprintf(stderr, "mutex_wakeup_rejected stage=%d\n", code);
        _exit(code);
    }
}

int __wrap_pthread_mutex_trylock(pthread_mutex_t *mutex)
{
    const int result = __real_pthread_mutex_trylock(mutex);
    if (result == EBUSY) {
        require(pthread_mutex_lock(&gate) == 0, 10);
        if (contended == 0) {
            contended = 1;
            require(pthread_cond_signal(&changed) == 0, 11);
            while (proceed == 0) {
                require(pthread_cond_wait(&changed, &gate) == 0, 12);
            }
        }
        require(pthread_mutex_unlock(&gate) == 0, 13);
    }
    return result;
}

int __wrap_Os_LinuxFutexWait(const volatile int32_t *address,
                             int32_t expected,
                             const struct timespec *interval)
{
    sigset_t mask;
    require(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0, 14);
    require(sigismember(&mask, SIGRTMIN) == 0, 15);
    require(pthread_mutex_lock(&gate) == 0, 16);
    wait_address = address;
    ++observed_wait;
    require(pthread_cond_signal(&changed) == 0, 17);
    require(pthread_mutex_unlock(&gate) == 0, 18);
    const int result = mode_error ? -EFAULT : __real_Os_LinuxFutexWait(address, expected, interval);
    wait_result = result;
    return result;
}

void __wrap_Os_LinuxFutexWake(const volatile int32_t *address)
{
    (void)__atomic_add_fetch(&observed_wake, 1, __ATOMIC_SEQ_CST);
    __real_Os_LinuxFutexWake(address);
}

static void *waiter(void *argument)
{
    (void)argument;
    require(pthread_mutex_lock(&gate) == 0, 19);
    worker_id = GetCurrentThreadId();
    while (go == 0) {
        require(pthread_cond_wait(&changed, &gate) == 0, 20);
    }
    require(pthread_mutex_unlock(&gate) == 0, 21);
    worker_result = WaitForSingleObject(target, 5000u);
    if (worker_result == WAIT_OBJECT_0) {
        sigset_t mask;
        require(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0, 22);
        require(sigismember(&mask, SIGRTMIN) == 1, 23);
        require(ReleaseMutex(target) != 0, 24);
        require(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0, 25);
        require(sigismember(&mask, SIGRTMIN) == 0, 26);
    }
    return NULL;
}

static int sleeping_on_mutex(void)
{
    char path[96];
    require(snprintf(path, sizeof(path), "/proc/self/task/%lu/syscall", (unsigned long)worker_id) >
                0,
            27);
    FILE *file = fopen(path, "r");
    require(file != NULL, 28);
    long number;
    unsigned long address;
    const int fields = fscanf(file, "%ld %lx", &number, &address);
    require(fclose(file) == 0, 29);
    return (fields == 2) && (number == __NR_futex) && (address == (uintptr_t)wait_address);
}

int main(int argc, char **argv)
{
    require(argc == 2, 30);
    mode_before = strcmp(argv[1], "before-wait") == 0;
    mode_error = strcmp(argv[1], "wait-error") == 0;
    target = CreateMutexA(NULL, 0, NULL);
    require(target != NULL, 31);
    pthread_t worker;
    /* Create the waiter before taking the mutex, so it inherits no held-mask state. */
    require(pthread_create(&worker, NULL, &waiter, NULL) == 0, 32);
    require(WaitForSingleObject(target, 0u) == WAIT_OBJECT_0, 33);
    require(WaitForSingleObject(target, 0u) == WAIT_OBJECT_0, 34);
    require(pthread_mutex_lock(&gate) == 0, 35);
    go = 1;
    require(pthread_cond_signal(&changed) == 0, 36);
    while (contended == 0) {
        require(pthread_cond_wait(&changed, &gate) == 0, 37);
    }
    require(ReleaseMutex(target) != 0, 38);
    require(__atomic_load_n(&observed_wake, __ATOMIC_SEQ_CST) == 0, 39);
    if (mode_before != 0) {
        require(ReleaseMutex(target) != 0, 40);
        require(__atomic_load_n(&observed_wake, __ATOMIC_SEQ_CST) == 1, 41);
    }
    proceed = 1;
    require(pthread_cond_signal(&changed) == 0, 42);
    if ((mode_before == 0) && (mode_error == 0)) {
        while (observed_wait == 0) {
            require(pthread_cond_wait(&changed, &gate) == 0, 43);
        }
        require(pthread_mutex_unlock(&gate) == 0, 44);
        const uint64_t started = GetTickCount64();
        while (sleeping_on_mutex() == 0) {
            require(GetTickCount64() - started < 5000u, 45);
            (void)sched_yield();
        }
        require(ReleaseMutex(target) != 0, 46);
    } else {
        require(pthread_mutex_unlock(&gate) == 0, 47);
    }
    require(pthread_join(worker, NULL) == 0, 48);
    if (mode_error != 0) {
        require(worker_result == WAIT_FAILED, 49);
        require(ReleaseMutex(target) != 0, 50);
    } else {
        require(worker_result == WAIT_OBJECT_0, 51);
        require(wait_result == (mode_before ? -EAGAIN : 0), 52);
    }
    require(observed_wait == 1, 53);
    require(CloseHandle(target) != 0, 54);
    printf("mutex_wakeup scenario=%s acquisition=observed masks=restored\n", argv[1]);
    return 0;
}
