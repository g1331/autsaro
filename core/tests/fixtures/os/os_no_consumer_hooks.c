#include "Os_IntegrationHooks.h"

void Os_IntegrationOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate) {
    /* These independent OS consumers have no ECU observer or ECU receipt. */
    (void)id;
    (void)pending;
    (void)predicate;
}

int Os_IntegrationTimingAuthorized(void) { return 1; }
