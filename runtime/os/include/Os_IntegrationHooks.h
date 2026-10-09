/** @file
 * @brief Link-selected consumer observation at the actual OS quiescence boundary.
 */
#ifndef OS_INTEGRATION_HOOKS_H
#define OS_INTEGRATION_HOOKS_H

#include "Os_Types.h"

/** @brief Observe a committed task wait before any completion receipt is published.
 * @param[in] id Task entering the OS waiting/empty completion boundary.
 * @param[in] pending Pending event bits after the task's work is complete.
 * @param[in] predicate Actual event predicate supplied to WaitEvent.
 * @details Called while the existing port critical section is held. Exactly one
 * linked consumer implements this hook. A standalone OS profile with no ECU
 * consumer observes nothing and must not manufacture an ECU completion.
 */
void Os_IntegrationOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate);

/** Authorize the actual bootstrap release of periodic resources after StartupHook.
 * A linked RTE/SchM consumer returns nonzero only after StartTiming. A standalone
 * OS consumer authorizes its native configured timers directly. No clock advances.
 */
int Os_IntegrationTimingAuthorized(void);

#endif
