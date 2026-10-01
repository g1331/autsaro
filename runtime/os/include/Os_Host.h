#ifndef AUTOSAR_OS_HOST_H
#define AUTOSAR_OS_HOST_H

#include <stdint.h>
/** @file Os_Host.h
 * @brief Native-only synchronization, actor, time and fault primitives.
 *
 * This host boundary does not advance the automotive logical clock. Windows
 * and Linux implementations preserve the same bounded OS service contract;
 * an unsupported host must fail compilation rather than select a fallback.
 */
#if defined(_WIN32)
#include "Os_Windows.h"
#elif defined(__linux__)
#include "Os_HostLinux.h"
#else
#error Unsupported native OS host
#endif

/* Windows Interlocked requires LONG (long on Win64); Linux uses int32_t.
 * Both are exactly four-byte aligned native cells, never Linux long. */
typedef LONG Os_Atomic32;
typedef char Os_Atomic32Width[(sizeof(Os_Atomic32) == sizeof(int32_t)) ? 1 : -1];
typedef char Os_Atomic32Alignment[(__alignof__(Os_Atomic32) >= __alignof__(int32_t)) ? 1 : -1];
typedef uint64_t Os_Atomic64;
typedef char Os_Atomic64Width[(sizeof(Os_Atomic64) == 8u) ? 1 : -1];
typedef char Os_Atomic64Alignment[(__alignof__(Os_Atomic64) >= 8u) ? 1 : -1];
#if defined(__linux__) && (__GCC_ATOMIC_LLONG_LOCK_FREE != 2)
#error Linux host requires always-lock-free 64-bit atomics
#endif
#endif
