#ifndef AUTOSAR_EPIC4_OS_STACK_H
#define AUTOSAR_EPIC4_OS_STACK_H
#include <windows.h>
#include <stdint.h>
#include "FreeRTOS.h"
#define OS_NATIVE_STACKS 21u
typedef struct {
    DWORD thread_id;
    char role;
    uintptr_t reserve_low, high, committed_low, sp;
    uintptr_t kernel_buffer_low, kernel_buffer_high;
    SIZE_T reserve, committed, guard;
    ULONG guarantee;
    unsigned observations;
} Os_NativeStack;
typedef struct {
    volatile LONG published;
    DWORD thread_id, exception;
    uintptr_t sp, address;
    char role;
    char origin;
} Os_StackFault;
extern Os_StackFault Os_Fault;
void Os_StackInit(void);
int Os_StackRegister(char role);
void Os_StackCheck(void);
void Os_StackObserve(HANDLE thread, const CONTEXT *context);
void Os_StackReport(void);
const Os_NativeStack *Os_StackCurrent(void);
void Os_StackRecordBuffer(const StackType_t *top);
void Os_BackendStackFault(char failed_role);
void Os_StackFatalExit(void);
int Os_StackHasFault(void);
LONG Os_StackFaultCount(void);
#endif
