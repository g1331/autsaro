/** @file Linux native stack registration, alternate signal stacks and fault reporting. */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "Os_Backend.h"
#include "Os_Stack.h"

#include <errno.h>
#include <linux/futex.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/syscall.h>
#include <unistd.h>

#define OS_ALTSTACK_BYTES 65536u

#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_NativeStackType stacks[OS_NATIVE_STACKS]; /**< Actor/stopped-observer records,
                                                   atomic allocation; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_StackLinux.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_StackLinux.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const Os_NativeStackType *const Os_ArtiStacks =
    stacks; /**< Fixed native stacks tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static CONTEXT
    saved_contexts[OS_NATIVE_STACKS]; /**< Observer-owned stopped contexts; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_64
#include "Os_MemMap.h"
OS_MEMMAP const uint64 Os_ArtiNativeContextSize =
    sizeof(CONTEXT); /**< Fixed native context size tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static union {
    long double alignment;
    unsigned char bytes[OS_ALTSTACK_BYTES];
} altstacks[OS_NATIVE_STACKS]; /**< Actor-owned alternate signal stacks; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    stack_published[OS_NATIVE_STACKS]; /**< Completed registration publication; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    stack_count; /**< Atomic native record reservations; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    fault_claimed; /**< Atomic first-fault winner latch; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    fault_count; /**< Atomic fault claim/count state; no Init reset. */

OS_MEMMAP static volatile Os_Atomic32Type
    fatal_claimed; /**< Atomic terminal-report winner latch; no Init reset. */

OS_MEMMAP static volatile int32_t fault_park; /**< Constant-zero futex park cell; no Init writes. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_StackFaultType fault_records[OS_NATIVE_STACKS]; /**< Per-actor fault slots,
                                                         atomic publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL Os_NativeStackType
    *current_stack; /**< Owning actor TLS stack record; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static char invalid_alt_role; /**< Startup-only test fault role; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP Os_StackFaultType
    Os_Fault; /**< First-fault winner snapshot, atomic publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP const void *Os_StackSavedContext(const Os_NativeStackType *stack)
{
    if (stack < stacks || stack >= stacks + OS_NATIVE_STACKS) {
        return NULL;
    }
    return &saved_contexts[stack - stacks];
}

OS_MEMMAP const Os_NativeStackType *Os_StackCurrent(void) { return current_stack; }

OS_MEMMAP int Os_StackHasFault(void) { return InterlockedCompareExchange(&fault_count, 0, 0) > 0; }

OS_MEMMAP LONG Os_StackFaultCount(void) { return InterlockedCompareExchange(&fault_count, 0, 0); }

OS_MEMMAP int Os_StackThreadHasFault(DWORD thread_id)
{
    size_t index;
    for (index = 0u; index < OS_NATIVE_STACKS; ++index) {
        const Os_StackFaultType *fault = &fault_records[index];
        if (__atomic_load_n(&fault->published, __ATOMIC_ACQUIRE) != 0 &&
            fault->thread_id == thread_id) {
            return 1;
        }
    }
    return 0;
}

OS_MEMMAP static void fault_park_forever(void)
{
    const struct timespec interval = {.tv_sec = 0, .tv_nsec = 10000000L};
    for (;;) {
        (void)syscall(SYS_futex, &fault_park, FUTEX_WAIT_PRIVATE, 0, &interval, NULL, 0);
    }
}

OS_MEMMAP static void publish_fault(
    Os_NativeStackType *record, DWORD signum, uintptr_t sp, uintptr_t address, char origin)
{
    Os_StackFaultType *local;
    if (record == NULL || record < stacks || record >= stacks + OS_NATIVE_STACKS) {
        _exit(E_OS_STATE);
    }
    local = &fault_records[record - stacks];
    local->thread_id = record->thread_id;
    local->role = record->role;
    local->signal_number = signum;
    local->origin = origin;
    local->sp = sp;
    local->address = address;
    __atomic_store_n(&local->published, 1, __ATOMIC_RELEASE);
    if (InterlockedCompareExchange(&fault_claimed, 1, 0) == 0) {
        Os_Fault.thread_id = record->thread_id;
        Os_Fault.role = record->role;
        Os_Fault.signal_number = signum;
        Os_Fault.origin = origin;
        Os_Fault.sp = sp;
        Os_Fault.address = address;
        __atomic_store_n(&Os_Fault.published, 1, __ATOMIC_RELEASE);
    }
    (void)InterlockedIncrement(&fault_count);
    Os_BackendSignalStackFault(record->role);
    fault_park_forever();
}

OS_MEMMAP static void guard_signal(int signum, siginfo_t *details, void *native_context)
{
    Os_NativeStackType *record = current_stack;
    uintptr_t address = (uintptr_t)details->si_addr;
    const ucontext_t *context = native_context;
    if (record == NULL || (signum != SIGSEGV && signum != SIGBUS) ||
        address < record->reserve_low || address >= record->committed_low) {
        _exit(E_OS_STATE);
    }
    publish_fault(
        record, (DWORD)signum, (uintptr_t)context->uc_mcontext.gregs[REG_RSP], address, 'E');
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_invalid_altstack(void)
{
    const char *invalid = getenv("AUTOSAR_OS_BAD_GUARANTEE");
    if (invalid != NULL && invalid[0] != '\0' && invalid[1] == '\0') {
        invalid_alt_role = invalid[0];
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_invalid_altstack(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackPrepare(void)
{
    struct sigaction handler = {0};
    read_invalid_altstack();
    if (Os_HostInstallSignals() == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
    handler.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigfillset(&handler.sa_mask);
    handler.sa_sigaction = guard_signal;
    if (sigaction(SIGSEGV, &handler, NULL) != 0 || sigaction(SIGBUS, &handler, NULL) != 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

OS_MEMMAP void Os_StackInit(void)
{
    if (Os_StackRegister('S') == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static int registration_stack_range(char role,
                                              uintptr_t *low,
                                              uintptr_t *high,
                                              size_t *guard)
{
    return role == invalid_alt_role ? 0 : Os_HostCurrentStackRange(low, high, guard);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static int registration_stack_range(char role,
                                              uintptr_t *low,
                                              uintptr_t *high,
                                              size_t *guard)
{
    (void)role;
    return Os_HostCurrentStackRange(low, high, guard);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_StackRegister(char role)
{
    LONG index = InterlockedIncrement(&stack_count) - 1;
    Os_NativeStackType *record;
    stack_t alternate;
    uintptr_t low = 0u;
    uintptr_t high = 0u;
    size_t guard = 0u;
    if (index < 0 || index >= (LONG)OS_NATIVE_STACKS ||
        registration_stack_range(role, &low, &high, &guard) == 0 || low >= high ||
        guard >= high - low) {
        return 0;
    }
    record = &stacks[index];
    alternate.ss_sp = altstacks[index].bytes;
    alternate.ss_size = sizeof(altstacks[index].bytes);
    alternate.ss_flags = 0;
    if (sigaltstack(&alternate, NULL) != 0) {
        return 0;
    }
    record->thread_id = GetCurrentThreadId();
    record->role = role;
    record->reserve_low = low;
    record->high = high;
    record->reserve = high - low;
    record->guard = guard;
    record->committed_low = low + guard;
    record->committed = high - record->committed_low;
    record->altstack_low = (uintptr_t)alternate.ss_sp;
    record->altstack_size = alternate.ss_size;
    current_stack = record;
    Os_StackCheck();
    (void)InterlockedExchange(&stack_published[index], 1);
    return 1;
}

OS_MEMMAP void Os_StackCheck(void)
{
    uintptr_t sp;
    OS_HOST_STACK_POINTER(sp);
    if (current_stack == NULL) {
        return;
    }
    if (sp < current_stack->committed_low || sp >= current_stack->high) {
        publish_fault(current_stack, 0u, sp, sp, 'M');
    }
    current_stack->sp = sp;
    ++current_stack->observations;
}

OS_MEMMAP void Os_StackRecordBuffer(const StackType_t *top)
{
    if (current_stack == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    current_stack->kernel_buffer_high = (uintptr_t)(top + 1);
    current_stack->kernel_buffer_low = current_stack->kernel_buffer_high - 512u * sizeof(*top);
    if (current_stack->reserve_low < current_stack->kernel_buffer_high &&
        current_stack->high > current_stack->kernel_buffer_low) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

OS_MEMMAP void Os_StackObserve(HANDLE thread, const CONTEXT *context)
{
    LONG index;
    const DWORD id = GetThreadId(thread);
    if (context == NULL || context->ContextFlags != CONTEXT_CONTROL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    for (index = 0; (index < InterlockedCompareExchange(&stack_count, 0, 0)) &&
                    (index < (LONG)OS_NATIVE_STACKS);
         ++index) {
        Os_NativeStackType *record = &stacks[index];
        /* A reservation can belong to another actor still registering. */
        if ((InterlockedCompareExchange(&stack_published[index], 0, 0) != 0) &&
            (record->thread_id == id)) {
            uintptr_t sp = context->Rsp;
            if (sp < record->committed_low || sp >= record->high) {
                publish_fault(record, 0u, sp, sp, 'M');
            }
            record->sp = sp;
            saved_contexts[index] = *context;
            saved_contexts[index].native.uc_mcontext.fpregs =
                &saved_contexts[index].native.__fpregs_mem;
            record->context_valid = 1u;
            ++record->observations;
            return;
        }
    }
    Os_BackendShutdown(E_OS_STATE);
}

OS_MEMMAP static size_t append_hex(char *output, size_t offset, DWORD value)
{
    unsigned index;
    for (index = 0u; index < 8u; ++index) {
        output[offset++] = "0123456789ABCDEF"[(value >> (28u - index * 4u)) & 15u];
    }
    return offset;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#define OS_START_SEC_CONST_8
#include "Os_MemMap.h"
OS_MEMMAP static const char fatal_prefix[] =
    "FATAL state=Failed reason=13 controllers_exhausted=1\nFAULT role="; /**< Immutable native
                                                                            fatal-report bytes;
                                                                            process lifetime. */

OS_MEMMAP static const char fatal_middle[] =
    " signal="; /**< Immutable native fatal-report bytes; process lifetime. */

OS_MEMMAP static const char fatal_suffix[] =
    " captured=1 record_complete=1\n"; /**< Immutable native fatal-report bytes; process lifetime.
                                        */
#define OS_STOP_SEC_CONST_8
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackFatalExit(void)
{
    char output[256];
    size_t used = 0u;
    size_t index;
    const Os_StackFaultType *fault = &Os_Fault;
    if (InterlockedCompareExchange(&fatal_claimed, 1, 0) != 0) {
        _exit(E_OS_STACKFAULT);
    }
    if (__atomic_load_n(&fault->published, __ATOMIC_ACQUIRE) == 0) {
        for (index = 0u; index < OS_NATIVE_STACKS; ++index) {
            if (__atomic_load_n(&fault_records[index].published, __ATOMIC_ACQUIRE) != 0) {
                fault = &fault_records[index];
                break;
            }
        }
    }
    for (index = 0u; index < sizeof(fatal_prefix) - 1u; ++index) {
        output[used++] = fatal_prefix[index];
    }
    output[used++] = fault->role;
    for (index = 0u; index < sizeof(fatal_middle) - 1u; ++index) {
        output[used++] = fatal_middle[index];
    }
    used = append_hex(output, used, fault->signal_number);
    for (index = 0u; index < sizeof(fatal_suffix) - 1u; ++index) {
        output[used++] = fatal_suffix[index];
    }
    if (write(STDOUT_FILENO, output, used) != (ssize_t)used) {
        const ssize_t fallback = write(STDERR_FILENO, output, used);
        (void)fallback;
    }
    _exit(E_OS_STACKFAULT);
}

OS_MEMMAP int Os_StackReport(void)
{
    int result = 0;
    LONG index;
    for (index = 0; (index < InterlockedCompareExchange(&stack_count, 0, 0)) &&
                    (index < (LONG)OS_NATIVE_STACKS);
         ++index) {
        const Os_NativeStackType *record = &stacks[index];
        if (InterlockedCompareExchange(&stack_published[index], 0, 0) == 0) {
            continue;
        }
        if (printf(
                "STACK role=%c tid=%u low=%llu high=%llu reserve=%llu commit=%llu "
                "guard=%llu altstack=%llu sp=%llu observations=%u buffer_low=%llu buffer_high=%llu "
                "context=linux-x86_64-ucontext valid=%u\n",
                record->role,
                record->thread_id,
                (unsigned long long)record->reserve_low,
                (unsigned long long)record->high,
                (unsigned long long)record->reserve,
                (unsigned long long)record->committed,
                (unsigned long long)record->guard,
                (unsigned long long)record->altstack_size,
                (unsigned long long)record->sp,
                record->observations,
                (unsigned long long)record->kernel_buffer_low,
                (unsigned long long)record->kernel_buffer_high,
                record->context_valid) < 0) {
            result = -1;
        }
    }
    for (index = 0; index < (LONG)OS_NATIVE_STACKS; ++index) {
        const Os_StackFaultType *fault = &fault_records[index];
        if (__atomic_load_n(&fault->published, __ATOMIC_ACQUIRE) != 0) {
            if (printf(
                    "FAULT role=%c tid=%u signal=%08X sp=%llu address=%llu captured=1 origin=%c\n",
                    fault->role,
                    fault->thread_id,
                    fault->signal_number,
                    (unsigned long long)fault->sp,
                    (unsigned long long)fault->address,
                    fault->origin) < 0) {
                result = -1;
            }
        }
    }
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
