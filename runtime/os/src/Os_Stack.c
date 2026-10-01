#define _WIN32_WINNT 0x0602
#include "Os_Backend.h"
#include "Os_Stack.h"
#include <stdio.h>

static Os_NativeStack stacks[OS_NATIVE_STACKS];
const Os_NativeStack *const Os_ArtiStacks = stacks;
static CONTEXT saved_contexts[OS_NATIVE_STACKS];
const size_t Os_ArtiNativeContextSize = sizeof(CONTEXT);
const void *Os_StackSavedContext(const Os_NativeStack *stack) {
    /* The caller supplies a record obtained from this module's own array. */
    return &saved_contexts[stack - stacks];
}
static volatile Os_Atomic32 stack_count;
static char invalid_guarantee_role;
static __thread Os_NativeStack *current_stack;
Os_StackFault Os_Fault;
static volatile Os_Atomic32 fault_claimed, fault_count;
static HANDLE fault_output;
static HANDLE fault_error;
static char fatal_text[256];
static volatile Os_Atomic32 fatal_claimed;
static Os_StackFault fault_records[OS_NATIVE_STACKS];
int Os_StackHasFault(void) { return InterlockedCompareExchange(&fault_count, 0, 0) > 0; }
LONG Os_StackFaultCount(void) { return InterlockedCompareExchange(&fault_count, 0, 0); }
const Os_NativeStack *Os_StackCurrent(void) { return current_stack; }
void Os_StackRecordBuffer(const StackType_t *top) {
    current_stack->kernel_buffer_high = (uintptr_t)(top + 1);
    current_stack->kernel_buffer_low = current_stack->kernel_buffer_high - 512u * sizeof(*top);
    if (current_stack->reserve_low < current_stack->kernel_buffer_high &&
        current_stack->high > current_stack->kernel_buffer_low) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

static void publish_fault(Os_NativeStack *record, DWORD code, uintptr_t sp, uintptr_t address,
                          char origin) {
    Os_StackFault *local = &fault_records[record - stacks];
    local->thread_id = record->thread_id;
    local->role = record->role;
    local->exception = code;
    local->origin = origin;
    local->sp = sp;
    local->address = address;
    InterlockedExchange(&local->published, 1);
    if (InterlockedCompareExchange(&fault_claimed, 1, 0) == 0) {
        Os_Fault.thread_id = record->thread_id;
        Os_Fault.role = record->role;
        Os_Fault.exception = code;
        Os_Fault.origin = origin;
        Os_Fault.sp = sp;
        Os_Fault.address = address;
        InterlockedExchange(&Os_Fault.published, 1);
    }
    InterlockedIncrement(&fault_count);
    Os_BackendStackFault(current_stack != NULL ? current_stack->role : record->role);
}
static LONG CALLBACK stack_exception(EXCEPTION_POINTERS *exception) {
    DWORD code = exception->ExceptionRecord->ExceptionCode;
    uintptr_t address = 0u;
    if (current_stack == NULL) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    if (code == EXCEPTION_ACCESS_VIOLATION && exception->ExceptionRecord->NumberParameters >= 2u) {
        address = exception->ExceptionRecord->ExceptionInformation[1];
    }
    if (code != EXCEPTION_STACK_OVERFLOW &&
        !(code == EXCEPTION_ACCESS_VIOLATION && address >= current_stack->reserve_low &&
          address < current_stack->committed_low)) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    /* Only preallocated stores and kernel events; never acquire the port mutex. */
    publish_fault(current_stack, code, (uintptr_t)exception->ContextRecord->Rsp, address, 'E');
    return EXCEPTION_CONTINUE_SEARCH;
}

void Os_StackInit(void) {
    char injected[2] = {0};
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    fault_output = GetStdHandle(STD_OUTPUT_HANDLE);
    fault_error = GetStdHandle(STD_ERROR_HANDLE);
    if (GetEnvironmentVariableA("AUTOSAR_OS_BAD_GUARANTEE", injected, sizeof(injected)) == 1u) {
        invalid_guarantee_role = injected[0];
    }
    if (AddVectoredExceptionHandler(1u, stack_exception) == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    if (!Os_StackRegister('S')) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

int Os_StackRegister(char role) {
    ULONG_PTR low, high;
    MEMORY_BASIC_INFORMATION region;
    uintptr_t cursor;
    ULONG guarantee = 16384u;
    LONG index = InterlockedIncrement(&stack_count) - 1;
    Os_NativeStack *record;
    if (invalid_guarantee_role == role) {
        guarantee = 0xffffffffu; /* Actual API rejection: larger than the reserved stack. */
    }
    if (index >= (LONG)OS_NATIVE_STACKS || !SetThreadStackGuarantee(&guarantee)) {
        return 0;
    }
    record = &stacks[index];
    GetCurrentThreadStackLimits(&low, &high);
    if (VirtualQuery((const void *)&cursor, &region, sizeof(region)) != sizeof(region)) {
        return 0;
    }
    record->thread_id = GetCurrentThreadId();
    record->role = role;
    record->reserve_low = (uintptr_t)region.AllocationBase;
    if (record->reserve_low >= high || low < record->reserve_low || low >= high) {
        return 0;
    }
    record->high = high;
    record->reserve = high - record->reserve_low;
    record->committed_low = high;
    cursor = record->reserve_low;
    while (cursor < high) {
        if (VirtualQuery((const void *)cursor, &region, sizeof(region)) != sizeof(region) ||
            region.RegionSize == 0u) {
            return 0;
        }
        if (region.State == MEM_COMMIT) {
            record->committed += region.RegionSize;
            if (cursor < record->committed_low) {
                record->committed_low = cursor;
            }
            if ((region.Protect & PAGE_GUARD) != 0u) {
                record->guard += region.RegionSize;
            }
        }
        cursor += region.RegionSize;
    }
    guarantee = 0u;
    if (!SetThreadStackGuarantee(&guarantee)) {
        return 0;
    }
    record->guarantee = guarantee;
    current_stack = record;
    Os_StackCheck();
    return 1;
}

void Os_StackCheck(void) {
    uintptr_t sp;
    ULONG_PTR low, high;
    MEMORY_BASIC_INFORMATION region;
    __asm__ volatile("movq %%rsp, %0" : "=r"(sp));
    if (current_stack == NULL) {
        return; /* Public preparation before StartOS does not yet execute an OS task. */
    }
    GetCurrentThreadStackLimits(&low, &high);
    if (sp < low || sp >= high || sp < current_stack->reserve_low || sp >= current_stack->high ||
        VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region) ||
        region.State != MEM_COMMIT || (region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0u) {
        publish_fault(current_stack, 0u, sp, sp, 'M');
    }
    current_stack->sp = sp;
    ++current_stack->observations;
}

void Os_StackObserve(HANDLE thread, const CONTEXT *context) {
    DWORD id = GetThreadId(thread);
    LONG i;
    for (i = 0; i < stack_count && i < (LONG)OS_NATIVE_STACKS; ++i) {
        Os_NativeStack *record = &stacks[i];
        if (record->thread_id == id) {
            uintptr_t sp = (uintptr_t)context->Rsp;
            MEMORY_BASIC_INFORMATION region;
            if (sp < record->reserve_low || sp >= record->high ||
                VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region) ||
                region.State != MEM_COMMIT ||
                (region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0u) {
                publish_fault(record, 0u, sp, sp, 'M');
            }
            record->sp = sp;
            saved_contexts[i] = *context;
            record->context_valid = 1u;
            ++record->observations;
            return;
        }
    }
    Os_BackendShutdown(E_OS_STATE);
}
void Os_StackFatalExit(void) {
    static const char prefix[] =
        "FATAL state=Failed reason=13 controllers_exhausted=1\nFAULT role=";
    static const char middle[] = " exception=";
    static const char suffix[] = " captured=1 record_complete=1\n";
    size_t used = 0u, i;
    DWORD written;
    const Os_StackFault *fault = &Os_Fault;
    if (InterlockedCompareExchange(&fatal_claimed, 1, 0) != 0) {
        Sleep(INFINITE);
        ExitProcess(E_OS_STACKFAULT);
    }
    if (fault->published == 0) {
        for (i = 0u; i < OS_NATIVE_STACKS; ++i) {
            if (fault_records[i].published != 0) {
                fault = &fault_records[i];
                break;
            }
        }
    }
    for (i = 0u; i < sizeof(prefix) - 1u; ++i) {
        fatal_text[used++] = prefix[i];
    }
    fatal_text[used++] = fault->role;
    for (i = 0u; i < sizeof(middle) - 1u; ++i) {
        fatal_text[used++] = middle[i];
    }
    for (i = 0u; i < 8u; ++i) {
        fatal_text[used++] = "0123456789ABCDEF"[(fault->exception >> (28u - 4u * i)) & 15u];
    }
    for (i = 0u; i < sizeof(suffix) - 1u; ++i) {
        fatal_text[used++] = suffix[i];
    }
    /* No CRT/ordinary mutex/allocation or another Hook on exhausted control stacks. */
    if (fault_output == NULL || fault_output == INVALID_HANDLE_VALUE ||
        !WriteFile(fault_output, fatal_text, (DWORD)used, &written, NULL) || written != used) {
        (void)WriteFile(fault_error, fatal_text, (DWORD)used, &written, NULL);
    }
    ExitProcess(E_OS_STACKFAULT);
}

void Os_StackReport(void) {
    LONG i;
    for (i = 0; i < stack_count && i < (LONG)OS_NATIVE_STACKS; ++i) {
        Os_NativeStack *record = &stacks[i];
        if (record->thread_id == 0u) {
            continue;
        }
        printf(
            "STACK role=%c tid=%lu low=%llu high=%llu reserve=%llu commit=%llu "
            "guard=%llu guarantee=%lu sp=%llu observations=%u buffer_low=%llu buffer_high=%llu\n",
            record->role, (unsigned long)record->thread_id, (unsigned long long)record->reserve_low,
            (unsigned long long)record->high, (unsigned long long)record->reserve,
            (unsigned long long)record->committed, (unsigned long long)record->guard,
            (unsigned long)record->guarantee, (unsigned long long)record->sp, record->observations,
            (unsigned long long)record->kernel_buffer_low,
            (unsigned long long)record->kernel_buffer_high);
    }
    for (i = 0; i < (LONG)OS_NATIVE_STACKS; ++i) {
        Os_StackFault *fault = &fault_records[i];
        if (fault->published == 0) {
            continue;
        }
        printf("FAULT role=%c tid=%lu exception=%08lX sp=%llu address=%llu captured=1 origin=%c\n",
               fault->role, (unsigned long)fault->thread_id, (unsigned long)fault->exception,
               (unsigned long long)fault->sp, (unsigned long long)fault->address, fault->origin);
    }
}
