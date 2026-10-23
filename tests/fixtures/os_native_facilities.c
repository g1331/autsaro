/** @file Linux native primitive consumers independent of automotive scheduling. */
#include "Os_Host.h"
#include "Os.h"
#include "Os_LinuxNative.h"
#include <asm/sigcontext.h>
#include <asm/ucontext.h>
#include <errno.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static int injected_error;
static HANDLE owned, other;
static DWORD worker_id;
static int worker_error;
static CONTEXT captured;
static unsigned char expected_fp[sizeof(captured.native.__fpregs_mem)];
static volatile sig_atomic_t captured_signal;
static void capture_signal(int signal_number, siginfo_t *info, void *context)
{
    struct ucontext kernel;
    (void)signal_number;
    (void)info;
    memcpy(&kernel, context, sizeof(kernel));
    if (kernel.uc_mcontext.fpstate != NULL) {
        memcpy(expected_fp, kernel.uc_mcontext.fpstate, sizeof(expected_fp));
        Os_LinuxCaptureContext(&captured, context);
        captured_signal = 1;
    }
}
void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }
int Os_FacilityTryLock(pthread_mutex_t *mutex)
{
    return injected_error != 0 ? EINVAL : pthread_mutex_trylock(mutex);
}
static void *contender(void *argument)
{
    (void)argument;
    worker_id = GetCurrentThreadId();
    if (WaitForSingleObject(other, 0u) != WAIT_OBJECT_0 ||
        WaitForSingleObject(owned, 0u) != WAIT_TIMEOUT || ReleaseMutex(owned) != FALSE ||
        ReleaseMutex(other) == FALSE) {
        worker_error = 1;
    }
    return NULL;
}
int main(int argc, char **argv)
{
    volatile int32_t cell = 1;
    const struct timespec immediate = {0, 0};
    uintptr_t low, high, sp;
    size_t guard;
    sigset_t original, current;
    pthread_t worker;
    struct sigaction handler = {0};
    if (argc != 2) {
        return 2;
    }
    injected_error = strcmp(argv[1], "error") == 0;
    handler.sa_sigaction = capture_signal;
    handler.sa_flags = SA_SIGINFO;
    if (sigemptyset(&handler.sa_mask) != 0 || sigaction(SIGUSR1, &handler, NULL) != 0 ||
        raise(SIGUSR1) != 0 || captured_signal != 1 || captured.Rsp == 0u ||
        Os_LinuxContextIp(&captured) == 0u || Os_LinuxContextOwnsFp(&captured) == 0 ||
        memcmp(expected_fp, &captured.native.__fpregs_mem, sizeof(expected_fp)) != 0) {
        return 9;
    }
    const uintptr_t saved_sp = captured.Rsp;
    Os_LinuxCaptureContext(&captured, &captured.native);
    if (captured.Rsp != saved_sp || Os_LinuxContextOwnsFp(&captured) == 0 ||
        memcmp(expected_fp, &captured.native.__fpregs_mem, sizeof(expected_fp)) != 0) {
        return 10;
    }
    if (Os_LinuxFutexWait(&cell, 0, &immediate) != -EAGAIN ||
        Os_LinuxFutexWait(&cell, 1, &immediate) != -ETIMEDOUT ||
        Os_LinuxThreadId() != (DWORD)getpid() ||
        Os_HostCurrentStackRange(&low, &high, &guard) == 0) {
        return 3;
    }
    OS_HOST_STACK_POINTER(sp);
    if (sp < low + guard || sp >= high || pthread_sigmask(SIG_SETMASK, NULL, &original) != 0) {
        return 4;
    }
    owned = CreateMutexA(NULL, FALSE, NULL);
    other = CreateMutexA(NULL, FALSE, NULL);
    if (owned == NULL || other == NULL) {
        return 5;
    }
    if (injected_error != 0) {
        if (WaitForSingleObject(owned, 0u) != WAIT_FAILED) {
            return 24;
        }
    } else {
        for (unsigned round = 0u; round < 2u; ++round) {
            if (WaitForSingleObject(owned, 0u) != WAIT_OBJECT_0 ||
                WaitForSingleObject(owned, 0u) != WAIT_OBJECT_0 || ReleaseMutex(owned) == FALSE ||
                pthread_sigmask(SIG_SETMASK, NULL, &current) != 0 ||
                sigismember(&current, SIGRTMIN) != 1 ||
                pthread_create(&worker, NULL, contender, NULL) != 0) {
                return 6;
            }
            if (pthread_join(worker, NULL) != 0 || worker_error != 0 ||
                worker_id == GetCurrentThreadId() || ReleaseMutex(owned) == FALSE ||
                ReleaseMutex(owned) != FALSE) {
                return 7;
            }
        }
    }
    if (pthread_sigmask(SIG_SETMASK, NULL, &current) != 0 ||
        sigismember(&original, SIGRTMIN) != sigismember(&current, SIGRTMIN) ||
        CloseHandle(owned) == FALSE || CloseHandle(other) == FALSE) {
        return 8;
    }
    puts("native facilities futex=2 mapping=1 mutex_owner=1 recursion=2 error=classified "
         "context=signal+self");
    return 0;
}
