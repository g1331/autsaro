/** @file Native-only synchronization, actor, time and fault primitives.
 *
 * This host boundary does not advance the automotive logical clock. Windows
 * and Linux implementations preserve the same bounded OS service contract;
 * an unsupported host must fail compilation rather than select a fallback.
 */
#ifndef AUTOSAR_OS_HOST_H
#define AUTOSAR_OS_HOST_H
#include "Os_Version.h"

#include <stdint.h>
/** SWS_BSW_00121 native GCC TLS mapping; initialized once per host thread. */
#define OS_HOST_THREAD_LOCAL __thread
/** Capture the current x64 stack pointer at the call site.
 * @param[out] sp Writable uintptr_t scalar; evaluated once as an assembly output.
 * @return No value; captures the register into sp.
 * Preconditions: controlled x64 GCC target. Context: current native actor; no blocking.
 * Reentrancy: reentrant; reads only the calling actor register.
 * Ownership: writes the scalar during expansion; no storage is retained.
 */
#define OS_HOST_STACK_POINTER(sp) __asm__ volatile("movq %%rsp, %0" : "=r"(sp))
#if defined(_WIN32)
#include "Os_Windows.h"
#elif defined(__linux__)
#include "Os_HostLinux.h"
#else
#error Unsupported native OS host
#endif

/* Windows Interlocked requires LONG (long on Win64); Linux uses int32_t.
 * Both are exactly four-byte aligned native cells, never Linux long. */
/** Native four-byte aligned Interlocked publication cell; long on Win64 and int32_t on Linux. */
typedef LONG Os_Atomic32Type;
/** Compile-time rejection when a native Interlocked cell has the wrong width. */
typedef char Os_Atomic32WidthType[(sizeof(Os_Atomic32Type) == sizeof(int32_t)) ? 1 : -1];
/** Compile-time rejection when a native Interlocked cell lacks four-byte alignment. */
typedef char
    Os_Atomic32AlignmentType[(__alignof__(Os_Atomic32Type) >= __alignof__(int32_t)) ? 1 : -1];
/** Native eight-byte atomic-storage representation; Linux requires always-lock-free operations. */
typedef uint64_t Os_Atomic64Type;
/** Compile-time rejection when a native 64-bit publication cell is not eight bytes. */
typedef char Os_Atomic64WidthType[(sizeof(Os_Atomic64Type) == 8u) ? 1 : -1];
/** Compile-time rejection when native 64-bit publication storage lacks eight-byte alignment. */
typedef char Os_Atomic64AlignmentType[(__alignof__(Os_Atomic64Type) >= 8u) ? 1 : -1];
#if defined(__linux__) && (__GCC_ATOMIC_LLONG_LOCK_FREE != 2)
#error Linux host requires always-lock-free 64-bit atomics
#endif

/** Owned native event or thread-observation handle; NULL denotes allocation/creation failure. */
typedef HANDLE Os_HostHandleType;
/** Native thread identity, distinct from an automotive Task or ISR identifier. */
typedef DWORD Os_HostThreadIdType;
/** Native thread exit code compatible with the controlled platform thread ABI. */
typedef DWORD Os_HostThreadResultType;
/** Native callback taking a retained context pointer and returning the native thread exit code. */
typedef LPTHREAD_START_ROUTINE Os_HostThreadEntryType;

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Load one initialized, aligned 32-bit publication cell atomically.
 * @param[in,out] cell Initialized writable aligned cell; atomic read-modify-write preserves its
 * value.
 * @return Atomically observed value.
 * Preconditions: cell is non-NULL, writable and naturally aligned; invalid storage is outside the
 * contract. Context: any native actor, including an existing OS critical section; no blocking.
 * Reentrancy: reentrant for the same or different cells; Interlocked provides atomic ordering.
 * Ownership: borrows cell for the call only; no address is retained.
 */
OS_MEMMAP Os_Atomic32Type Os_HostAtomicLoad(volatile Os_Atomic32Type *cell);

/** Replace a publication cell and return its prior value.
 * @param[in,out] cell Initialized writable aligned publication cell.
 * @param[in] value Replacement cell value.
 * @return Value preceding the exchange.
 * Preconditions: cell is non-NULL, writable and naturally aligned; invalid storage is outside the
 * contract. Context: any native actor, including an existing OS critical section; no blocking.
 * Reentrancy: reentrant for the same or different cells; Interlocked provides atomic ordering.
 * Ownership: borrows cell for the call only; no address is retained.
 */
OS_MEMMAP Os_Atomic32Type Os_HostAtomicExchange(volatile Os_Atomic32Type *cell,
                                                Os_Atomic32Type value);

/** Compare with expected, publish desired on equality, and return prior value.
 * @param[in,out] cell Initialized writable aligned publication cell.
 * @param[in] desired Value written when the comparison matches.
 * @param[in] expected Value required for replacement.
 * @return Prior value, whether or not it matched expected.
 * Preconditions: cell is non-NULL, writable and naturally aligned; invalid storage is outside the
 * contract. Context: any native actor, including an existing OS critical section; no blocking.
 * Reentrancy: reentrant for the same or different cells; Interlocked provides atomic ordering.
 * Ownership: borrows cell for the call only; no address is retained.
 */
OS_MEMMAP Os_Atomic32Type Os_HostAtomicCompareExchange(volatile Os_Atomic32Type *cell,
                                                       Os_Atomic32Type desired,
                                                       Os_Atomic32Type expected);

/** Read the calling native actor identity, independent of the automotive clock.
 * @return Calling native thread identifier.
 * Preconditions: initialized host C environment.
 * Context: any native actor; no blocking or automotive time effect.
 * Reentrancy: reentrant; identity is local to the calling native thread.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP Os_HostThreadIdType Os_HostThreadIdentity(void);

/** Monotonic host milliseconds used only for bounded supervision.
 * @return Monotonic host milliseconds since the platform clock origin.
 * Preconditions: initialized host clock.
 * Context: native supervision, outside BSW exclusive areas; not an automotive tick source.
 * Reentrancy: reentrant; observes the host monotonic clock.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP uint64_t Os_HostMonotonicMs(void);

/** Sleep the calling host actor for a finite number of host milliseconds.
 * @param[in] milliseconds Host delay in milliseconds; zero yields execution.
 * @par Return
 * None.
 * Preconditions: finite host delay; automotive scheduling must not rely on this host timer.
 * Context: native actor outside OS critical and BSW exclusive areas; blocks the caller.
 * Reentrancy: independent native actors may sleep concurrently.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP void Os_HostSleepMs(uint32_t milliseconds);

/** Create an initially clear manual-reset event; NULL means allocation failed.
 * @return Initially clear manual-reset event handle, or NULL on failure.
 * Preconditions: host event allocation available; failure returns NULL.
 * Context: native setup/supervision outside OS critical and BSW exclusive areas.
 * Reentrancy: reentrant; each successful call creates an independent event.
 * Ownership: caller owns the returned handle and closes it with Os_HostClose.
 */
OS_MEMMAP Os_HostHandleType Os_HostOpenManualEvent(void);

/** Create a bounded-stack native actor. Callback storage must outlive the actor.
 * @param[in] entry Thread body, valid until the actor ends.
 * @param[in,out] argument Callback context, valid for the whole actor lifetime; may be NULL if
 * entry permits.
 * @return Native thread observer handle, or NULL on creation failure.
 * Preconditions: non-NULL entry and live callback context; host creation failure returns NULL.
 * Context: native setup/supervision outside OS critical and BSW exclusive areas.
 * Reentrancy: reentrant; each successful call creates a separate actor.
 * Ownership: entry and argument are retained until the actor ends; caller owns the returned
 * observer handle.
 */
OS_MEMMAP Os_HostHandleType Os_HostSpawnThread(Os_HostThreadEntryType entry, void *argument);

/** Create a joinable supervisory actor, not an automotive/host-input actor.
 * @param[in] entry Thread body, valid until the actor ends.
 * @param[in,out] argument Callback context, valid for the whole actor lifetime; may be NULL if
 * entry permits.
 * @return Native thread observer handle, or NULL on creation failure.
 * Preconditions: non-NULL entry and live callback context; host creation failure returns NULL.
 * Context: native setup/supervision outside OS critical and BSW exclusive areas.
 * Reentrancy: reentrant; each successful call creates a separate actor.
 * Ownership: entry and argument are retained until the actor ends; caller owns the returned
 * observer handle.
 */
OS_MEMMAP Os_HostHandleType Os_HostSpawnObserverThread(Os_HostThreadEntryType entry,
                                                       void *argument);

/** Return nonzero only when the event is signaled or the thread has ended.
 * @param[in] handle Open event or thread observation handle.
 * @param[in] milliseconds Host wait duration; platform infinite sentinel is allowed.
 * @return Nonzero only for signaled/completed state; zero for timeout or native failure.
 * Preconditions: valid open event/thread handle; do not close it while another actor waits.
 * Context: native supervisor outside OS critical and BSW exclusive areas; may block.
 * Reentrancy: concurrent waits require the host object to remain alive.
 * Ownership: borrows the handle without closing or transferring it.
 */
OS_MEMMAP int Os_HostWait(Os_HostHandleType handle, uint32_t milliseconds);

/** Close a host observer handle, not the live actor it observes.
 * @param[in] handle Open host observer/event handle to close.
 * @return Nonzero on successful close; zero on native failure.
 * Preconditions: valid handle exclusively owned for closing; no concurrent waiter may use a closed
 * handle. Context: native cleanup outside OS critical and BSW exclusive areas. Reentrancy: not
 * reentrant for the same handle; independent handles may close concurrently. Ownership: consumes
 * the handle on success; closing a thread observer does not terminate its actor.
 */
OS_MEMMAP int Os_HostClose(Os_HostHandleType handle);

/** Select byte-preserving stdin/stdout operation. Failure returns zero.
 * @return Nonzero when both streams preserve bytes; zero on a native mode failure.
 * Preconditions: stdin and stdout descriptors are open.
 * Context: native startup before stream consumers exist, outside exclusive areas.
 * Reentrancy: non-reentrant with concurrent standard-stream mode changes.
 * Ownership: changes process stream modes; does not close or transfer either stream.
 */
OS_MEMMAP int Os_HostSetBinaryStandardStreams(void);

/** Terminate this host process with the supplied status; never returns.
 * @param[in] status Native process exit status.
 * @par Return
 * None.
 * Preconditions: initialized host environment; no cleanup is promised after this call.
 * Context: fatal native shutdown, including a damaged-actor recovery path.
 * Reentrancy: the first process termination wins; no call returns.
 * Ownership: scalar arguments are copied; no application storage is retained.
 */
OS_MEMMAP void Os_HostExit(uint32_t status);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Host.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Host.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
