/** @file Native synchronization cells, handles and actor callback types. */
#ifndef AUTOSAR_OS_HOST_TYPES_H
#define AUTOSAR_OS_HOST_TYPES_H
#include "Os_Version.h"
#include <stdint.h>
#if defined(_WIN32)
#include "Os_Windows.h"
#elif defined(__linux__)
#include "Os_HostLinux.h"
#else
#error Unsupported native OS host
#endif

/** Native four-byte aligned Interlocked cell; long on Win64 and int32_t on Linux. */
typedef LONG Os_Atomic32Type;
/** Native eight-byte atomic storage; Linux requires always-lock-free operations. */
typedef uint64_t Os_Atomic64Type;
/* The type bounds reject incompatible native cells without declaring unused guard types. */
enum {
    OS_ATOMIC32_WIDTH = sizeof(
        char[(sizeof(Os_Atomic32Type) == sizeof(int32_t)) ? 1 : -1]), /**< Reject incompatible
                                                                         32-bit cell width. */
    OS_ATOMIC32_ALIGN = sizeof(char[(__alignof__(Os_Atomic32Type) >= __alignof__(int32_t))
                                        ? 1
                                        : -1]), /**< Reject under-aligned 32-bit cells. */
    OS_ATOMIC64_WIDTH =
        sizeof(char[(sizeof(Os_Atomic64Type) == 8u) ? 1 : -1]), /**< Reject incompatible 64-bit cell
                                                                   width. */
    OS_ATOMIC64_ALIGN =
        sizeof(char[(__alignof__(Os_Atomic64Type) >= 8u) ? 1 : -1]), /**< Reject under-aligned
                                                                        64-bit cells. */
};
#if defined(__linux__) && (__GCC_ATOMIC_LLONG_LOCK_FREE != 2)
#error Linux host requires always-lock-free 64-bit atomics
#endif

/** Owned event or thread-observation handle; NULL denotes creation failure. */
typedef HANDLE Os_HostHandleType;
/** Native thread identity, distinct from an automotive Task or ISR identifier. */
typedef DWORD Os_HostThreadIdType;
/** Native exit code compatible with the controlled platform thread ABI. */
typedef DWORD Os_HostThreadResultType;
/** Native callback borrowing its context until the actor has retired. */
typedef LPTHREAD_START_ROUTINE Os_HostThreadEntryType;
#endif
