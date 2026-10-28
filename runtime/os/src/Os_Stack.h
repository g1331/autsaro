/** @file Private native stack and fault records; Linux adds signal-stack fields. */
#ifndef AUTOSAR_OS_STACK_H
#define AUTOSAR_OS_STACK_H
#include "Os_StackTypes.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern const Os_NativeStackType *const Os_ArtiStacks;
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Stack.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Stack.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
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
/* Reports all available records; returns -1 if any stream write fails. */
OS_MEMMAP int Os_StackReport(void);

OS_MEMMAP const Os_NativeStackType *Os_StackCurrent(void);

/** Resolve an owned record by identity; OS_NATIVE_STACKS denotes any other address.
 * @param[in] stack Borrowed address; never dereferenced by this lookup.
 * @return Matching static slot, or OS_NATIVE_STACKS for NULL, foreign or one-past addresses.
 * Context: any native actor or signal handler; bounded and allocation free.
 * Ownership: does not retain the address or mutate the static records.
 */
OS_MEMMAP static inline unsigned Os_StackIndex(const Os_NativeStackType *stack)
{
    for (unsigned index = 0u; index < OS_NATIVE_STACKS; ++index) {
        if (stack == &Os_ArtiStacks[index]) {
            return index;
        }
    }
    return OS_NATIVE_STACKS;
}

OS_MEMMAP const void *Os_StackSavedContext(const Os_NativeStackType *stack);

OS_MEMMAP void Os_StackRecordBuffer(const StackType_t *top);

OS_MEMMAP void Os_BackendStackFault(char failed_role);

OS_MEMMAP void Os_StackFatalExit(void);

OS_MEMMAP int Os_StackHasFault(void);

OS_MEMMAP LONG Os_StackFaultCount(void);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
