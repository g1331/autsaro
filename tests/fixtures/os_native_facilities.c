/** @file Linux native primitive consumers independent of automotive scheduling. */
#include "Os_Host.h"
#include "Os.h"
#include "Os_Stack.h"
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
static uintptr_t expected_sp;
static unsigned char expected_fp[sizeof(captured.native.__fpregs_mem)];
static volatile sig_atomic_t captured_signal;
static const void *frame_source;
static volatile sig_atomic_t frame_copies;
void *Os_TestNativeCopy(
    void *destination, const void *source, size_t length, size_t capacity, int moving)
{
    if (source == frame_source && frame_source != NULL) {
        ++frame_copies;
        const size_t expected = frame_copies == 1 ? 304u : 168u;
        if (length != expected || frame_copies > 2) {
            static const char capture[] = "frame-copy rejected capture\n";
            static const char sp[] = "frame-copy rejected sp\n";
            const char *message = frame_copies == 1 ? capture : sp;
            const size_t bytes = frame_copies == 1 ? sizeof(capture) - 1u : sizeof(sp) - 1u;
            if (write(STDERR_FILENO, message, bytes) != (ssize_t)bytes) {
                _exit(44);
            }
            _exit(frame_copies == 1 ? 41 : 42);
        }
    }
    return moving != 0 ? __builtin___memmove_chk(destination, source, length, capacity)
                       : __builtin___memcpy_chk(destination, source, length, capacity);
}
static volatile unsigned deep_returns;
static int reject_context(const char *stage, int code)
{
    return fprintf(stderr, "native-context rejected %s\n", stage) < 0 ? 46 : code;
}
static DWORD completed_actor(void *argument)
{
    (void)argument;
    return 37u;
}
__attribute__((noinline)) static void deep_call(unsigned depth)
{
    if (depth != 0u) {
        deep_call(depth - 1u);
    }
    __asm__ volatile("" ::: "memory");
    ++deep_returns;
}
static DWORD deep_actor(void *argument)
{
    uintptr_t low, high, sp;
    uintptr_t ssp = 0u;
    size_t guard;
    (void)argument;
    OS_HOST_STACK_POINTER(sp);
    __asm__ volatile("rdsspq %0" : "+r"(ssp));
    if (Os_HostCurrentStackRange(&low, &high, &guard) == 0 || sp < low + guard || sp >= high) {
        return 1u;
    }
    FILE *maps = fopen("/proc/self/maps", "r");
    unsigned long shadow_size = 0u;
    if (maps != NULL) {
        char line[256];
        while (fgets(line, sizeof(line), maps) != NULL) {
            unsigned long first, last;
            if (sscanf(line, "%lx-%lx", &first, &last) == 2 && first <= ssp && ssp < last) {
                shadow_size = last - first;
                break;
            }
        }
        (void)fclose(maps);
    }
    printf("native-stack usable=%lu shadow=%lu ssp=%lu\n",
           (unsigned long)(high - low - guard),
           shadow_size,
           (unsigned long)ssp);
    (void)fflush(stdout);
    deep_call(50000u);
    return deep_returns == 50001u ? 0u : 2u;
}
static void capture_signal(int signal_number, siginfo_t *info, void *context)
{
    struct ucontext kernel;
    (void)signal_number;
    (void)info;
    /* Fixed x64 kernel prefix through its 64-bit mask, independent of glibc's tail. */
    memcpy(&kernel, context, 304u);
    expected_sp = kernel.uc_mcontext.rsp;
    if (kernel.uc_mcontext.fpstate != NULL) {
        memcpy(expected_fp, kernel.uc_mcontext.fpstate, sizeof(expected_fp));
        frame_source = context;
        Os_LinuxCaptureContext(&captured, context);
        captured_signal = Os_LinuxSignalSp(context) == expected_sp ? 1 : 0;
        frame_source = NULL;
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
    if (strcmp(argv[1], "owned-capacity") == 0) {
        if (pthread_sigmask(SIG_SETMASK, NULL, &original) != 0 ||
            CreateMutexA(NULL, TRUE, "unsupported") != NULL) {
            return 25;
        }
        owned = CreateMutexA(NULL, TRUE, NULL);
        if (owned == NULL) {
            return 26;
        }
        for (unsigned depth = 1u; depth < 32u; ++depth) {
            if (WaitForSingleObject(owned, 0u) != WAIT_OBJECT_0) {
                return 27;
            }
        }
        if (WaitForSingleObject(owned, 0u) != WAIT_FAILED) {
            return 28;
        }
        for (unsigned depth = 0u; depth < 32u; ++depth) {
            if (ReleaseMutex(owned) == FALSE) {
                return 29;
            }
        }
        if (ReleaseMutex(owned) != FALSE || pthread_sigmask(SIG_SETMASK, NULL, &current) != 0 ||
            sigismember(&original, SIGRTMIN) != sigismember(&current, SIGRTMIN) ||
            CloseHandle(owned) == FALSE) {
            return 30;
        }
        puts("native mutex initially-owned depth=32 overflow=rejected mask=restored");
        return 0;
    }
    if (strcmp(argv[1], "actor-id") == 0) {
        DWORD identity = 0u;
        DWORD code = 0u;
        CONTEXT context = {0};
        Os_StackPrepare();
        HANDLE actor = CreateThread(NULL, 262144u, completed_actor, NULL, 0u, &identity);
        if (actor == NULL || identity == 0u || identity != GetThreadId(actor) ||
            WaitForSingleObject(actor, 2000u) != WAIT_OBJECT_0 ||
            WaitForSingleObject(actor, 0u) != WAIT_OBJECT_0 ||
            GetExitCodeThread(actor, &code) == FALSE || code != 37u ||
            SuspendThread(actor) != UINT32_MAX || ResumeThread(actor) != UINT32_MAX ||
            GetThreadContext(actor, &context) != FALSE || CloseHandle(actor) == FALSE) {
            return 31;
        }
        puts("native actor identity=published completion=joined-once ended=rejected");
        return 0;
    }
    if (strcmp(argv[1], "depth") == 0 || strcmp(argv[1], "cet-depth") == 0) {
        if (argv[1][0] == 'c') {
            long result;
            unsigned long enabled = 0u;
            __asm__ volatile("syscall"
                             : "=a"(result)
                             : "a"(158L), "D"(0x5001L), "S"(1L)
                             : "rcx", "r11", "memory");
            if (result != 0) {
                printf("native-stack CET not_assessed enable=%ld\n", result);
                (void)fflush(stdout);
                _exit(77);
            }
            __asm__ volatile("syscall"
                             : "=a"(result)
                             : "a"(158L), "D"(0x5005L), "S"(&enabled)
                             : "rcx", "r11", "memory");
            printf("native-stack CET status=%lu result=%ld\n", enabled, result);
            (void)fflush(stdout);
            if (result != 0 || enabled != 1u) {
                _exit(12);
            }
        }
        Os_StackPrepare();
        HANDLE actor = CreateThread(NULL, 1048576u, deep_actor, NULL, 0u, NULL);
        if (actor == NULL || WaitForSingleObject(actor, 10000u) != WAIT_OBJECT_0 ||
            deep_returns != 50001u || CloseHandle(actor) == FALSE) {
            _exit(13);
        }
        puts("native-stack depth=50000 completed");
        (void)fflush(stdout);
        _exit(0);
    }
    injected_error = strcmp(argv[1], "error") == 0;
    struct ucontext snapshot;
    handler.sa_sigaction = capture_signal;
    handler.sa_flags = SA_SIGINFO;
    if (sigemptyset(&handler.sa_mask) != 0 || sigaction(SIGUSR1, &handler, NULL) != 0 ||
        raise(SIGUSR1) != 0) {
        return reject_context("setup", 9);
    }
    const char *failure =
        captured_signal != 1                                                           ? "signal-sp"
        : captured.Rsp != expected_sp                                                  ? "rsp"
        : captured.ContextFlags != CONTEXT_CONTROL                                     ? "flags"
        : memcmp(expected_fp, &captured.native.__fpregs_mem, sizeof(expected_fp)) != 0 ? "fp"
                                                                                       : NULL;
    if (failure != NULL) {
        return reject_context(failure, 9);
    }
    if (strcmp(argv[1], "frame-copies") == 0 && frame_copies != 2) {
        return 43;
    }
    memcpy(&snapshot, &captured.native, sizeof(snapshot));
    if (snapshot.uc_mcontext.rip == 0u ||
        (const void *)snapshot.uc_mcontext.fpstate != &captured.native.__fpregs_mem) {
        return reject_context("fp-owner", 11);
    }
    const uintptr_t saved_sp = captured.Rsp;
    Os_LinuxCaptureContext(&captured, &captured.native);
    memcpy(&snapshot, &captured.native, sizeof(snapshot));
    if (captured.Rsp != saved_sp || captured.ContextFlags != CONTEXT_CONTROL ||
        (const void *)snapshot.uc_mcontext.fpstate != &captured.native.__fpregs_mem ||
        memcmp(expected_fp, &captured.native.__fpregs_mem, sizeof(expected_fp)) != 0) {
        return reject_context("self-copy", 10);
    }
    CONTEXT absent = {0};
    const unsigned char zero_fp[sizeof(absent.native.__fpregs_mem)] = {0};
    snapshot.uc_mcontext.fpstate = NULL;
    Os_LinuxCaptureContext(&absent, &snapshot);
    memcpy(&snapshot, &absent.native, sizeof(snapshot));
    if (absent.Rsp != saved_sp || absent.ContextFlags != CONTEXT_CONTROL ||
        snapshot.uc_mcontext.fpstate != NULL ||
        memcmp(zero_fp, &absent.native.__fpregs_mem, sizeof(zero_fp)) != 0) {
        return reject_context("absent-fp", 45);
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
