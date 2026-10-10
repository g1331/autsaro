#ifndef AUTOSAR_OS_HOST_LINUX_H
#define AUTOSAR_OS_HOST_LINUX_H

#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <ucontext.h>

typedef int32_t LONG;
typedef uint32_t DWORD;
typedef uint32_t ULONG;
typedef uintptr_t ULONG_PTR;
typedef size_t SIZE_T;
typedef int BOOL;
typedef void *LPVOID;
typedef const char *LPCSTR;
typedef void *LPSECURITY_ATTRIBUTES;
typedef DWORD *LPDWORD;
typedef DWORD (*LPTHREAD_START_ROUTINE)(void *);
typedef struct Os_HostHandle *HANDLE;
typedef struct {
    ucontext_t native;
    uintptr_t Rsp;
    DWORD ContextFlags;
} CONTEXT;

typedef char Os_HostLongMustBe32Bits[(sizeof(LONG) == 4u) ? 1 : -1];
#if !defined(__GCC_ATOMIC_INT_LOCK_FREE) || (__GCC_ATOMIC_INT_LOCK_FREE != 2)
#error Linux host requires always-lock-free 32-bit atomics
#endif

#define WINAPI
#define CALLBACK
#define VOID void
#ifndef TRUE
#define TRUE 1
#endif
#ifndef FALSE
#define FALSE 0
#endif
#define INFINITE UINT32_MAX
#define WAIT_OBJECT_0 0u
#define WAIT_TIMEOUT 258u
#define WAIT_FAILED UINT32_MAX
#define WAIT_IO_COMPLETION 192u
#define STILL_ACTIVE 259u
#define CONTEXT_CONTROL 1u
#define CREATE_SUSPENDED 0x00000004u
#define STACK_SIZE_PARAM_IS_A_RESERVATION 0x00010000u
#define DUPLICATE_SAME_ACCESS 0x00000002u

static inline LONG InterlockedCompareExchange(volatile LONG *cell, LONG desired, LONG expected) {
    __atomic_compare_exchange_n(cell, &expected, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return expected;
}
static inline LONG InterlockedExchange(volatile LONG *cell, LONG value) {
    return __atomic_exchange_n(cell, value, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedOr(volatile LONG *cell, LONG mask) {
    return __atomic_fetch_or(cell, mask, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedAnd(volatile LONG *cell, LONG mask) {
    return __atomic_fetch_and(cell, mask, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedIncrement(volatile LONG *cell) {
    return __atomic_add_fetch(cell, 1, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedDecrement(volatile LONG *cell) {
    return __atomic_sub_fetch(cell, 1, __ATOMIC_SEQ_CST);
}

HANDLE Os_HostCurrentThread(void);
HANDLE Os_HostDispatcherThread(void);
void Os_HostSetDispatcher(HANDLE thread);
int Os_HostInstallSignals(void);
HANDLE Os_HostCreateActor(SIZE_T stack, LPTHREAD_START_ROUTINE start, LPVOID argument, DWORD flags,
                          LPDWORD id, int register_host);
int Os_HostStackRange(HANDLE thread, uintptr_t *low, uintptr_t *high, size_t *guard);
int Os_HostCurrentStackRange(uintptr_t *low, uintptr_t *high, size_t *guard);
int Os_HostStopActor(HANDLE thread, DWORD timeout_ms);

HANDLE CreateEventA(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name);
HANDLE CreateMutexA(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);
HANDLE CreateThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                    LPVOID argument, DWORD flags, LPDWORD id);
DWORD WaitForSingleObject(HANDLE handle, DWORD milliseconds);
DWORD WaitForSingleObjectEx(HANDLE handle, DWORD milliseconds, BOOL alertable);
DWORD WaitForMultipleObjects(DWORD count, const HANDLE *handles, BOOL all, DWORD milliseconds);
BOOL ResetEvent(HANDLE event);
BOOL Os_HostSetEvent(HANDLE event);
BOOL ReleaseMutex(HANDLE mutex);
BOOL CloseHandle(HANDLE handle);
DWORD SuspendThread(HANDLE thread);
DWORD ResumeThread(HANDLE thread);
BOOL GetThreadContext(HANDLE thread, CONTEXT *context);
DWORD GetThreadId(HANDLE thread);
DWORD GetCurrentThreadId(void);
DWORD GetTickCount(void);
uint64_t GetTickCount64(void);
void Sleep(DWORD milliseconds);
void ExitProcess(DWORD status) __attribute__((noreturn));
BOOL DuplicateHandle(HANDLE source_process, HANDLE source_thread, HANDLE target_process,
                     HANDLE *copy, DWORD access, BOOL inherit, DWORD options);
HANDLE GetCurrentProcess(void);
HANDLE GetCurrentThread(void);
BOOL GetExitCodeThread(HANDLE thread, DWORD *code);

#endif
