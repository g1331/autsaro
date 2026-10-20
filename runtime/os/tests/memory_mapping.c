/** @file Os_MemMap.h sections bracket Task, ISR, callback and hook code without leaking. */
#include "Os.h"
#include <stdio.h>

/* Verifies[Os]: SWS_BSW_00006.
 * Verifies: SWS_MemMap_00003.
 * Independent native storage roles retain C startup, const and actor TLS semantics. */
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP uint32 mapped_cleared; /**< Fixture-owned zero startup state; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_INIT_32
#include "Os_MemMap.h"
OS_MEMMAP uint32 mapped_initialized =
    0x203u; /**< Fixture-owned initialized value; no Init reset. */
#define OS_STOP_SEC_VAR_INIT_32
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const uint32 *const mapped_reference =
    &mapped_initialized; /**< Immutable fixture view. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"
OS_MEMMAP OS_THREAD_LOCAL uint32 mapped_actor; /**< Calling actor TLS fixture; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_32
#include "Os_MemMap.h"

static unsigned calls; /**< Mapped function call count; fixture-owned, no Init reset. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
TASK(FirstTask) { ++calls; }
TASK(SecondTask) { ++calls; }
ISR(FirstISR) { ++calls; }
ISR(SecondISR) { ++calls; }
ALARMCALLBACK(AlarmBody) { ++calls; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
void ErrorHook(StatusType Error)
{
    (void)Error;
    ++calls;
}
void PreTaskHook(void) { ++calls; }
void PostTaskHook(void) { ++calls; }
void StartupHook(void) { ++calls; }
void ShutdownHook(StatusType Error)
{
    (void)Error;
    ++calls;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_MEMMAP_CODE_ACTIVE) || defined(OS_MEMMAP_ACTIVE) || defined(OS_MEMMAP) ||           \
    defined(OS_START_SEC_CODE) || defined(OS_STOP_SEC_CODE)
#error Memory mapping state leaked from a closed section
#endif

int main(void)
{
    if (mapped_cleared != 0u || mapped_initialized != 0x203u ||
        mapped_reference != &mapped_initialized || *mapped_reference != 0x203u ||
        mapped_actor != 0u) {
        return 2;
    }
    mapped_cleared = 7u;
    mapped_actor = 9u;
    if (mapped_cleared != 7u || mapped_actor != 9u) {
        return 3;
    }
    OS_TASK_ENTRY(FirstTask)();
    OS_TASK_ENTRY(SecondTask)();
    OS_ISR_ENTRY(FirstISR)();
    OS_ISR_ENTRY(SecondISR)();
    AlarmBody();
    ErrorHook(E_OK);
    PreTaskHook();
    PostTaskHook();
    StartupHook();
    ShutdownHook(E_OK);
    printf("memory_mapping calls=%u\n", calls);
    return calls == 10u ? 0 : 1;
}
