/** @file Controlled GCC x64 adapter; no automotive time or scheduling. */
#include "Det_Host.h"
#include <stdlib.h>

/* Same controlled GCC native TLS mechanism as the delivered OS hook context.
 * Each reporting thread has its own recursion boundary; independent threads
 * may report concurrently. Development reporting never returns to clear it.
 */
static __thread boolean reporting_development;

#define DET_START_SEC_CODE
#include "Det_MemMap.h"
DET_CODE boolean Ecu_DetEnterDevelopmentReport(void) {
    boolean entered = FALSE;
    if (reporting_development == FALSE) {
        reporting_development = TRUE;
        entered = TRUE;
    }
    return entered;
}
DET_CODE void Ecu_DetHalt(void) {
    /* R24-11 requires stopping execution, including an uninitialized report.
     * The controlled host aborts the ECU process instead of hanging its owner.
     */
    abort();
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"
