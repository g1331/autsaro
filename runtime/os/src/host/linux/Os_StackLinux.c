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

static Os_NativeStack stacks[OS_NATIVE_STACKS];
const Os_NativeStack *const Os_ArtiStacks = stacks;
static CONTEXT saved_contexts[OS_NATIVE_STACKS];
const size_t Os_ArtiNativeContextSize = sizeof(CONTEXT);
static union {
    long double alignment;
    unsigned char bytes[OS_ALTSTACK_BYTES];
} altstacks[OS_NATIVE_STACKS];
static volatile Os_Atomic32 stack_count;
static volatile Os_Atomic32 fault_claimed;
static volatile Os_Atomic32 fault_count;
static volatile Os_Atomic32 fatal_claimed;
static volatile int32_t fault_park;
static Os_StackFault fault_records[OS_NATIVE_STACKS];
static __thread Os_NativeStack *current_stack;
static char invalid_alt_role;
Os_StackFault Os_Fault;

const void *Os_StackSavedContext(const Os_NativeStack *stack) {
    if (stack < stacks || stack >= stacks + OS_NATIVE_STACKS) {
        return NULL;
    }
    return &saved_contexts[stack - stacks];
}

const Os_NativeStack *Os_StackCurrent(void) { return current_stack; }

int Os_StackHasFault(void) { return InterlockedCompareExchange(&fault_count, 0, 0) > 0; }

LONG Os_StackFaultCount(void) { return InterlockedCompareExchange(&fault_count, 0, 0); }
int Os_StackThreadHasFault(DWORD thread_id) {
    size_t index;
    for (index = 0u; index < OS_NATIVE_STACKS; ++index) {
        const Os_StackFault *fault = &fault_records[index];
        if (__atomic_load_n(&fault->published, __ATOMIC_ACQUIRE) != 0 &&
            fault->thread_id == thread_id) {
            return 1;
        }
    }
    return 0;
}

static void fault_park_forever(void) {
    const struct timespec interval = {.tv_sec = 0, .tv_nsec = 10000000L};
    for (;;) {
        (void)syscall(SYS_futex, &fault_park, FUTEX_WAIT_PRIVATE, 0, &interval, NULL, 0);
    }
}

static void publish_fault(Os_NativeStack *record, DWORD signum, uintptr_t sp, uintptr_t address,
                          char origin) {
    Os_StackFault *local;
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

static void guard_signal(int signum, siginfo_t *details, void *native_context) {
    Os_NativeStack *record = current_stack;
    uintptr_t address = (uintptr_t)details->si_addr;
    const ucontext_t *context = native_context;
    if (record == NULL || (signum != SIGSEGV && signum != SIGBUS) ||
        address < record->reserve_low || address >= record->committed_low) {
        _exit(E_OS_STATE);
    }
    publish_fault(record, (DWORD)signum, (uintptr_t)context->uc_mcontext.gregs[REG_RSP], address,
                  'E');
}

void Os_StackPrepare(void) {
    struct sigaction handler = {0};
    const char *invalid = getenv("AUTOSAR_OS_BAD_GUARANTEE");
    if (invalid != NULL && invalid[0] != '\0' && invalid[1] == '\0') {
        invalid_alt_role = invalid[0];
    }
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

void Os_StackInit(void) {
    if (Os_StackRegister('S') == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

int Os_StackRegister(char role) {
    LONG index = InterlockedIncrement(&stack_count) - 1;
    Os_NativeStack *record;
    stack_t alternate;
    uintptr_t low = 0u;
    uintptr_t high = 0u;
    size_t guard = 0u;
    if (index < 0 || index >= (LONG)OS_NATIVE_STACKS || role == invalid_alt_role ||
        Os_HostCurrentStackRange(&low, &high, &guard) == 0 || low >= high || guard >= high - low) {
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
    return 1;
}

void Os_StackCheck(void) {
    uintptr_t sp;
    __asm__ volatile("movq %%rsp, %0" : "=r"(sp));
    if (current_stack == NULL) {
        return;
    }
    if (sp < current_stack->committed_low || sp >= current_stack->high) {
        publish_fault(current_stack, 0u, sp, sp, 'M');
    }
    current_stack->sp = sp;
    ++current_stack->observations;
}

void Os_StackRecordBuffer(const StackType_t *top) {
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

void Os_StackObserve(HANDLE thread, const CONTEXT *context) {
    LONG index;
    const DWORD id = GetThreadId(thread);
    if (context == NULL || context->ContextFlags != CONTEXT_CONTROL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    for (index = 0; index < stack_count && index < (LONG)OS_NATIVE_STACKS; ++index) {
        Os_NativeStack *record = &stacks[index];
        if (record->thread_id == id) {
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

static size_t append_hex(char *output, size_t offset, DWORD value) {
    unsigned index;
    for (index = 0u; index < 8u; ++index) {
        output[offset++] = "0123456789ABCDEF"[(value >> (28u - index * 4u)) & 15u];
    }
    return offset;
}

void Os_StackFatalExit(void) {
    static const char prefix[] =
        "FATAL state=Failed reason=13 controllers_exhausted=1\nFAULT role=";
    static const char middle[] = " signal=";
    static const char suffix[] = " captured=1 record_complete=1\n";
    char output[256];
    size_t used = 0u;
    size_t index;
    const Os_StackFault *fault = &Os_Fault;
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
    for (index = 0u; index < sizeof(prefix) - 1u; ++index) {
        output[used++] = prefix[index];
    }
    output[used++] = fault->role;
    for (index = 0u; index < sizeof(middle) - 1u; ++index) {
        output[used++] = middle[index];
    }
    used = append_hex(output, used, fault->signal_number);
    for (index = 0u; index < sizeof(suffix) - 1u; ++index) {
        output[used++] = suffix[index];
    }
    if (write(STDOUT_FILENO, output, used) != (ssize_t)used) {
        const ssize_t fallback = write(STDERR_FILENO, output, used);
        (void)fallback;
    }
    _exit(E_OS_STACKFAULT);
}

void Os_StackReport(void) {
    LONG index;
    for (index = 0; index < stack_count && index < (LONG)OS_NATIVE_STACKS; ++index) {
        const Os_NativeStack *record = &stacks[index];
        if (record->thread_id == 0u) {
            continue;
        }
        printf("STACK role=%c tid=%u low=%llu high=%llu reserve=%llu commit=%llu "
               "guard=%llu altstack=%llu sp=%llu observations=%u buffer_low=%llu buffer_high=%llu "
               "context=linux-x86_64-ucontext valid=%u\n",
               record->role, record->thread_id, (unsigned long long)record->reserve_low,
               (unsigned long long)record->high, (unsigned long long)record->reserve,
               (unsigned long long)record->committed, (unsigned long long)record->guard,
               (unsigned long long)record->altstack_size, (unsigned long long)record->sp,
               record->observations, (unsigned long long)record->kernel_buffer_low,
               (unsigned long long)record->kernel_buffer_high, record->context_valid);
    }
    for (index = 0; index < (LONG)OS_NATIVE_STACKS; ++index) {
        const Os_StackFault *fault = &fault_records[index];
        if (__atomic_load_n(&fault->published, __ATOMIC_ACQUIRE) != 0) {
            printf("FAULT role=%c tid=%u signal=%08X sp=%llu address=%llu captured=1 origin=%c\n",
                   fault->role, fault->thread_id, fault->signal_number,
                   (unsigned long long)fault->sp, (unsigned long long)fault->address,
                   fault->origin);
        }
    }
}
