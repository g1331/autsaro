/** @file Controlled GCC x64 adapter; no automotive time or scheduling. */
#include "Det_Host.h"
#if defined(_WIN32)
/* Import process primitives without unrelated RPC boolean declarations. */
#include <processthreadsapi.h>
#else
#include <unistd.h>
#endif

#define DET_HOST_FATAL_STATUS 134

/* Same controlled GCC native TLS mechanism as the delivered OS hook context.
 * Each reporting thread has its own recursion boundary; independent threads
 * may report concurrently. Development reporting never returns to clear it.
 */

#define DET_START_SEC_CODE
#include "Det_MemMap.h"
DET_CODE boolean Det_HostEnterDevelopmentReport(void)
{
    static __thread boolean
        reporting_development; /**< Per-thread recursion boundary; never reset. */
    boolean entered = FALSE;
    if (reporting_development == FALSE) {
        reporting_development = TRUE;
        entered = TRUE;
    }
    return entered;
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"

#if defined(_WIN32)
#define DET_START_SEC_CODE
#include "Det_MemMap.h"
DET_CODE void Det_HostHalt(void)
{
    /* Stop the entire ECU without CRT cleanup, locks or buffered output. */
    (void)TerminateProcess(GetCurrentProcess(), DET_HOST_FATAL_STATUS);
    /* Self-termination never returns on success; trap if the platform refuses it. */
    __builtin_trap();
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"
#else
#define DET_START_SEC_CODE
#include "Det_MemMap.h"
DET_CODE void Det_HostHalt(void)
{
    /* Stop the entire ECU without CRT cleanup, locks or buffered output. */
    _exit(DET_HOST_FATAL_STATUS);
}
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"

#endif
