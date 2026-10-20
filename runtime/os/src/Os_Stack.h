/** @file Private native stack and fault records; Linux adds signal-stack fields. */
#ifndef AUTOSAR_OS_STACK_H
#define AUTOSAR_OS_STACK_H
#include "Os_Host.h"
#include <stdint.h>
#include "FreeRTOS.h"
#define OS_NATIVE_STACKS 21u
typedef struct Os_NativeStackType {
    DWORD thread_id;
    char role;
    uintptr_t reserve_low, high, committed_low, sp;
    uintptr_t kernel_buffer_low, kernel_buffer_high;
    SIZE_T reserve, committed, guard;
#ifdef _WIN32
    ULONG guarantee;
#else
    uintptr_t altstack_low;
    SIZE_T altstack_size;
#endif
    unsigned observations;
    uint8_t context_valid;
} Os_NativeStackType;
extern const Os_NativeStackType *const Os_ArtiStacks;
typedef struct {
    volatile Os_Atomic32Type published;
    DWORD thread_id;
#ifdef _WIN32
    DWORD exception;
#else
    DWORD signal_number;
#endif
    uintptr_t sp, address;
    char role;
    char origin;
} Os_StackFaultType;
extern Os_StackFaultType Os_Fault;
void Os_StackInit(void);
#ifdef __linux__
void Os_StackPrepare(void);
int Os_StackThreadHasFault(DWORD thread_id);
#endif
int Os_StackRegister(char role);
void Os_StackCheck(void);
void Os_StackObserve(HANDLE thread, const CONTEXT *context);
/* Reports all available records; returns -1 if any stream write fails. */
int Os_StackReport(void);
const Os_NativeStackType *Os_StackCurrent(void);
const void *Os_StackSavedContext(const Os_NativeStackType *stack);
void Os_StackRecordBuffer(const StackType_t *top);
void Os_BackendStackFault(uint8 failed_role);
void Os_StackFatalExit(void);
int Os_StackHasFault(void);
LONG Os_StackFaultCount(void);
#endif
