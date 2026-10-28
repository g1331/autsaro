/** @file Native stack and fault layouts shared by OS implementation consumers. */
#ifndef AUTOSAR_OS_STACK_TYPES_H
#define AUTOSAR_OS_STACK_TYPES_H
#include "Os_Version.h"
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
#endif
