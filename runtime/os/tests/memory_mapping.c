#include "Os.h"
#include <stdio.h>

static unsigned calls;
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
void ErrorHook(StatusType Error) {
    (void)Error;
    ++calls;
}
void PreTaskHook(void) { ++calls; }
void PostTaskHook(void) { ++calls; }
void StartupHook(void) { ++calls; }
void ShutdownHook(StatusType Error) {
    (void)Error;
    ++calls;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_MEMMAP_CODE_ACTIVE) || defined(OS_START_SEC_CODE) || defined(OS_STOP_SEC_CODE)
#error Memory mapping state leaked from a closed section
#endif

int main(void) {
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
