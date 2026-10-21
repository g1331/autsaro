/** @file
 * @brief Link-selected consumer observation at the actual OS quiescence boundary.
 */
#ifndef OS_INTEGRATION_HOOKS_H
#define OS_INTEGRATION_HOOKS_H
#include "Os_Version.h"

#include "Os_Types.h"

/** @brief Observe a committed task wait before any completion receipt is published.
 * @param[in] id Task entering the OS waiting/empty completion boundary.
 * @param[in] pending Pending event bits after the task's work is complete.
 * @param[in] predicate Actual event predicate supplied to WaitEvent.
 * @details Called while the existing port critical section is held. Exactly one
 * linked consumer implements this hook. A standalone OS profile with no ECU
 * consumer observes nothing and must not manufacture an ECU completion.
 * @par Return
 * None.
 * Preconditions: OS has committed the Task wait; linked observer must preserve the completion
 * protocol. Context: OS Task completion path with the port critical section held; no blocking or
 * cross-module area entry. Reentrancy: non-reentrant at this single-core serialized boundary.
 * Ownership: scalar snapshots only; no pointer transferred or retained.
 */
void Os_IntegrationOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);

/** Authorize the actual bootstrap release of periodic resources after StartupHook.
 * A linked RTE/SchM consumer returns nonzero only after StartTiming. A standalone
 * OS consumer authorizes its native configured timers directly. No clock advances.
 * @return Nonzero when the linked consumer authorizes timing; zero otherwise.
 * Preconditions: linked observer initialized by StartupHook; a missing timing start remains
 * unauthorized. Context: OS bootstrap before releasing periodic resources, outside BSW exclusive
 * areas. Reentrancy: non-reentrant startup query; no automotive clock advancement. Ownership:
 * returns a scalar observation; retains no caller storage.
 */
sint32 Os_IntegrationTimingAuthorized(void);

#endif
