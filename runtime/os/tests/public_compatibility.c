#ifdef OS_PUBLIC_TYPES_HOST_BEFORE
#include "Os_Host.h"
#endif
#ifdef OS_PUBLIC_RTE_BEFORE
#include "Rte_Os_Type.h"
#endif
#include "Os.h"
#ifndef OS_PUBLIC_RTE_BEFORE
#include "Rte_Os_Type.h"
#endif
#ifdef OS_PUBLIC_TYPES_HOST_AFTER
#include "Os_Host.h"
#endif
#include <stdio.h>

#define ConfiguredLegacyId 3u
/* The baseline builds only an object for the symbol comparison. It omits the
 * legacy declarations while retaining the same independent executable code. */
#ifndef OS_PUBLIC_COMPAT_BASELINE
DeclareTask(ConfiguredLegacyId);
DeclareTask(UnconfiguredLegacyId);
DeclareResource(ConfiguredLegacyId);
DeclareResource(UnconfiguredLegacyId);
DeclareEvent(ConfiguredLegacyId);
DeclareEvent(UnconfiguredLegacyId);
DeclareAlarm(ConfiguredLegacyId);
DeclareAlarm(UnconfiguredLegacyId);
DeclareTask(ConfiguredLegacyId);
DeclareResource(ConfiguredLegacyId);
DeclareEvent(ConfiguredLegacyId);
DeclareAlarm(ConfiguredLegacyId);
#endif

/* Independent normative name list, with implementation-assigned values.
 * The existing service ABI is checked separately below. */
static const unsigned errors[] = {E_OK,
                                  E_OS_ACCESS,
                                  E_OS_CALLEVEL,
                                  E_OS_ID,
                                  E_OS_LIMIT,
                                  E_OS_NOFUNC,
                                  E_OS_RESOURCE,
                                  E_OS_STATE,
                                  E_OS_VALUE,
                                  E_OS_DISABLEDINT,
                                  E_OS_ILLEGAL_ADDRESS,
                                  E_OS_MISSINGEND,
                                  E_OS_STACKFAULT,
                                  E_OS_PROTECTION_MEMORY,
                                  E_OS_CORE,
                                  E_OS_NESTING_DEADLOCK,
                                  E_OS_PROTECTION_LOCKED,
                                  E_OS_SPINLOCK,
                                  E_OS_SERVICEID,
                                  E_OS_PROTECTION_EXCEPTION,
                                  E_OS_INTERFERENCE_DEADLOCK,
                                  E_OS_PROTECTION_TIME,
                                  E_OS_PROTECTION_ARRIVAL};

int main(void) {
    unsigned evaluations = 0u;
#ifndef OS_PUBLIC_COMPAT_BASELINE
    DeclareTask(++evaluations);
    DeclareResource(++evaluations);
    DeclareEvent(++evaluations);
    DeclareAlarm(++evaluations);
#endif
    if ((evaluations != 0u) || (sizeof(errors) / sizeof(errors[0]) != 23u) || (E_OK != 0u) ||
        (E_OS_STACKFAULT != 13u)) {
        return 1;
    }
    for (unsigned i = 0u; i < 23u; ++i) {
        if ((errors[i] > UINT8_MAX) || ((i != 0u) && (errors[i] == 0u))) {
            return 2;
        }
        if ((i < 12u) && (errors[i] != i)) {
            return 3;
        }
        for (unsigned j = 0u; j < i; ++j) {
            if (errors[i] == errors[j]) {
                return 4;
            }
        }
        const StatusType stored = (StatusType)errors[i];
        if ((unsigned)stored != errors[i]) {
            return 5;
        }
    }
    if (puts("public_compatibility declarations=16 evaluations=0 error_codes=23 unique=pass") ==
        EOF) {
        return 6;
    }
    return 0;
}
