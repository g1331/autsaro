#ifndef AUTOSAR_EPIC4_OS_STACK_H
#define AUTOSAR_EPIC4_OS_STACK_H
#include "Os_Host.h"
#include <stdint.h>
#include "FreeRTOS.h"
#define OS_NATIVE_STACKS 21u
typedef struct Os_NativeStack {
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
} Os_NativeStack;
extern const Os_NativeStack *const Os_ArtiStacks;
typedef struct {
    volatile Os_Atomic32 published;
    DWORD thread_id;
#ifdef _WIN32
    DWORD exception;
#else
    DWORD signal_number;
#endif
    uintptr_t sp, address;
    char role;
    char origin;
} Os_StackFault;
extern Os_StackFault Os_Fault;
void Os_StackInit(void);
#ifdef __linux__
void Os_StackPrepare(void);
int Os_StackThreadHasFault(DWORD thread_id);
#endif
int Os_StackRegister(char role);
void Os_StackCheck(void);
void Os_StackObserve(HANDLE thread, const CONTEXT *context);
void Os_StackReport(void);
const Os_NativeStack *Os_StackCurrent(void);
const void *Os_StackSavedContext(const Os_NativeStack *stack);
void Os_StackRecordBuffer(const StackType_t *top);
void Os_BackendStackFault(char failed_role);
void Os_StackFatalExit(void);
int Os_StackHasFault(void);
LONG Os_StackFaultCount(void);
#endif
