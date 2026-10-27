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
#include "Os_HostTypes.h"

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
 * observer handle. Linux host-input stack setup or restoration failure terminates the native
 * process.
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
 * observer handle. On Linux, complete a successful wait before closing this joinable supervisor.
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
 * the handle on success; closing a thread observer does not terminate its actor. A Linux joinable
 * supervisor that has not completed a successful wait is rejected and remains owned by the caller.
 */
OS_MEMMAP int Os_HostClose(Os_HostHandleType handle);

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
