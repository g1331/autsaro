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
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern const Os_NativeStackType *const Os_ArtiStacks;
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
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
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP extern Os_StackFaultType Os_Fault;
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackInit(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackPrepare(void);

OS_MEMMAP int Os_StackThreadHasFault(DWORD thread_id);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_StackRegister(char role);

OS_MEMMAP void Os_StackCheck(void);

OS_MEMMAP void Os_StackObserve(HANDLE thread, const CONTEXT *context);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* Reports all available records; returns -1 if any stream write fails. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_StackReport(void);

OS_MEMMAP const Os_NativeStackType *Os_StackCurrent(void);

OS_MEMMAP const void *Os_StackSavedContext(const Os_NativeStackType *stack);

OS_MEMMAP void Os_StackRecordBuffer(const StackType_t *top);

OS_MEMMAP void Os_BackendStackFault(char failed_role);

OS_MEMMAP void Os_StackFatalExit(void);

OS_MEMMAP int Os_StackHasFault(void);

OS_MEMMAP LONG Os_StackFaultCount(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
