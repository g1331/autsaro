/** @file Linux native ABI aliases, constants and lock-free cell requirements. */
#ifndef AUTOSAR_OS_HOST_LINUX_TYPES_H
#define AUTOSAR_OS_HOST_LINUX_TYPES_H
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

#endif
