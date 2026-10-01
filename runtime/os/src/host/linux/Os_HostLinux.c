#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "Os_HostLinux.h"
#include "Os_Stack.h"

#include <errno.h>
#include <linux/futex.h>
#include <poll.h>
#include <signal.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define OS_HOST_HANDLES 96u
#define OS_HOST_DEFAULT_STACK 262144u
#define OS_HOST_WAIT_SLICE_NS 10000000L

typedef enum {
    OS_HOST_UNUSED = 0,
    OS_HOST_EVENT,
    OS_HOST_MUTEX,
    OS_HOST_THREAD,
    OS_HOST_PROCESS
} Os_HostKind;

struct Os_HostHandle {
    volatile int32_t reserved;
    Os_HostKind kind;
    int fd;
    int manual;
    pthread_mutex_t mutex;
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
};

static struct Os_HostHandle handles[OS_HOST_HANDLES];
static struct Os_HostHandle process_handle = {.reserved = 1, .kind = OS_HOST_PROCESS};
static HANDLE dispatcher;
static __thread HANDLE current_thread;
static __thread sigset_t held_masks[32];
static __thread unsigned held_depth;

static int futex_wait(volatile int32_t *cell, int32_t expected) {
    const struct timespec interval = {.tv_sec = 0, .tv_nsec = OS_HOST_WAIT_SLICE_NS};
    return (int)syscall(SYS_futex, cell, FUTEX_WAIT_PRIVATE, expected, &interval, NULL, 0);
}

static void futex_wake(volatile int32_t *cell) {
    (void)syscall(SYS_futex, cell, FUTEX_WAKE_PRIVATE, INT32_MAX, NULL, NULL, 0);
}

static uint64_t monotonic_ms(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        return UINT64_MAX;
    }
    return ((uint64_t)now.tv_sec * UINT64_C(1000)) + ((uint64_t)now.tv_nsec / UINT64_C(1000000));
}

static HANDLE reserve_handle(Os_HostKind kind) {
    size_t index;
    for (index = 0u; index < OS_HOST_HANDLES; ++index) {
        int32_t expected = 0;
        if (__atomic_compare_exchange_n(&handles[index].reserved, &expected, 1, 0, __ATOMIC_SEQ_CST,
                                        __ATOMIC_SEQ_CST)) {
            handles[index].kind = kind;
            handles[index].fd = -1;
            return &handles[index];
        }
    }
    return NULL;
}

static void capture_context(HANDLE thread, const ucontext_t *source) {
    thread->captured.native = *source;
    if (source->uc_mcontext.fpregs != NULL) {
        thread->captured.native.__fpregs_mem = *source->uc_mcontext.fpregs;
        thread->captured.native.uc_mcontext.fpregs = &thread->captured.native.__fpregs_mem;
    }
    thread->captured.Rsp = (uintptr_t)source->uc_mcontext.gregs[REG_RSP];
    thread->captured.ContextFlags = CONTEXT_CONTROL;
}

static void park_signal(int signal_number, siginfo_t *info, void *native_context) {
    HANDLE thread = current_thread;
    int32_t generation;
    (void)signal_number;
    (void)info;
    if (thread == NULL || thread->kind != OS_HOST_THREAD) {
        return;
    }
    generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
    if (generation <= __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE)) {
        return;
    }
    capture_context(thread, (const ucontext_t *)native_context);
    __atomic_store_n(&thread->acknowledged, generation, __ATOMIC_RELEASE);
    futex_wake(&thread->acknowledged);
    while (__atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE) < generation) {
        (void)futex_wait(&thread->resumed, generation - 1);
    }
}

static void fault_stop_signal(int signal_number, siginfo_t *info, void *native_context) {
    HANDLE thread = current_thread;
    (void)signal_number;
    (void)info;
    if (thread == NULL) {
        _exit(1);
    }
    capture_context(thread, (const ucontext_t *)native_context);
    __atomic_store_n(&thread->acknowledged, INT32_MAX, __ATOMIC_RELEASE);
    futex_wake(&thread->acknowledged);
    for (;;) {
        const int32_t current = __atomic_load_n(&thread->resumed, __ATOMIC_ACQUIRE);
        (void)futex_wait(&thread->resumed, current);
    }
}

int Os_HostInstallSignals(void) {
    struct sigaction handler = {0};
    handler.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&handler.sa_mask);
    sigaddset(&handler.sa_mask, SIGRTMIN);
    handler.sa_sigaction = park_signal;
    if (sigaction(SIGRTMIN, &handler, NULL) != 0) {
        return 0;
    }
    sigfillset(&handler.sa_mask);
    handler.sa_sigaction = fault_stop_signal;
    return sigaction(SIGRTMIN + 1, &handler, NULL) == 0;
}

static void *actor_start(void *argument) {
    HANDLE thread = argument;
    current_thread = thread;
    __atomic_store_n(&thread->thread_id, GetCurrentThreadId(), __ATOMIC_RELEASE);
    if ((thread->register_host != 0) && (Os_StackRegister('H') == 0)) {
        thread->exit_code = UINT32_MAX;
    } else {
        thread->exit_code = thread->entry(thread->argument);
    }
    __atomic_store_n(&thread->ended, 1, __ATOMIC_RELEASE);
    futex_wake(&thread->ended);
    return NULL;
}

HANDLE Os_HostCreateActor(SIZE_T stack, LPTHREAD_START_ROUTINE start, LPVOID argument, DWORD flags,
                          LPDWORD id, int register_host) {
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
    if (reserve < (size_t)PTHREAD_STACK_MIN + (size_t)page) {
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
        status = pthread_attr_setstack(&attributes, (char *)thread->mapping + page,
                                       reserve - (size_t)page);
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

HANDLE CreateThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                    LPVOID argument, DWORD flags, LPDWORD id) {
    if (attributes != NULL) {
        return NULL;
    }
    return Os_HostCreateActor(stack, start, argument, flags, id, 1);
}

HANDLE CreateEventA(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name) {
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

BOOL Os_HostSetEvent(HANDLE event) {
    const uint64_t signal_value = 1u;
    if (event == NULL || event->kind != OS_HOST_EVENT || event->fd < 0) {
        return FALSE;
    }
    return write(event->fd, &signal_value, sizeof(signal_value)) == (ssize_t)sizeof(signal_value);
}

BOOL ResetEvent(HANDLE event) {
    uint64_t value;
    if (event == NULL || event->kind != OS_HOST_EVENT || event->fd < 0) {
        return FALSE;
    }
    while (read(event->fd, &value, sizeof(value)) == sizeof(value)) {
    }
    return errno == EAGAIN;
}

HANDLE CreateMutexA(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name) {
    HANDLE mutex;
    pthread_mutexattr_t type;
    if (attributes != NULL || name != NULL) {
        return NULL;
    }
    mutex = reserve_handle(OS_HOST_MUTEX);
    if (mutex == NULL) {
        return NULL;
    }
    if (pthread_mutexattr_init(&type) != 0) {
        mutex->reserved = 0;
        return NULL;
    }
    if (pthread_mutexattr_settype(&type, PTHREAD_MUTEX_RECURSIVE) != 0 ||
        pthread_mutex_init(&mutex->mutex, &type) != 0) {
        (void)pthread_mutexattr_destroy(&type);
        mutex->reserved = 0;
        return NULL;
    }
    (void)pthread_mutexattr_destroy(&type);
    if (owner != 0 && WaitForSingleObject(mutex, 0u) != WAIT_OBJECT_0) {
        (void)pthread_mutex_destroy(&mutex->mutex);
        mutex->reserved = 0;
        return NULL;
    }
    return mutex;
}

DWORD WaitForSingleObject(HANDLE handle, DWORD milliseconds) {
    uint64_t end = monotonic_ms();
    if (handle == NULL || end == UINT64_MAX) {
        return WAIT_FAILED;
    }
    if (milliseconds != INFINITE) {
        end += milliseconds;
    }
    for (;;) {
        if (handle->kind == OS_HOST_EVENT && handle->fd >= 0) {
            struct pollfd observed = {.fd = handle->fd, .events = POLLIN, .revents = 0};
            uint64_t value;
            int ready = poll(&observed, 1u, 0);
            if (ready > 0 && (observed.revents & POLLIN) != 0) {
                if (handle->manual != 0 ||
                    read(handle->fd, &value, sizeof(value)) == sizeof(value)) {
                    return WAIT_OBJECT_0;
                }
            }
        } else if (handle->kind == OS_HOST_MUTEX) {
            sigset_t blocked, previous;
            sigemptyset(&blocked);
            sigaddset(&blocked, SIGRTMIN);
            if (held_depth >= 32u || pthread_sigmask(SIG_BLOCK, &blocked, &previous) != 0) {
                return WAIT_FAILED;
            }
            if (pthread_mutex_trylock(&handle->mutex) == 0) {
                held_masks[held_depth++] = previous;
                return WAIT_OBJECT_0;
            }
            (void)pthread_sigmask(SIG_SETMASK, &previous, NULL);
        } else if (handle->kind == OS_HOST_THREAD) {
            if (__atomic_load_n(&handle->ended, __ATOMIC_ACQUIRE) != 0) {
                if (__atomic_exchange_n(&handle->joined, 1, __ATOMIC_ACQ_REL) == 0) {
                    if (pthread_join(handle->thread, NULL) != 0) {
                        return WAIT_FAILED;
                    }
                }
                return WAIT_OBJECT_0;
            }
        } else {
            return WAIT_FAILED;
        }
        if (milliseconds != INFINITE && monotonic_ms() >= end) {
            return WAIT_TIMEOUT;
        }
        {
            const struct timespec interval = {.tv_sec = 0, .tv_nsec = 1000000L};
            (void)nanosleep(&interval, NULL);
        }
    }
}

DWORD WaitForSingleObjectEx(HANDLE handle, DWORD milliseconds, BOOL alertable) {
    (void)alertable;
    return WaitForSingleObject(handle, milliseconds);
}

DWORD WaitForMultipleObjects(DWORD count, const HANDLE *observed, BOOL all, DWORD milliseconds) {
    DWORD index;
    uint64_t end = monotonic_ms();
    if (all == 0 || observed == NULL || count == 0u || count > OS_HOST_HANDLES) {
        return WAIT_FAILED;
    }
    if (milliseconds != INFINITE) {
        end += milliseconds;
    }
    for (index = 0u; index < count; ++index) {
        DWORD remaining = milliseconds == INFINITE
                              ? INFINITE
                              : (end > monotonic_ms() ? (DWORD)(end - monotonic_ms()) : 0u);
        if (WaitForSingleObject(observed[index], remaining) != WAIT_OBJECT_0) {
            return WAIT_FAILED;
        }
    }
    return WAIT_OBJECT_0;
}

BOOL ReleaseMutex(HANDLE mutex) {
    int released;
    if (mutex == NULL || mutex->kind != OS_HOST_MUTEX || held_depth == 0u) {
        return FALSE;
    }
    released = pthread_mutex_unlock(&mutex->mutex);
    if (released == 0) {
        --held_depth;
        (void)pthread_sigmask(SIG_SETMASK, &held_masks[held_depth], NULL);
    }
    return released == 0;
}

BOOL CloseHandle(HANDLE handle) {
    if (handle == NULL || handle == &process_handle) {
        return FALSE;
    }
    if (handle->kind == OS_HOST_EVENT && handle->fd >= 0) {
        int result = close(handle->fd);
        if (result == 0) {
            handle->fd = -1;
            __atomic_store_n(&handle->reserved, 0, __ATOMIC_RELEASE);
        }
        return result == 0;
    }
    if (handle->kind == OS_HOST_MUTEX) {
        return pthread_mutex_destroy(&handle->mutex) == 0;
    }
    if (handle->kind == OS_HOST_THREAD) {
        if ((handle->register_host == 0) &&
            (__atomic_load_n(&handle->ended, __ATOMIC_ACQUIRE) != 0) &&
            (__atomic_load_n(&handle->joined, __ATOMIC_ACQUIRE) != 0)) {
            if (munmap(handle->mapping, handle->mapping_size) != 0) {
                return FALSE;
            }
            handle->mapping = NULL;
            __atomic_store_n(&handle->reserved, 0, __ATOMIC_RELEASE);
        }
        /* A live actor continues after its observer closes. Automotive/host
         * registrations retain their stack identity until process exit. */
        return TRUE;
    }
    return FALSE;
}

DWORD SuspendThread(HANDLE thread) {
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

DWORD ResumeThread(HANDLE thread) {
    int32_t generation;
    if (thread == NULL || thread->kind != OS_HOST_THREAD) {
        return UINT32_MAX;
    }
    generation = __atomic_load_n(&thread->requested, __ATOMIC_ACQUIRE);
    if (generation == 0 || __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) < generation) {
        return UINT32_MAX;
    }
    __atomic_store_n(&thread->resumed, generation, __ATOMIC_RELEASE);
    futex_wake(&thread->resumed);
    return 1u;
}

BOOL GetThreadContext(HANDLE thread, CONTEXT *context) {
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
    *context = thread->captured;
    context->native.uc_mcontext.fpregs = &context->native.__fpregs_mem;
    return TRUE;
}

DWORD GetCurrentThreadId(void) {
    long id = syscall(SYS_gettid);
    return id > 0 && (uint64_t)id <= UINT32_MAX ? (DWORD)id : 0u;
}

DWORD GetThreadId(HANDLE thread) {
    return thread != NULL ? __atomic_load_n(&thread->thread_id, __ATOMIC_ACQUIRE) : 0u;
}

uint64_t GetTickCount64(void) { return monotonic_ms(); }

DWORD GetTickCount(void) { return (DWORD)monotonic_ms(); }

void Sleep(DWORD milliseconds) {
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

void ExitProcess(DWORD status) { _exit((int)status); }

HANDLE Os_HostCurrentThread(void) {
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

HANDLE GetCurrentProcess(void) { return &process_handle; }

HANDLE GetCurrentThread(void) { return Os_HostCurrentThread(); }

BOOL DuplicateHandle(HANDLE source_process, HANDLE source_thread, HANDLE target_process,
                     HANDLE *copy, DWORD access, BOOL inherit, DWORD options) {
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

BOOL GetExitCodeThread(HANDLE thread, DWORD *code) {
    if (thread == NULL || code == NULL || thread->kind != OS_HOST_THREAD) {
        return FALSE;
    }
    *code =
        __atomic_load_n(&thread->ended, __ATOMIC_ACQUIRE) != 0 ? thread->exit_code : STILL_ACTIVE;
    return TRUE;
}

HANDLE Os_HostDispatcherThread(void) { return __atomic_load_n(&dispatcher, __ATOMIC_ACQUIRE); }

void Os_HostSetDispatcher(HANDLE thread) {
    __atomic_store_n(&dispatcher, thread, __ATOMIC_RELEASE);
}

int Os_HostStackRange(HANDLE thread, uintptr_t *low, uintptr_t *high, size_t *guard) {
    if (thread == NULL || thread->mapping == NULL || low == NULL || high == NULL || guard == NULL) {
        return 0;
    }
    *low = (uintptr_t)thread->mapping;
    *high = *low + thread->mapping_size;
    *guard = thread->guard_size;
    return 1;
}

int Os_HostCurrentStackRange(uintptr_t *low, uintptr_t *high, size_t *guard) {
    pthread_attr_t attributes;
    void *base = NULL;
    size_t length = 0u;
    size_t guard_bytes = 0u;
    if (current_thread != NULL && current_thread->mapping != NULL) {
        return Os_HostStackRange(current_thread, low, high, guard);
    }
    if (low == NULL || high == NULL || guard == NULL ||
        pthread_getattr_np(pthread_self(), &attributes) != 0) {
        return 0;
    }
    if (pthread_attr_getstack(&attributes, &base, &length) != 0 ||
        pthread_attr_getguardsize(&attributes, &guard_bytes) != 0) {
        (void)pthread_attr_destroy(&attributes);
        return 0;
    }
    (void)pthread_attr_destroy(&attributes);
    *low = (uintptr_t)base;
    *high = *low + length;
    *guard = guard_bytes;
    return 1;
}

int Os_HostStopActor(HANDLE thread, DWORD timeout_ms) {
    const uint64_t end = monotonic_ms() + timeout_ms;
    if (thread == NULL || thread->kind != OS_HOST_THREAD ||
        pthread_kill(thread->thread, SIGRTMIN + 1) != 0) {
        return 0;
    }
    while (__atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE) != INT32_MAX) {
        if (monotonic_ms() >= end) {
            return 0;
        }
        (void)futex_wait(&thread->acknowledged,
                         __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE));
    }
    return 1;
}
