/** @file Linux host layer emulating the Win32 threads, events and signals the OS backend uses. */
#include "Os_HostLinux.h"
#include "Os_LinuxNative.h"
#include "Os_HostIo.h"
#include "Os_Stack.h"

#include <errno.h>
#include <linux/futex.h>
#include <poll.h>
#include <signal.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <asm/mman.h>
#ifdef __SANITIZE_ADDRESS__
#include <sanitizer/common_interface_defs.h>
#define OS_LINUX_STACK_START(fake, bottom, length)                                                 \
    __sanitizer_start_switch_fiber((fake), (bottom), (length))
#define OS_LINUX_STACK_FINISH(fake, bottom, length)                                                \
    __sanitizer_finish_switch_fiber((fake), (bottom), (length))
#else
#define OS_LINUX_STACK_START(fake, bottom, length)  ((void)0)
#define OS_LINUX_STACK_FINISH(fake, bottom, length) ((void)0)
#endif
#include <time.h>
#include <unistd.h>

#define OS_HOST_HANDLES       96u
#define OS_HOST_DEFAULT_STACK 262144u
#define OS_HOST_WAIT_SLICE_NS 10000000L

typedef enum {
    OS_HOST_UNUSED = 0,
    OS_HOST_EVENT,
    OS_HOST_MUTEX,
    OS_HOST_THREAD,
    OS_HOST_PROCESS
} Os_HostKindType;

struct Os_HostHandleType {
    volatile int32_t reserved;
    Os_HostKindType kind;
    int fd;
    int manual;
    pthread_mutex_t mutex;
    DWORD mutex_owner;
    uint32_t mutex_depth;
    pthread_t thread;
    DWORD thread_id;
    LPTHREAD_START_ROUTINE entry;
    LPVOID argument;
    void *mapping;
    size_t mapping_size;
    size_t guard_size;
    int register_host;
    volatile int32_t requested;
    volatile int32_t acknowledged;
    volatile int32_t resumed;
    volatile int32_t ended;
    volatile int32_t joined;
    DWORD exit_code;
    CONTEXT captured;
#ifdef __SANITIZE_ADDRESS__
    void *bootstrap_fake;
    const void *bootstrap_bottom;
    size_t bootstrap_size;
#endif
};

#define OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static struct Os_HostHandleType
    handles[OS_HOST_HANDLES]; /**< Atomically reserved slots, per-handle
                   synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostLinux.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostLinux.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_INIT_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static struct Os_HostHandleType process_handle = {
    .reserved = 1,
    .kind = OS_HOST_PROCESS}; /**< C-initialized immutable process pseudo-handle; no Init writes. */
#define OS_STOP_SEC_VAR_INIT_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE dispatcher; /**< Atomic dispatcher-handle publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL HANDLE
    current_thread; /**< Native actor TLS handle; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL sigset_t
    held_masks[32]; /**< Actor TLS prior signal masks; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL unsigned
    held_depth; /**< Actor TLS signal-mask nesting depth; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static int futex_wait(const volatile int32_t *cell, int32_t expected)
{
    const struct timespec interval = {.tv_sec = 0, .tv_nsec = OS_HOST_WAIT_SLICE_NS};
    return Os_LinuxFutexWait(cell, expected, &interval);
}

OS_MEMMAP static void futex_wake(const volatile int32_t *cell) { Os_LinuxFutexWake(cell); }

OS_MEMMAP static uint64_t monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        return UINT64_MAX;
    }
    return ((uint64_t)now.tv_sec * UINT64_C(1000)) + ((uint64_t)now.tv_nsec / UINT64_C(1000000));
}

OS_MEMMAP static HANDLE reserve_handle(Os_HostKindType kind)
{
    size_t index;
    for (index = 0u; index < OS_HOST_HANDLES; ++index) {
        int32_t expected = 0;
        if (__atomic_compare_exchange_n(
                &handles[index].reserved, &expected, 1, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST)) {
            handles[index].kind = kind;
            handles[index].fd = -1;
            return &handles[index];
        }
    }
    return NULL;
}

OS_MEMMAP static void terminal_park(HANDLE thread)
{
    __atomic_store_n(&thread->acknowledged, INT32_MAX, __ATOMIC_RELEASE);
    futex_wake(&thread->acknowledged);
    for (;;) {
        const int32_t current = __atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE);
        (void)futex_wait(&thread->resumed, current);
    }
}

OS_MEMMAP static void park_signal(int signal_number, siginfo_t *info, void *native_context)
{
    /* Preserve the interrupted actor's errno across signal-context capture and parking. */
    const int interrupted_errno = errno;
    HANDLE thread = current_thread;
    (void)signal_number;
    (void)info;
    if ((thread != NULL) && (thread->kind == OS_HOST_THREAD)) {
        const int32_t generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
        if (generation > __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE)) {
            Os_LinuxCaptureContext(&thread->captured, native_context);
            __atomic_store_n(&thread->acknowledged, generation, __ATOMIC_RELEASE);
            futex_wake(&thread->acknowledged);
            for (;;) {
                /* Stop an already parked actor without nesting another signal handler. */
                if (InterlockedCompareExchange(&thread->requested, 0, 0) == INT32_MAX) {
                    terminal_park(thread);
                }
                if (InterlockedCompareExchange(&thread->resumed, 0, 0) >= generation) {
                    break;
                }
                (void)futex_wait(&thread->resumed, generation - 1);
            }
        }
    }
    errno = interrupted_errno;
}

OS_MEMMAP static void fault_stop_signal(int signal_number, siginfo_t *info, void *native_context)
{
    HANDLE thread = current_thread;
    (void)signal_number;
    (void)info;
    if (thread == NULL) {
        _exit(1);
    }
    Os_LinuxCaptureContext(&thread->captured, native_context);
    terminal_park(thread);
}

OS_MEMMAP int Os_HostInstallSignals(void)
{
    struct sigaction handler = {0};
    handler.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&handler.sa_mask);
    sigaddset(&handler.sa_mask, SIGRTMIN);
    handler.sa_sigaction = park_signal;
    if ((sigaddset(&handler.sa_mask, SIGRTMIN + 1) != 0) ||
        (sigaction(SIGRTMIN, &handler, NULL) != 0)) {
        return 0;
    }
    sigfillset(&handler.sa_mask);
    handler.sa_sigaction = fault_stop_signal;
    return sigaction(SIGRTMIN + 1, &handler, NULL) == 0;
}

OS_MEMMAP static DWORD actor_body(void *argument)
{
    HANDLE thread = current_thread;
    (void)argument;
    OS_LINUX_STACK_FINISH(NULL, &thread->bootstrap_bottom, &thread->bootstrap_size);
    __atomic_store_n(&thread->thread_id, GetCurrentThreadId(), __ATOMIC_RELEASE);
    DWORD result;
    if ((thread->register_host != 0) && (Os_StackRegister('H') == 0)) {
        result = UINT32_MAX;
    } else {
        result = thread->entry(thread->argument);
    }
    /* Finite helper actors destroy their execution fiber before pthread retirement. */
    OS_LINUX_STACK_START(NULL, thread->bootstrap_bottom, thread->bootstrap_size);
    return result;
}

OS_MEMMAP static void *actor_start(void *argument)
{
    HANDLE thread = argument;
    current_thread = thread;
    OS_LINUX_STACK_START(&thread->bootstrap_fake,
                         (char *)thread->mapping + thread->guard_size,
                         thread->mapping_size - thread->guard_size);
    thread->exit_code =
        Os_LinuxCallOnStack(&((char *)thread->mapping)[thread->mapping_size], actor_body, thread);
    OS_LINUX_STACK_FINISH(thread->bootstrap_fake, NULL, NULL);
    __atomic_store_n(&thread->ended, 1, __ATOMIC_RELEASE);
    futex_wake(&thread->ended);
    return NULL;
}

OS_MEMMAP HANDLE Os_HostCreateActor(SIZE_T stack,
                                    LPTHREAD_START_ROUTINE start,
                                    LPVOID argument,
                                    DWORD flags,
                                    LPDWORD id,
                                    int register_host)
{
    HANDLE thread;
    pthread_attr_t attributes;
    long page;
    size_t reserve;
    int status;
    if (start == NULL || (flags & CREATE_SUSPENDED) != 0u ||
        (flags & ~(STACK_SIZE_PARAM_IS_A_RESERVATION | CREATE_SUSPENDED)) != 0u) {
        return NULL;
    }
    page = sysconf(_SC_PAGESIZE);
    if (page <= 0 || stack > SIZE_MAX - (size_t)page) {
        return NULL;
    }
    reserve = stack == 0u ? OS_HOST_DEFAULT_STACK : stack;
    const long minimum = sysconf(_SC_THREAD_STACK_MIN);
    if ((minimum <= 0) || (reserve < ((size_t)minimum + (size_t)page))) {
        return NULL;
    }
    reserve = ((reserve + (size_t)page - 1u) / (size_t)page) * (size_t)page;
    thread = reserve_handle(OS_HOST_THREAD);
    if (thread == NULL) {
        return NULL;
    }
    thread->mapping =
        mmap(NULL, reserve, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (thread->mapping == MAP_FAILED) {
        thread->mapping = NULL;
        thread->reserved = 0;
        return NULL;
    }
    if (mprotect(thread->mapping, (size_t)page, PROT_NONE) != 0) {
        (void)munmap(thread->mapping, reserve);
        thread->mapping = NULL;
        thread->reserved = 0;
        return NULL;
    }
    thread->mapping_size = reserve;
    thread->guard_size = (size_t)page;
    thread->entry = start;
    thread->argument = argument;
    thread->register_host = register_host;
    thread->thread_id = 0u;
    thread->requested = 0;
    thread->acknowledged = 0;
    thread->resumed = 0;
    thread->ended = 0;
    thread->joined = 0;
    thread->exit_code = 0u;
    status = pthread_attr_init(&attributes);
    if (status == 0) {
        status = pthread_attr_setstacksize(&attributes, OS_HOST_DEFAULT_STACK);
        if (status == 0) {
            status = pthread_create(&thread->thread, &attributes, actor_start, thread);
        }
        (void)pthread_attr_destroy(&attributes);
    }
    if (status != 0) {
        (void)munmap(thread->mapping, reserve);
        thread->mapping = NULL;
        thread->reserved = 0;
        return NULL;
    }
    if (id != NULL) {
        const uint64_t end = monotonic_ms() + 2000u;
        while (GetThreadId(thread) == 0u && monotonic_ms() < end) {
            const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
            (void)nanosleep(&interval, NULL);
        }
        if (GetThreadId(thread) == 0u) {
            return NULL;
        }
        *id = GetThreadId(thread);
    }
    return thread;
}

OS_MEMMAP HANDLE CreateThread(LPSECURITY_ATTRIBUTES attributes,
                              SIZE_T stack,
                              LPTHREAD_START_ROUTINE start,
                              LPVOID argument,
                              DWORD flags,
                              LPDWORD id)
{
    if (attributes != NULL) {
        return NULL;
    }
    return Os_HostCreateActor(stack, start, argument, flags, id, 1);
}

OS_MEMMAP HANDLE CreateEventA(LPSECURITY_ATTRIBUTES attributes,
                              BOOL manual,
                              BOOL initial,
                              LPCSTR name)
{
    HANDLE event;
    if (attributes != NULL || name != NULL) {
        return NULL;
    }
    event = reserve_handle(OS_HOST_EVENT);
    if (event == NULL) {
        return NULL;
    }
    event->manual = manual != 0;
    event->fd = eventfd(initial != 0 ? 1u : 0u, EFD_CLOEXEC | EFD_NONBLOCK);
    if (event->fd < 0) {
        event->reserved = 0;
        return NULL;
    }
    return event;
}

OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event)
{
    const uint64_t signal_value = 1u;
    if (event == NULL || event->kind != OS_HOST_EVENT || event->fd < 0) {
        return FALSE;
    }
    return write(event->fd, &signal_value, sizeof(signal_value)) == (ssize_t)sizeof(signal_value);
}

OS_MEMMAP BOOL ResetEvent(HANDLE event)
{
    uint64_t value;
    if (event == NULL || event->kind != OS_HOST_EVENT || event->fd < 0) {
        return FALSE;
    }
    while (read(event->fd, &value, sizeof(value)) == sizeof(value)) {
    }
    return errno == EAGAIN;
}

OS_MEMMAP HANDLE CreateMutexA(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name)
{
    HANDLE mutex;
    if (attributes != NULL || name != NULL) {
        return NULL;
    }
    mutex = reserve_handle(OS_HOST_MUTEX);
    if (mutex == NULL) {
        return NULL;
    }
    if (pthread_mutex_init(&mutex->mutex, NULL) != 0) {
        mutex->reserved = 0;
        return NULL;
    }
    if (owner != 0 && WaitForSingleObject(mutex, 0u) != WAIT_OBJECT_0) {
        (void)pthread_mutex_destroy(&mutex->mutex);
        mutex->reserved = 0;
        return NULL;
    }
    return mutex;
}

OS_MEMMAP DWORD WaitForSingleObject(HANDLE handle, DWORD milliseconds)
{
    uint64_t end = monotonic_ms();
    DWORD outcome = WAIT_FAILED;
    if ((handle != NULL) && (end != UINT64_MAX)) {
        int pending = 1;
        if (milliseconds != INFINITE) {
            end += milliseconds;
        }
        while (pending != 0) {
            if ((handle->kind == OS_HOST_EVENT) && (handle->fd >= 0)) {
                struct pollfd observed = {.fd = handle->fd, .events = POLLIN, .revents = 0};
                uint64_t value;
                const int ready = poll(&observed, 1u, 0);
                if ((ready > 0) && ((observed.revents & POLLIN) != 0)) {
                    if ((handle->manual != 0) ||
                        (read(handle->fd, &value, sizeof(value)) == sizeof(value))) {
                        outcome = WAIT_OBJECT_0;
                        pending = 0;
                    }
                }
            } else if (handle->kind == OS_HOST_MUTEX) {
                sigset_t blocked, previous;
                sigemptyset(&blocked);
                sigaddset(&blocked, SIGRTMIN);
                if ((held_depth < 32u) && (pthread_sigmask(SIG_BLOCK, &blocked, &previous) == 0)) {
                    const DWORD caller = GetCurrentThreadId();
                    int locked = EINVAL;
                    if (caller != 0u) {
                        if (__atomic_load_n(&handle->mutex_owner, __ATOMIC_ACQUIRE) == caller) {
                            /* mutex_depth is bounded by the existing held_depth limit. */
                            ++handle->mutex_depth;
                            locked = 0;
                        } else {
                            locked = pthread_mutex_trylock(&handle->mutex);
                            if (locked == 0) {
                                handle->mutex_depth = 1u;
                                __atomic_store_n(&handle->mutex_owner, caller, __ATOMIC_RELEASE);
                            }
                        }
                    }
                    if (locked == 0) {
                        held_masks[held_depth++] = previous;
                        outcome = WAIT_OBJECT_0;
                        pending = 0;
                    } else {
                        (void)pthread_sigmask(SIG_SETMASK, &previous, NULL);
                        if (locked != EBUSY) {
                            pending = 0;
                        }
                    }
                } else {
                    pending = 0;
                }
            } else if (handle->kind == OS_HOST_THREAD) {
                if (__atomic_load_n(&handle->ended, __ATOMIC_ACQUIRE) != 0) {
                    if ((__atomic_exchange_n(&handle->joined, 1, __ATOMIC_ACQ_REL) != 0) ||
                        (pthread_join(handle->thread, NULL) == 0)) {
                        outcome = WAIT_OBJECT_0;
                    }
                    pending = 0;
                }
            } else {
                pending = 0;
            }
            if ((pending != 0) && (milliseconds != INFINITE) && (monotonic_ms() >= end)) {
                outcome = WAIT_TIMEOUT;
                pending = 0;
            }
            if (pending != 0) {
                const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
                (void)nanosleep(&interval, NULL);
            }
        }
    }
    return outcome;
}

OS_MEMMAP DWORD WaitForSingleObjectEx(HANDLE handle, DWORD milliseconds, BOOL alertable)
{
    (void)alertable;
    return WaitForSingleObject(handle, milliseconds);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

/* Same 1 ms poll as WaitForSingleObject; the first signalled handle wins. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static DWORD wait_any(DWORD count,
                                const HANDLE *observed,
                                DWORD milliseconds,
                                uint64_t end)
{
    DWORD result = WAIT_TIMEOUT;
    int waiting = 1;
    while (waiting != 0) {
        DWORD index;
        for (index = 0u; (index < count) && (waiting != 0); ++index) {
            const DWORD waited = WaitForSingleObject(observed[index], 0u);
            if (waited != WAIT_TIMEOUT) {
                result = (waited == WAIT_OBJECT_0) ? (WAIT_OBJECT_0 + index) : WAIT_FAILED;
                waiting = 0;
            }
        }
        if ((waiting != 0) && (milliseconds != INFINITE) && (monotonic_ms() >= end)) {
            waiting = 0;
        } else if (waiting != 0) {
            const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
            (void)nanosleep(&interval, NULL);
        } else {
            /* A handle was signalled or failed. */
        }
    }
    return result;
}

OS_MEMMAP static DWORD wait_all(DWORD count,
                                const HANDLE *observed,
                                DWORD milliseconds,
                                uint64_t end)
{
    DWORD result = WAIT_OBJECT_0;
    DWORD index;
    for (index = 0u; (index < count) && (result == WAIT_OBJECT_0); ++index) {
        const uint64_t now = monotonic_ms();
        const DWORD remaining =
            (milliseconds == INFINITE) ? INFINITE : ((end > now) ? (DWORD)(end - now) : 0u);
        if (WaitForSingleObject(observed[index], remaining) != WAIT_OBJECT_0) {
            result = WAIT_FAILED;
        }
    }
    return result;
}

OS_MEMMAP DWORD WaitForMultipleObjects(DWORD count,
                                       const HANDLE *observed,
                                       BOOL all,
                                       DWORD milliseconds)
{
    DWORD result = WAIT_FAILED;
    uint64_t end = monotonic_ms();
    if ((observed != NULL) && (count != 0u) && (count <= OS_HOST_HANDLES)) {
        if (milliseconds != INFINITE) {
            end += milliseconds;
        }
        result = (all == 0) ? wait_any(count, observed, milliseconds, end)
                            : wait_all(count, observed, milliseconds, end);
    }
    return result;
}

OS_MEMMAP BOOL ReleaseMutex(HANDLE mutex)
{
    int released = 0;
    if ((mutex == NULL) || (mutex->kind != OS_HOST_MUTEX) || (held_depth == 0u) ||
        (__atomic_load_n(&mutex->mutex_owner, __ATOMIC_ACQUIRE) != GetCurrentThreadId())) {
        return FALSE;
    }
    if (mutex->mutex_depth > 1u) {
        --mutex->mutex_depth;
    } else {
        __atomic_store_n(&mutex->mutex_owner, 0u, __ATOMIC_RELEASE);
        released = pthread_mutex_unlock(&mutex->mutex);
    }
    if (released == 0) {
        --held_depth;
        (void)pthread_sigmask(SIG_SETMASK, &held_masks[held_depth], NULL);
    }
    return released == 0;
}

OS_MEMMAP static BOOL close_thread(HANDLE thread)
{
    BOOL closed = TRUE;
    if (thread->register_host != 0) {
        /* Release pthread resources without discarding the registered stack identity. */
        if ((InterlockedExchange(&thread->joined, 1) == 0) &&
            (pthread_detach(thread->thread) != 0)) {
            closed = FALSE;
        }
    } else if ((InterlockedCompareExchange(&thread->ended, 0, 0) != 0) &&
               (InterlockedCompareExchange(&thread->joined, 0, 0) != 0)) {
        if (munmap(thread->mapping, thread->mapping_size) != 0) {
            closed = FALSE;
        } else {
            thread->mapping = NULL;
            (void)InterlockedExchange(&thread->reserved, 0);
        }
    } else {
        /* Closing an observer does not stop its live actor. */
    }
    return closed;
}

OS_MEMMAP BOOL CloseHandle(HANDLE handle)
{
    BOOL closed = FALSE;
    if ((handle != NULL) && (handle != &process_handle)) {
        if ((handle->kind == OS_HOST_EVENT) && (handle->fd >= 0)) {
            const int result = close(handle->fd);
            if (result == 0) {
                handle->fd = -1;
                (void)InterlockedExchange(&handle->reserved, 0);
            }
            closed = (result == 0) ? TRUE : FALSE;
        } else if (handle->kind == OS_HOST_MUTEX) {
            closed = (pthread_mutex_destroy(&handle->mutex) == 0) ? TRUE : FALSE;
        } else if (handle->kind == OS_HOST_THREAD) {
            closed = close_thread(handle);
        } else {
            /* Unsupported handle kinds cannot be closed. */
        }
    }
    return closed;
}

OS_MEMMAP DWORD SuspendThread(HANDLE thread)
{
    int32_t generation;
    uint64_t end;
    if (thread == NULL || thread->kind != OS_HOST_THREAD ||
        __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0) {
        return UINT32_MAX;
    }
    if (__atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE) !=
        __atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE)) {
        return 1u;
    }
    generation = __atomic_add_fetch(&thread->requested, 1, __ATOMIC_ACQ_REL);
    if (pthread_kill(thread->thread, SIGRTMIN) != 0) {
        return UINT32_MAX;
    }
    end = monotonic_ms() + 2000u;
    while (__atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) < generation) {
        if (monotonic_ms() >= end || __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0) {
            return UINT32_MAX;
        }
        (void)futex_wait(&thread->acknowledged, generation - 1);
    }
    return 0u;
}

OS_MEMMAP DWORD ResumeThread(HANDLE thread)
{
    int32_t generation;
    if (thread == NULL || thread->kind != OS_HOST_THREAD) {
        return UINT32_MAX;
    }
    generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
    if ((generation == 0) || (generation == INT32_MAX) ||
        (InterlockedCompareExchange(&thread->acknowledged, 0, 0) < generation)) {
        return UINT32_MAX;
    }
    __atomic_store_n(&thread->resumed, generation, __ATOMIC_RELEASE);
    futex_wake(&thread->resumed);
    return 1u;
}

OS_MEMMAP BOOL GetThreadContext(HANDLE thread, CONTEXT *context)
{
    int32_t generation;
    if (thread == NULL || context == NULL || thread->kind != OS_HOST_THREAD) {
        return FALSE;
    }
    generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
    if (__atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) != INT32_MAX &&
        (generation == 0 ||
         __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) < generation)) {
        return FALSE;
    }
    Os_LinuxCaptureContext(context, &thread->captured.native);
    return TRUE;
}

OS_MEMMAP DWORD GetCurrentThreadId(void) { return Os_LinuxThreadId(); }

OS_MEMMAP DWORD GetThreadId(HANDLE thread)
{
    return thread != NULL ? __atomic_load_n(&thread->thread_id, __ATOMIC_ACQUIRE) : 0u;
}

OS_MEMMAP uint64_t GetTickCount64(void) { return monotonic_ms(); }

OS_MEMMAP DWORD GetTickCount(void) { return (DWORD)monotonic_ms(); }

OS_MEMMAP void Sleep(DWORD milliseconds)
{
    struct timespec pause_time;
    if (milliseconds == INFINITE) {
        for (;;) {
            (void)pause();
        }
    }
    pause_time.tv_sec = (time_t)(milliseconds / 1000u);
    pause_time.tv_nsec = (long)(milliseconds % 1000u) * 1000000L;
    while (nanosleep(&pause_time, &pause_time) != 0 && errno == EINTR) {
    }
}

OS_MEMMAP void ExitProcess(DWORD status) { _exit((int)status); }

OS_MEMMAP HANDLE Os_HostCurrentThread(void)
{
    HANDLE thread;
    if (current_thread != NULL) {
        return current_thread;
    }
    thread = reserve_handle(OS_HOST_THREAD);
    if (thread != NULL) {
        thread->thread = pthread_self();
        thread->thread_id = GetCurrentThreadId();
        current_thread = thread;
    }
    return thread;
}

OS_MEMMAP HANDLE GetCurrentProcess(void) { return &process_handle; }

OS_MEMMAP HANDLE GetCurrentThread(void) { return Os_HostCurrentThread(); }

OS_MEMMAP BOOL DuplicateHandle(HANDLE source_process,
                               HANDLE source_thread,
                               HANDLE target_process,
                               HANDLE *copy,
                               DWORD access,
                               BOOL inherit,
                               DWORD options)
{
    (void)access;
    (void)inherit;
    (void)options;
    if (copy == NULL || source_process != &process_handle || target_process != &process_handle ||
        source_thread == NULL || source_thread->kind != OS_HOST_THREAD) {
        return FALSE;
    }
    *copy = source_thread;
    return TRUE;
}

OS_MEMMAP BOOL GetExitCodeThread(HANDLE thread, DWORD *code)
{
    if (thread == NULL || code == NULL || thread->kind != OS_HOST_THREAD) {
        return FALSE;
    }
    *code =
        __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0 ? thread->exit_code : STILL_ACTIVE;
    return TRUE;
}

OS_MEMMAP HANDLE Os_HostDispatcherThread(void)
{
    return __atomic_load_n(&dispatcher, __ATOMIC_ACQUIRE);
}

OS_MEMMAP void Os_HostSetDispatcher(HANDLE thread)
{
    __atomic_store_n(&dispatcher, thread, __ATOMIC_RELEASE);
}

OS_MEMMAP int Os_HostStackRange(HANDLE thread, uintptr_t *low, uintptr_t *high, size_t *guard)
{
    if (thread == NULL || thread->mapping == NULL || low == NULL || high == NULL || guard == NULL) {
        return 0;
    }
    *low = (uintptr_t)thread->mapping;
    *high = *low + thread->mapping_size;
    *guard = thread->guard_size;
    return 1;
}

OS_MEMMAP int Os_HostCurrentStackRange(uintptr_t *low, uintptr_t *high, size_t *guard)
{
    if (current_thread != NULL && current_thread->mapping != NULL) {
        return Os_HostStackRange(current_thread, low, high, guard);
    }
    if ((low == NULL) || (high == NULL) || (guard == NULL)) {
        return 0;
    }
    return Os_LinuxMappedStack(low, high, guard);
}

OS_MEMMAP int Os_HostStopActor(HANDLE thread, DWORD timeout_ms)
{
    const uint64_t end = monotonic_ms() + timeout_ms;
    int stopped = 0;
    if ((thread != NULL) && (thread->kind == OS_HOST_THREAD)) {
        (void)InterlockedExchange(&thread->requested, INT32_MAX);
        futex_wake(&thread->resumed);
        if (pthread_kill(thread->thread, SIGRTMIN + 1) == 0) {
            while ((InterlockedCompareExchange(&thread->acknowledged, 0, 0) != INT32_MAX) &&
                   (monotonic_ms() < end)) {
                (void)futex_wait(&thread->acknowledged,
                                 InterlockedCompareExchange(&thread->acknowledged, 0, 0));
            }
            stopped =
                (InterlockedCompareExchange(&thread->acknowledged, 0, 0) == INT32_MAX) ? 1 : 0;
        }
    }
    return stopped;
}
OS_MEMMAP static int write_bytes(int descriptor, const char *data, size_t length)
{
    size_t offset = 0u;
    int result = 1;
    while ((offset < length) && (result != 0)) {
        ssize_t count;
        errno = 0;
        count = write(descriptor, &data[offset], length - offset);
        if (count > 0) {
            offset += (size_t)count;
        } else if ((count < 0) && (errno == EINTR)) {
            /* No bytes transferred; retry the same suffix. */
        } else {
            result = 0;
        }
    }
    return result;
}

OS_MEMMAP int Os_HostWriteBytes(int error_stream, const char *data, size_t length)
{
    sigset_t blocked;
    sigset_t previous;
    sigset_t pending;
    int result = 0;
    const struct timespec immediate = {0, 0};
    if ((error_stream == 0) || (error_stream == 1)) {
        /* Convert this call's broken pipe into an error without changing the
         * process disposition or consuming a signal already owned by the caller. */
        int prepared;
        errno = 0;
        prepared = sigemptyset(&blocked);
        if (prepared == 0) {
            errno = 0;
            prepared = sigaddset(&blocked, SIGPIPE);
        }
        if (prepared == 0) {
            prepared = pthread_sigmask(SIG_BLOCK, &blocked, &previous);
        }
        if (prepared == 0) {
            errno = 0;
            if (sigpending(&pending) == 0) {
                errno = 0;
                const int was_pending = sigismember(&pending, SIGPIPE);
                if (was_pending >= 0) {
                    errno = 0;
                    result = write_bytes(
                        (error_stream == 0) ? STDOUT_FILENO : STDERR_FILENO, data, length);
                    if ((result == 0) && (errno == EPIPE) && (was_pending == 0)) {
                        int signal_number;
                        do {
                            errno = 0;
                            signal_number = sigtimedwait(&blocked, NULL, &immediate);
                        } while ((signal_number < 0) && (errno == EINTR));
                        /* An ignored SIGPIPE creates no pending signal; the
                         * output result remains failure in either case. */
                    }
                }
            }
            if (pthread_sigmask(SIG_SETMASK, &previous, NULL) != 0) {
                result = 0;
            }
        }
    }
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
