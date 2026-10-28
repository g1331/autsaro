/** @file Linux host layer emulating the Win32 threads, events and signals the OS backend uses. */
#include "Os_HostLinux.h"
#include "Os_LinuxNative.h"
#include "Os_HostIo.h"
#include "Os_Stack.h"

#include <errno.h>
#include <linux/futex.h>
#include <poll.h>
/* MR12 RULE 21.5 VIOLATION: The fixed Linux port retains signal-frame parking and terminal
 * stop with generation ACKs, thread masks and pending-SIGPIPE preservation; see ADR 0024. */
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
/* MR12 RULE 21.10 VIOLATION: The fixed Linux port retains monotonic deadlines, EINTR remainder
 * handling and zero-time SIGPIPE waits; pthread.h also includes time.h. See ADR 0024. */
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
    unsigned char *mapping;
    size_t mapping_size;
    size_t guard_size;
    int register_host;
    volatile int32_t requested;
    volatile int32_t acknowledged;
    volatile int32_t resumed;
    volatile int32_t ended;
    volatile int32_t joined; /**< 0 unclaimed, 2 join/detach in progress, 1 completed. */
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
    host_handles[OS_HOST_HANDLES]; /**< Atomically reserved slots, per-handle
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
OS_MEMMAP static void futex_wait(const volatile int32_t *cell, int32_t expected)
{
    const struct timespec interval = {.tv_sec = 0, .tv_nsec = OS_HOST_WAIT_SLICE_NS};
    const int result = Os_LinuxFutexWait(cell, expected, &interval);
    if ((result != 0) && (result != -EAGAIN) && (result != -EINTR) && (result != -ETIMEDOUT)) {
        ExitProcess(1u);
    }
}

OS_MEMMAP static void futex_wake(const volatile int32_t *cell) { Os_LinuxFutexWake(cell); }

OS_MEMMAP static uint64_t monotonic_ms(void)
{
    struct timespec now;
    errno = 0;
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
        if (__atomic_compare_exchange_n(&host_handles[index].reserved,
                                        &expected,
                                        1,
                                        false,
                                        __ATOMIC_SEQ_CST,
                                        __ATOMIC_SEQ_CST)) {
            host_handles[index].kind = kind;
            host_handles[index].fd = -1;
            return &host_handles[index];
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
        futex_wait(&thread->resumed, current);
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
        if (generation != __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE)) {
            Os_LinuxCaptureContext(&thread->captured, native_context);
            __atomic_store_n(&thread->acknowledged, generation, __ATOMIC_RELEASE);
            futex_wake(&thread->acknowledged);
            for (;;) {
                const int32_t resumed = __atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE);
                /* Stop an already parked actor without nesting another signal handler. */
                if (InterlockedCompareExchange(&thread->requested, 0, 0) == INT32_MAX) {
                    terminal_park(thread);
                }
                if (resumed == generation) {
                    break;
                }
                futex_wait(&thread->resumed, resumed);
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
    const int suspend_signal = SIGRTMIN;
    const unsigned signal_flags = (unsigned)SA_SIGINFO | (unsigned)SA_ONSTACK;
    handler.sa_flags = (int)signal_flags;
    errno = 0;
    (void)sigemptyset(&handler.sa_mask);
    errno = 0;
    (void)sigaddset(&handler.sa_mask, suspend_signal);
    handler.sa_sigaction = &park_signal;
    errno = 0;
    if (sigaddset(&handler.sa_mask, suspend_signal + 1) != 0) {
        return 0;
    }
    errno = 0;
    if (sigaction(suspend_signal, &handler, NULL) != 0) {
        return 0;
    }
    errno = 0;
    (void)sigfillset(&handler.sa_mask);
    handler.sa_sigaction = &fault_stop_signal;
    errno = 0;
    return (sigaction(suspend_signal + 1, &handler, NULL) == 0) ? 1 : 0;
}

OS_MEMMAP static DWORD actor_body(void *argument)
{
    HANDLE thread = current_thread;
    (void)argument;
    OS_LINUX_STACK_FINISH(NULL, &thread->bootstrap_bottom, &thread->bootstrap_size);
    __atomic_store_n(&thread->thread_id, GetCurrentThreadId(), __ATOMIC_RELEASE);
    DWORD result;
    int registered = 1;
    const int register_host = thread->register_host;
    stack_t previous = {0};
    if (register_host != 0) {
        if (Os_LinuxAltStack(NULL, &previous) != 0) {
            ExitProcess(1u);
        }
        registered = Os_StackRegister('H');
    }
    if (registered == 0) {
        ExitProcess(1u);
    } else {
        result = thread->entry(thread->argument);
    }
    /* Hand the original thread stack back to its owner before pthread retirement. */
    if (register_host != 0) {
        if (Os_LinuxAltStack(&previous, NULL) != 0) {
            ExitProcess(1u);
        }
    }
    /* Finite helper actors destroy their execution fiber before pthread retirement. */
    OS_LINUX_STACK_START(NULL, thread->bootstrap_bottom, thread->bootstrap_size);
    return result;
}

OS_MEMMAP static void *actor_start(void *argument)
{
    HANDLE thread = NULL;
    for (size_t index = 0u; index < OS_HOST_HANDLES; ++index) {
        if (argument == &host_handles[index]) {
            thread = &host_handles[index];
            break;
        }
    }
    if (thread == NULL) {
        ExitProcess(1u);
    }
    current_thread = thread;
    OS_LINUX_STACK_START(&thread->bootstrap_fake,
                         thread->mapping + thread->guard_size,
                         thread->mapping_size - thread->guard_size);
    thread->exit_code =
        Os_LinuxCallOnStack(&thread->mapping[thread->mapping_size], &actor_body, thread);
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
    if ((start == NULL) || ((flags & CREATE_SUSPENDED) != 0u) ||
        ((flags & ~(STACK_SIZE_PARAM_IS_A_RESERVATION | CREATE_SUSPENDED)) != 0u)) {
        return NULL;
    }
    errno = 0;
    page = sysconf(_SC_PAGESIZE);
    if ((page <= 0) || (stack > (SIZE_MAX - (size_t)page))) {
        return NULL;
    }
    reserve = (stack == 0u) ? OS_HOST_DEFAULT_STACK : stack;
    errno = 0;
    const long minimum = sysconf(_SC_THREAD_STACK_MIN);
    if ((minimum <= 0) || (reserve < ((size_t)minimum + (size_t)page))) {
        return NULL;
    }
    reserve = ((reserve + (size_t)page - 1u) / (size_t)page) * (size_t)page;
    thread = reserve_handle(OS_HOST_THREAD);
    if (thread == NULL) {
        return NULL;
    }
    errno = 0;
    const unsigned protection = (unsigned)PROT_READ | (unsigned)PROT_WRITE;
    const unsigned mapping_flags = (unsigned)MAP_PRIVATE | (unsigned)MAP_ANONYMOUS;
    /* MR12 RULE 11.5 VIOLATION: mmap returns page-aligned bytes within the reviewed mapping. */
    thread->mapping = mmap(NULL, reserve, (int)protection, (int)mapping_flags, -1, 0);
    if (thread->mapping == MAP_FAILED) {
        thread->mapping = NULL;
        (void)InterlockedExchange(&thread->reserved, 0);
        return NULL;
    }
    errno = 0;
    if (mprotect(thread->mapping, (size_t)page, PROT_NONE) != 0) {
        errno = 0;
        /* Failed rollback must not discard ownership of a live mapping. */
        if (munmap(thread->mapping, reserve) != 0) {
            ExitProcess(1u);
        }
        thread->mapping = NULL;
        (void)InterlockedExchange(&thread->reserved, 0);
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
        /* clone3 sizes the hardware shadow stack from this pthread reserve. */
        status = pthread_attr_setstacksize(
            &attributes, (reserve > OS_HOST_DEFAULT_STACK) ? reserve : OS_HOST_DEFAULT_STACK);
        if (status == 0) {
            status = pthread_create(&thread->thread, &attributes, &actor_start, thread);
        }
        (void)pthread_attr_destroy(&attributes);
    }
    if (status != 0) {
        errno = 0;
        if (munmap(thread->mapping, reserve) != 0) {
            ExitProcess(1u);
        }
        thread->mapping = NULL;
        (void)InterlockedExchange(&thread->reserved, 0);
        return NULL;
    }
    if (id != NULL) {
        const uint64_t end = monotonic_ms() + 2000u;
        while (GetThreadId(thread) == 0u) {
            if (monotonic_ms() >= end) {
                break;
            }
            const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
            errno = 0;
            (void)nanosleep(&interval, NULL);
        }
        if (GetThreadId(thread) == 0u) {
            /* The actor owns its callback context and may already be running. */
            ExitProcess(1u);
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
    if ((attributes != NULL) || (name != NULL)) {
        return NULL;
    }
    event = reserve_handle(OS_HOST_EVENT);
    if (event == NULL) {
        return NULL;
    }
    event->manual = manual != 0;
    errno = 0;
    const unsigned event_flags = (unsigned)EFD_CLOEXEC | (unsigned)EFD_NONBLOCK;
    event->fd = eventfd((initial != 0) ? 1u : 0u, (int)event_flags);
    if (event->fd < 0) {
        (void)InterlockedExchange(&event->reserved, 0);
        return NULL;
    }
    return event;
}

OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event)
{
    const uint64_t signal_value = 1u;
    ssize_t written;
    if ((event == NULL) || (event->kind != OS_HOST_EVENT) || (event->fd < 0)) {
        return 0;
    }
    do {
        errno = 0;
        written = write(event->fd, &signal_value, sizeof(signal_value));
    } while ((written < 0) && (errno == EINTR));
    return (written == (ssize_t)sizeof(signal_value)) ? 1 : 0;
}

OS_MEMMAP static int read_event(HANDLE event)
{
    uint64_t value;
    ssize_t received;
    do {
        errno = 0;
        received = read(event->fd, &value, sizeof(value));
    } while ((received < 0) && (errno == EINTR));
    /* Only a failed nonblocking read can establish that the event is empty. */
    return (received == (ssize_t)sizeof(value))    ? 1
           : ((received < 0) && (errno == EAGAIN)) ? 0
                                                   : -1;
}

OS_MEMMAP BOOL ResetEvent(HANDLE event)
{
    int received;
    if ((event == NULL) || (event->kind != OS_HOST_EVENT) || (event->fd < 0)) {
        return 0;
    }
    do {
        received = read_event(event);
    } while (received > 0);
    return (received == 0) ? 1 : 0;
}

OS_MEMMAP HANDLE CreateMutexA(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name)
{
    HANDLE mutex = NULL;
    if ((attributes == NULL) && (name == NULL)) {
        mutex = reserve_handle(OS_HOST_MUTEX);
        if (mutex != NULL) {
            if (pthread_mutex_init(&mutex->mutex, NULL) != 0) {
                (void)InterlockedExchange(&mutex->reserved, 0);
                mutex = NULL;
            } else if (owner != 0) {
                if (WaitForSingleObject(mutex, 0u) != WAIT_OBJECT_0) {
                    (void)pthread_mutex_destroy(&mutex->mutex);
                    (void)InterlockedExchange(&mutex->reserved, 0);
                    mutex = NULL;
                }
            } else {
                /* Leave the mutex initially unowned. */
            }
        }
    }
    return mutex;
}

OS_MEMMAP static DWORD try_wait_mutex(HANDLE handle)
{
    DWORD outcome = WAIT_FAILED;
    sigset_t blocked;
    sigset_t previous;
    const int suspend_signal = SIGRTMIN;
    errno = 0;
    (void)sigemptyset(&blocked);
    errno = 0;
    (void)sigaddset(&blocked, suspend_signal);
    if (held_depth < 32u) {
        if (pthread_sigmask(SIG_BLOCK, &blocked, &previous) == 0) {
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
                held_masks[held_depth] = previous;
                ++held_depth;
                outcome = WAIT_OBJECT_0;
            } else {
                (void)pthread_sigmask(SIG_SETMASK, &previous, NULL);
                outcome = (locked == EBUSY) ? WAIT_TIMEOUT : WAIT_FAILED;
            }
        }
    }
    return outcome;
}

OS_MEMMAP static DWORD wait_event(HANDLE event)
{
    struct pollfd observed = {.fd = event->fd, .events = POLLIN, .revents = 0};
    int ready;
    do {
        errno = 0;
        ready = poll(&observed, 1u, 0);
    } while ((ready < 0) && (errno == EINTR));
    DWORD outcome = WAIT_FAILED;
    if ((ready >= 0) && (((unsigned)observed.revents &
                          ((unsigned)POLLERR | (unsigned)POLLHUP | (unsigned)POLLNVAL)) == 0u)) {
        outcome = WAIT_TIMEOUT;
        if ((ready > 0) && (((unsigned)observed.revents & (unsigned)POLLIN) != 0u)) {
            const int received = (event->manual != 0) ? 1 : read_event(event);
            outcome = (received > 0) ? WAIT_OBJECT_0 : (received == 0) ? WAIT_TIMEOUT : WAIT_FAILED;
        }
    }
    return outcome;
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
                outcome = wait_event(handle);
                pending = (outcome == WAIT_TIMEOUT) ? 1 : 0;
            } else if (handle->kind == OS_HOST_MUTEX) {
                outcome = try_wait_mutex(handle);
                pending = (outcome == WAIT_TIMEOUT) ? 1 : 0;
            } else if (handle->kind == OS_HOST_THREAD) {
                if (__atomic_load_n(&handle->ended, __ATOMIC_ACQUIRE) != 0) {
                    const int32_t previous = InterlockedCompareExchange(&handle->joined, 2, 0);
                    if (previous == 1) {
                        outcome = WAIT_OBJECT_0;
                        pending = 0;
                    } else if (previous == 0) {
                        if (pthread_join(handle->thread, NULL) == 0) {
                            (void)InterlockedExchange(&handle->joined, 1);
                            outcome = WAIT_OBJECT_0;
                        } else {
                            (void)InterlockedExchange(&handle->joined, 0);
                        }
                        pending = 0;
                    } else {
                        /* Another waiter owns the join; completion is not yet published. */
                    }
                }
            } else {
                pending = 0;
            }
            if ((pending != 0) && (milliseconds != INFINITE)) {
                const uint64_t now = monotonic_ms();
                if (now == UINT64_MAX) {
                    outcome = WAIT_FAILED;
                    pending = 0;
                } else if (now >= end) {
                    outcome = WAIT_TIMEOUT;
                    pending = 0;
                } else {
                    /* The finite deadline has not expired. */
                }
            }
            if (pending != 0) {
                const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
                errno = 0;
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

/* Same 1 ms poll as WaitForSingleObject; the first signalled handle wins. */
OS_MEMMAP static DWORD wait_any(DWORD count,
                                const HANDLE *observed,
                                DWORD milliseconds,
                                uint64_t end)
{
    DWORD result = WAIT_TIMEOUT;
    int waiting = 1;
    while (waiting != 0) {
        DWORD index;
        index = 0u;
        while ((index < count) && (waiting != 0)) {
            const DWORD waited = WaitForSingleObject(observed[index], 0u);
            if (waited != WAIT_TIMEOUT) {
                result = (waited == WAIT_OBJECT_0) ? (WAIT_OBJECT_0 + index) : WAIT_FAILED;
                waiting = 0;
            }
            ++index;
        }
        if ((waiting != 0) && (milliseconds != INFINITE)) {
            const uint64_t now = monotonic_ms();
            if (now == UINT64_MAX) {
                result = WAIT_FAILED;
                waiting = 0;
            } else if (now >= end) {
                waiting = 0;
            } else {
                /* The finite deadline has not expired. */
            }
        }
        if (waiting != 0) {
            const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
            errno = 0;
            (void)nanosleep(&interval, NULL);
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
    index = 0u;
    while ((index < count) && (result == WAIT_OBJECT_0)) {
        const uint64_t now = monotonic_ms();
        if (now == UINT64_MAX) {
            result = WAIT_FAILED;
        } else {
            const DWORD remaining =
                (milliseconds == INFINITE) ? INFINITE : ((end > now) ? (DWORD)(end - now) : 0u);
            result = WaitForSingleObject(observed[index], remaining);
        }
        ++index;
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
    if ((observed != NULL) && (count != 0u) && (count <= OS_HOST_HANDLES) && (end != UINT64_MAX)) {
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
    int released = EINVAL;
    if ((mutex != NULL) && (mutex->kind == OS_HOST_MUTEX) && (held_depth != 0u)) {
        const DWORD owner = __atomic_load_n(&mutex->mutex_owner, __ATOMIC_ACQUIRE);
        if (owner == GetCurrentThreadId()) {
            released = 0;
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
        }
    }
    return (released == 0) ? 1 : 0;
}

OS_MEMMAP static BOOL close_thread(HANDLE thread)
{
    BOOL closed = 1;
    if (thread->register_host != 0) {
        /* Release pthread resources without discarding the registered stack identity. */
        const int32_t previous = InterlockedCompareExchange(&thread->joined, 2, 0);
        if (previous == 0) {
            if (pthread_detach(thread->thread) != 0) {
                (void)InterlockedExchange(&thread->joined, 0);
                closed = 0;
            } else {
                (void)InterlockedExchange(&thread->joined, 1);
            }
        } else if (previous != 1) {
            closed = 0;
        } else {
            /* A completed join or detach already released the pthread resource. */
        }
    } else {
        /* A joinable observer retains its owner until a successful wait and close. */
        closed = 0;
        if (InterlockedCompareExchange(&thread->ended, 0, 0) != 0) {
            if (InterlockedCompareExchange(&thread->joined, 0, 0) == 1) {
                errno = 0;
                if (munmap(thread->mapping, thread->mapping_size) != 0) {
                    closed = 0;
                } else {
                    thread->mapping = NULL;
                    (void)InterlockedExchange(&thread->reserved, 0);
                    closed = 1;
                }
            }
        }
    }
    return closed;
}

OS_MEMMAP BOOL CloseHandle(HANDLE handle)
{
    BOOL closed = 0;
    if ((handle != NULL) && (handle != &process_handle)) {
        if ((handle->kind == OS_HOST_EVENT) && (handle->fd >= 0)) {
            errno = 0;
            const int result = close(handle->fd);
            /* Linux releases the descriptor even on EINTR; it must never be closed again. */
            handle->fd = -1;
            (void)InterlockedExchange(&handle->reserved, 0);
            closed = (result == 0) ? 1 : 0;
        } else if (handle->kind == OS_HOST_MUTEX) {
            closed = (pthread_mutex_destroy(&handle->mutex) == 0) ? 1 : 0;
            if (closed != 0) {
                handle->kind = OS_HOST_UNUSED;
                (void)InterlockedExchange(&handle->reserved, 0);
            }
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
    DWORD result = UINT32_MAX;
    bool active = false;
    if ((thread != NULL) && (thread->kind == OS_HOST_THREAD)) {
        active = __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) == 0;
    }
    if (active) {
        int32_t previous = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
        if (previous != INT32_MAX) {
            if (previous != __atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE)) {
                result = 1u;
            } else {
                /* Reserve INT32_MAX for stop; CAS preserves a concurrent stop request. */
                const int32_t generation = (previous == (INT32_MAX - 1)) ? 1 : (previous + 1);
                if (__atomic_compare_exchange_n(&thread->requested,
                                                &previous,
                                                generation,
                                                false,
                                                __ATOMIC_ACQ_REL,
                                                __ATOMIC_ACQUIRE)) {
                    if (pthread_kill(thread->thread, SIGRTMIN) == 0) {
                        const uint64_t end = monotonic_ms() + 2000u;
                        int32_t acknowledged =
                            __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE);
                        while ((acknowledged != generation) && (acknowledged != INT32_MAX)) {
                            bool failed = monotonic_ms() >= end;
                            if (!failed) {
                                failed = __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0;
                            }
                            if (failed) {
                                break;
                            }
                            futex_wait(&thread->acknowledged, acknowledged);
                            acknowledged = __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE);
                        }
                        if (acknowledged == generation) {
                            result = 0u;
                        }
                    }
                }
            }
        }
    }
    return result;
}

OS_MEMMAP DWORD ResumeThread(HANDLE thread)
{
    DWORD result = UINT32_MAX;
    if ((thread != NULL) && (thread->kind == OS_HOST_THREAD)) {
        const int32_t generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
        if ((generation != 0) && (generation != INT32_MAX)) {
            const int32_t acknowledged = InterlockedCompareExchange(&thread->acknowledged, 0, 0);
            if (acknowledged == generation) {
                __atomic_store_n(&thread->resumed, generation, __ATOMIC_RELEASE);
                futex_wake(&thread->resumed);
                result = 1u;
            }
        }
    }
    return result;
}

OS_MEMMAP BOOL GetThreadContext(HANDLE thread, CONTEXT *context)
{
    BOOL result = 0;
    if ((thread != NULL) && (context != NULL) && (thread->kind == OS_HOST_THREAD)) {
        const int32_t generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
        bool allowed = __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) == INT32_MAX;
        if ((!allowed) && (generation != 0)) {
            allowed = __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) == generation;
        }
        if (allowed) {
            Os_LinuxCaptureContext(context, &thread->captured.native);
            result = 1;
        }
    }
    return result;
}

OS_MEMMAP DWORD GetCurrentThreadId(void) { return Os_LinuxThreadId(); }

OS_MEMMAP DWORD GetThreadId(HANDLE thread)
{
    return (thread != NULL) ? __atomic_load_n(&thread->thread_id, __ATOMIC_ACQUIRE) : 0u;
}

OS_MEMMAP uint64_t GetTickCount64(void) { return monotonic_ms(); }

OS_MEMMAP DWORD GetTickCount(void) { return (DWORD)monotonic_ms(); }

OS_MEMMAP void Sleep(DWORD milliseconds)
{
    struct timespec pause_time;
    if (milliseconds == INFINITE) {
        for (;;) {
            errno = 0;
            (void)pause();
        }
    }
    pause_time.tv_sec = (time_t)milliseconds / (time_t)1000;
    pause_time.tv_nsec = ((long)milliseconds % 1000L) * 1000000L;
    int status;
    do {
        errno = 0;
        status = nanosleep(&pause_time, &pause_time);
    } while ((status != 0) && (errno == EINTR));
}

OS_MEMMAP void ExitProcess(DWORD status) { _exit((int)status); }

OS_MEMMAP static HANDLE Os_HostCurrentThread(void)
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
    if ((copy == NULL) || (source_process != &process_handle) ||
        (target_process != &process_handle) || (source_thread == NULL) ||
        (source_thread->kind != OS_HOST_THREAD)) {
        return 0;
    }
    *copy = source_thread;
    return 1;
}

OS_MEMMAP BOOL GetExitCodeThread(HANDLE thread, DWORD *code)
{
    if ((thread == NULL) || (code == NULL) || (thread->kind != OS_HOST_THREAD)) {
        return 0;
    }
    *code =
        (__atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0) ? thread->exit_code : STILL_ACTIVE;
    return 1;
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
    if ((thread == NULL) || (thread->mapping == NULL) || (low == NULL) || (high == NULL) ||
        (guard == NULL)) {
        return 0;
    }
    /* MR12 RULE 11.4 VIOLATION: Full-width native bounds are compared, never dereferenced. */
    *low = (uintptr_t)thread->mapping;
    *high = *low + thread->mapping_size;
    *guard = thread->guard_size;
    return 1;
}

OS_MEMMAP int Os_HostCurrentStackRange(uintptr_t *low, uintptr_t *high, size_t *guard)
{
    if ((current_thread != NULL) && (current_thread->mapping != NULL)) {
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
            while (InterlockedCompareExchange(&thread->acknowledged, 0, 0) != INT32_MAX) {
                if (monotonic_ms() >= end) {
                    break;
                }
                futex_wait(&thread->acknowledged,
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
