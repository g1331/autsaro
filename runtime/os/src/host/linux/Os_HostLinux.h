/** @file Private Linux host declarations: Win32-style types, atomics and thread/event calls. */
#ifndef AUTOSAR_OS_HOST_LINUX_H
#define AUTOSAR_OS_HOST_LINUX_H
#include "Os_Version.h"

#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <ucontext.h>

typedef int32_t LONG;
typedef uint32_t DWORD;
typedef size_t SIZE_T;
typedef int BOOL;
typedef void *LPVOID;
typedef const char *LPCSTR;
typedef void *LPSECURITY_ATTRIBUTES;
typedef DWORD *LPDWORD;
typedef DWORD (*LPTHREAD_START_ROUTINE)(void *argument);
typedef struct Os_HostHandleType *HANDLE;
typedef struct {
    ucontext_t native;
    uintptr_t Rsp;
    DWORD ContextFlags;
} CONTEXT;

enum {
    OS_HOST_LONG_WIDTH = sizeof(char[(sizeof(LONG) == 4u) ? 1 : -1]), /**< Reject incompatible
                                                                       Interlocked cells. */
};
#if !defined(__GCC_ATOMIC_INT_LOCK_FREE) || (__GCC_ATOMIC_INT_LOCK_FREE != 2)
#error Linux host requires always-lock-free 32-bit atomics
#endif

#define WINAPI
#define INFINITE                          UINT32_MAX
#define WAIT_OBJECT_0                     0u
#define WAIT_TIMEOUT                      258u
#define WAIT_FAILED                       UINT32_MAX
#define WAIT_IO_COMPLETION                192u
#define STILL_ACTIVE                      259u
#define CONTEXT_CONTROL                   1u
#define CREATE_SUSPENDED                  0x00000004u
#define STACK_SIZE_PARAM_IS_A_RESERVATION 0x00010000u
#define DUPLICATE_SAME_ACCESS             0x00000002u

static inline LONG InterlockedCompareExchange(volatile LONG *cell, LONG desired, LONG expected)
{
    (void)__atomic_compare_exchange_n(
        cell, &expected, desired, false, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return expected;
}
static inline LONG InterlockedExchange(volatile LONG *cell, LONG value)
{
    return __atomic_exchange_n(cell, value, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedOr(volatile LONG *cell, LONG mask)
{
    return __atomic_fetch_or(cell, mask, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedAnd(volatile LONG *cell, LONG mask)
{
    return __atomic_fetch_and(cell, mask, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedIncrement(volatile LONG *cell)
{
    return __atomic_add_fetch(cell, 1, __ATOMIC_SEQ_CST);
}
static inline LONG InterlockedDecrement(volatile LONG *cell)
{
    return __atomic_sub_fetch(cell, 1, __ATOMIC_SEQ_CST);
}

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP HANDLE Os_HostDispatcherThread(void);

OS_MEMMAP void Os_HostSetDispatcher(HANDLE thread);

OS_MEMMAP int Os_HostInstallSignals(void);

/** Recoverable creation failures return NULL; a failed mapping rollback terminates the process. */
OS_MEMMAP HANDLE Os_HostCreateActor(SIZE_T stack,
                                    LPTHREAD_START_ROUTINE start,
                                    LPVOID argument,
                                    DWORD flags,
                                    LPDWORD id,
                                    int register_host);

OS_MEMMAP int Os_HostStackRange(HANDLE thread, uintptr_t *low, uintptr_t *high, size_t *guard);

OS_MEMMAP int Os_HostCurrentStackRange(uintptr_t *low, uintptr_t *high, size_t *guard);

OS_MEMMAP int Os_HostStopActor(HANDLE thread, DWORD timeout_ms);

OS_MEMMAP HANDLE CreateEventA(LPSECURITY_ATTRIBUTES attributes,
                              BOOL manual,
                              BOOL initial,
                              LPCSTR name);

OS_MEMMAP HANDLE CreateMutexA(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);

OS_MEMMAP HANDLE CreateThread(LPSECURITY_ATTRIBUTES attributes,
                              SIZE_T stack,
                              LPTHREAD_START_ROUTINE start,
                              LPVOID argument,
                              DWORD flags,
                              LPDWORD id);

OS_MEMMAP DWORD WaitForSingleObject(HANDLE handle, DWORD milliseconds);

OS_MEMMAP DWORD WaitForSingleObjectEx(HANDLE handle, DWORD milliseconds, BOOL alertable);

OS_MEMMAP DWORD WaitForMultipleObjects(DWORD count,
                                       const HANDLE *observed,
                                       BOOL all,
                                       DWORD milliseconds);

OS_MEMMAP BOOL ResetEvent(HANDLE event);

OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event);

OS_MEMMAP BOOL ReleaseMutex(HANDLE mutex);

OS_MEMMAP BOOL CloseHandle(HANDLE handle);

OS_MEMMAP DWORD SuspendThread(HANDLE thread);

OS_MEMMAP DWORD ResumeThread(HANDLE thread);

OS_MEMMAP BOOL GetThreadContext(HANDLE thread, CONTEXT *context);

OS_MEMMAP DWORD GetThreadId(HANDLE thread);

OS_MEMMAP DWORD GetCurrentThreadId(void);

OS_MEMMAP DWORD GetTickCount(void);

OS_MEMMAP uint64_t GetTickCount64(void);

OS_MEMMAP void Sleep(DWORD milliseconds);

OS_MEMMAP void ExitProcess(DWORD status) __attribute__((noreturn));

OS_MEMMAP BOOL DuplicateHandle(HANDLE source_process,
                               HANDLE source_thread,
                               HANDLE target_process,
                               HANDLE *copy,
                               DWORD access,
                               BOOL inherit,
                               DWORD options);

OS_MEMMAP HANDLE GetCurrentProcess(void);

OS_MEMMAP HANDLE GetCurrentThread(void);

OS_MEMMAP BOOL GetExitCodeThread(HANDLE thread, DWORD *code);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostLinux.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostLinux.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif

#endif
