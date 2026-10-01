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

typedef HANDLE Os_HostHandle;
typedef DWORD Os_HostThreadId;
typedef DWORD Os_HostThreadResult;
typedef LPTHREAD_START_ROUTINE Os_HostThreadEntry;

/** Load one initialized, aligned 32-bit publication cell atomically. */
Os_Atomic32 Os_HostAtomicLoad(volatile Os_Atomic32 *cell);
/** Replace a publication cell and return its prior value. */
Os_Atomic32 Os_HostAtomicExchange(volatile Os_Atomic32 *cell, Os_Atomic32 value);
/** Compare with expected, publish desired on equality, and return prior value. */
Os_Atomic32 Os_HostAtomicCompareExchange(volatile Os_Atomic32 *cell, Os_Atomic32 desired,
                                         Os_Atomic32 expected);
/** Read the calling native actor identity, independent of the automotive clock. */
Os_HostThreadId Os_HostThreadIdentity(void);
/** Monotonic host milliseconds used only for bounded supervision. */
uint64_t Os_HostMonotonicMs(void);
/** Sleep the calling host actor for a finite number of host milliseconds. */
void Os_HostSleepMs(uint32_t milliseconds);
/** Create an initially clear manual-reset event; NULL means allocation failed. */
Os_HostHandle Os_HostOpenManualEvent(void);
/** Create a bounded-stack native actor. Callback storage must outlive the actor. */
Os_HostHandle Os_HostSpawnThread(Os_HostThreadEntry entry, void *argument);
/** Create a joinable supervisory actor, not an automotive/host-input actor. */
Os_HostHandle Os_HostSpawnObserverThread(Os_HostThreadEntry entry, void *argument);
/** Return nonzero only when the event is signaled or the thread has ended. */
int Os_HostWait(Os_HostHandle handle, uint32_t milliseconds);
/** Close a host observer handle, not the live actor it observes. */
int Os_HostClose(Os_HostHandle handle);
/** Select byte-preserving stdin/stdout operation. Failure returns zero. */
int Os_HostSetBinaryStandardStreams(void);
/** Terminate this host process with the supplied status; never returns. */
void Os_HostExit(uint32_t status);
#endif
