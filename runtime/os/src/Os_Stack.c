/** @file Native stack registration, guard checks and fault capture for the Win64 target. */
#include "Os_Backend.h"
#include "Os_Stack.h"
#include "Os_HostIo.h"

#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_NativeStackType stacks[OS_NATIVE_STACKS]; /**< Actor/stopped-observer records,
                                                   atomic allocation; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Stack.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Stack.c -> Os_MemMap.h: incompatible AUTOSAR release"
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
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP const void *Os_StackSavedContext(const Os_NativeStackType *stack)
{
    /* The caller supplies a record obtained from this module's own array. */
    return &saved_contexts[stack - stacks];
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    stack_count; /**< Atomic native record reservations; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static char invalid_guarantee_role; /**< Startup-only test fault role; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL Os_NativeStackType
    *current_stack; /**< Owning actor TLS stack record; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP Os_StackFaultType
    Os_Fault; /**< First-fault winner snapshot, atomic publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type fault_claimed,
    fault_count; /**< Atomic fault claim/count state; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE
    fault_output; /**< Startup-only stdout handle for fatal writes; no Init reset. */

OS_MEMMAP static HANDLE
    fault_error; /**< Startup-only stderr handle for fatal writes; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
OS_MEMMAP static char fatal_text[256]; /**< Fatal-path winner owns this buffer; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_8
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    fatal_claimed; /**< Atomic terminal-report winner latch; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_StackFaultType fault_records[OS_NATIVE_STACKS]; /**< Per-actor fault slots,
                                                         atomic publication; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_StackHasFault(void)
{
    return (InterlockedCompareExchange(&fault_count, 0, 0) > 0) ? 1 : 0;
}

OS_MEMMAP LONG Os_StackFaultCount(void) { return InterlockedCompareExchange(&fault_count, 0, 0); }

OS_MEMMAP const Os_NativeStackType *Os_StackCurrent(void) { return current_stack; }

OS_MEMMAP void Os_StackRecordBuffer(const StackType_t *top)
{
    current_stack->kernel_buffer_high = (uintptr_t)&top[1];
    current_stack->kernel_buffer_low = current_stack->kernel_buffer_high - (512u * sizeof(*top));
    if ((current_stack->reserve_low < current_stack->kernel_buffer_high) &&
        (current_stack->high > current_stack->kernel_buffer_low)) {
        Os_BackendShutdown(E_OS_STATE);
    }
}

OS_MEMMAP static void publish_fault(
    Os_NativeStackType *record, DWORD code, uintptr_t sp, uintptr_t address, char origin)
{
    Os_StackFaultType *local = &fault_records[record - stacks];
    local->thread_id = record->thread_id;
    local->role = record->role;
    local->exception = code;
    local->origin = origin;
    local->sp = sp;
    local->address = address;
    (void)InterlockedExchange(&local->published, 1);
    if (InterlockedCompareExchange(&fault_claimed, 1, 0) == 0) {
        Os_Fault.thread_id = record->thread_id;
        Os_Fault.role = record->role;
        Os_Fault.exception = code;
        Os_Fault.origin = origin;
        Os_Fault.sp = sp;
        Os_Fault.address = address;
        (void)InterlockedExchange(&Os_Fault.published, 1);
    }
    (void)InterlockedIncrement(&fault_count);
    Os_BackendStackFault((current_stack != NULL) ? current_stack->role : record->role);
}

OS_MEMMAP static LONG CALLBACK stack_exception(EXCEPTION_POINTERS *exception)
{
    DWORD code = exception->ExceptionRecord->ExceptionCode;
    uintptr_t address = 0u;
    if (current_stack == NULL) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    if ((code == EXCEPTION_ACCESS_VIOLATION) &&
        (exception->ExceptionRecord->NumberParameters >= 2u)) {
        address = exception->ExceptionRecord->ExceptionInformation[1];
    }
    if ((code != EXCEPTION_STACK_OVERFLOW) &&
        !((code == EXCEPTION_ACCESS_VIOLATION) && (address >= current_stack->reserve_low) &&
          (address < current_stack->committed_low))) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    /* Only preallocated stores and kernel events; never acquire the port mutex. */
    publish_fault(current_stack, code, (uintptr_t)exception->ContextRecord->Rsp, address, 'E');
    return EXCEPTION_CONTINUE_SEARCH;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_invalid_guarantee(void)
{
    char injected[2] = {0};
    if (GetEnvironmentVariableA("AUTOSAR_OS_BAD_GUARANTEE", injected, sizeof(injected)) == 1u) {
        invalid_guarantee_role = injected[0];
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void read_invalid_guarantee(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_StackInit(void)
{

    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    fault_output = GetStdHandle(STD_OUTPUT_HANDLE);
    fault_error = GetStdHandle(STD_ERROR_HANDLE);
    read_invalid_guarantee();
    if (AddVectoredExceptionHandler(1u, &stack_exception) == NULL) {
        Os_BackendShutdown(E_OS_STATE);
    }
    if (Os_StackRegister('S') == 0) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#if defined(OS_HOST_FAILURE_TESTS) && (OS_HOST_FAILURE_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static ULONG native_stack_guarantee(char role)
{
    /* The injected value exceeds the actual reserve and must fail the native API. */
    return (invalid_guarantee_role == role) ? 0xffffffffu : 16384u;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static ULONG native_stack_guarantee(char role)
{
    (void)role;
    return 16384u;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_StackRegister(char role)
{
    MEMORY_BASIC_INFORMATION region;
    uintptr_t cursor;
    ULONG guarantee = native_stack_guarantee(role);
    LONG index = InterlockedIncrement(&stack_count) - 1;
    Os_NativeStackType *record;
    if (index >= (LONG)OS_NATIVE_STACKS) {
        return 0;
    }
    if (SetThreadStackGuarantee(&guarantee) == 0) {
        return 0;
    }
    record = &stacks[index];
    if (VirtualQuery((const void *)&cursor, &region, sizeof(region)) != sizeof(region)) {
        return 0;
    }
    record->thread_id = GetCurrentThreadId();
    record->role = role;
    record->reserve_low = (uintptr_t)region.AllocationBase;
    cursor = record->reserve_low;
    record->committed_low = UINTPTR_MAX;
    for (;;) {
        if ((VirtualQuery((const void *)cursor, &region, sizeof(region)) != sizeof(region)) ||
            (region.RegionSize == 0u) || (region.RegionSize > (UINTPTR_MAX - cursor))) {
            return 0;
        }
        if ((uintptr_t)region.AllocationBase != record->reserve_low) {
            break;
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
    record->high = cursor;
    uintptr_t sp;
    OS_HOST_STACK_POINTER(sp);
    if ((record->reserve_low >= record->high) || (record->committed_low >= record->high) ||
        (sp < record->committed_low) || (sp >= record->high)) {
        return 0;
    }
    record->reserve = record->high - record->reserve_low;
    guarantee = 0u;
    if (SetThreadStackGuarantee(&guarantee) == 0) {
        return 0;
    }
    record->guarantee = guarantee;
    current_stack = record;
    Os_StackCheck();
    return 1;
}

OS_MEMMAP void Os_StackCheck(void)
{
    uintptr_t sp;
    MEMORY_BASIC_INFORMATION region;
    OS_HOST_STACK_POINTER(sp);
    if (current_stack == NULL) {
        return; /* Public preparation before StartOS does not yet execute an OS task. */
    }
    boolean invalid = (sp < current_stack->reserve_low) || (sp >= current_stack->high);
    if (!invalid) {
        invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);
    }
    if (!invalid) {
        invalid =
            (region.State != MEM_COMMIT) || ((region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0u);
    }
    if (invalid) {
        publish_fault(current_stack, 0u, sp, sp, 'M');
    }
    current_stack->sp = sp;
    ++current_stack->observations;
}

OS_MEMMAP void Os_StackObserve(HANDLE thread, const CONTEXT *context)
{
    DWORD id = GetThreadId(thread);
    LONG i;
    i = 0;
    while ((i < stack_count) && (i < (LONG)OS_NATIVE_STACKS)) {
        Os_NativeStackType *record = &stacks[i];
        if (record->thread_id == id) {
            uintptr_t sp = (uintptr_t)context->Rsp;
            MEMORY_BASIC_INFORMATION region;
            boolean invalid = (sp < record->reserve_low) || (sp >= record->high);
            if (!invalid) {
                invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);
            }
            if (!invalid) {
                invalid = (region.State != MEM_COMMIT) ||
                          ((region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) != 0u);
            }
            if (invalid) {
                publish_fault(record, 0u, sp, sp, 'M');
            }
            record->sp = sp;
            saved_contexts[i] = *context;
            record->context_valid = 1u;
            ++record->observations;
            return;
        }
        ++i;
    }
    Os_BackendShutdown(E_OS_STATE);
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
    " exception="; /**< Immutable native fatal-report bytes; process lifetime. */

OS_MEMMAP static const char fatal_suffix[] =
    " captured=1 record_complete=1\n"; /**< Immutable native fatal-report bytes; process lifetime.
                                        */
#define OS_STOP_SEC_CONST_8
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static int fatal_write(HANDLE handle, const char *data, size_t length)
{
    size_t offset = 0u;
    int result = 1;
    while ((offset < length) && (result != 0)) {
        DWORD written = 0u;
        if ((WriteFile(handle, &data[offset], (DWORD)(length - offset), &written, NULL) == 0) ||
            (written == 0u)) {
            result = 0;
        } else {
            offset += written;
        }
    }
    return result;
}

OS_MEMMAP void Os_StackFatalExit(void)
{
    size_t used = 0u, i;
    const Os_StackFaultType *fault = &Os_Fault;
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
    for (i = 0u; i < sizeof(fatal_prefix) - 1u; ++i) {
        fatal_text[used] = fatal_prefix[i];
        ++used;
    }
    fatal_text[used] = fault->role;
    ++used;
    for (i = 0u; i < sizeof(fatal_middle) - 1u; ++i) {
        fatal_text[used] = fatal_middle[i];
        ++used;
    }
    for (i = 0u; i < 8u; ++i) {
        fatal_text[used] = "0123456789ABCDEF"[(fault->exception >> (28u - (4u * i))) & 15u];
        ++used;
    }
    for (i = 0u; i < sizeof(fatal_suffix) - 1u; ++i) {
        fatal_text[used] = fatal_suffix[i];
        ++used;
    }
    /* No CRT/ordinary mutex/allocation or another Hook on exhausted control stacks. */
    int written = 0;
    if ((fault_output != NULL) && (fault_output != INVALID_HANDLE_VALUE)) {
        written = fatal_write(fault_output, fatal_text, used);
    }
    if (written == 0) {
        (void)fatal_write(fault_error, fatal_text, used);
    }
    ExitProcess(E_OS_STACKFAULT);
}

OS_MEMMAP static int report_stack(const Os_NativeStackType *record)
{
    char storage[512];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextAppend(&text, "STACK role=");
    Os_HostTextByte(&text, record->role);
    Os_HostTextField(&text, " tid=", record->thread_id);
    Os_HostTextField(&text, " low=", record->reserve_low);
    Os_HostTextField(&text, " high=", record->high);
    Os_HostTextField(&text, " reserve=", record->reserve);
    Os_HostTextField(&text, " commit=", record->committed);
    Os_HostTextField(&text, " guard=", record->guard);
    Os_HostTextField(&text, " guarantee=", record->guarantee);
    Os_HostTextField(&text, " sp=", record->sp);
    Os_HostTextField(&text, " observations=", record->observations);
    Os_HostTextField(&text, " buffer_low=", record->kernel_buffer_low);
    Os_HostTextField(&text, " buffer_high=", record->kernel_buffer_high);
    Os_HostTextByte(&text, '\n');
    return Os_HostTextWrite(&text);
}

OS_MEMMAP static int report_fault(const Os_StackFaultType *fault)
{
    char storage[256];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextAppend(&text, "FAULT role=");
    Os_HostTextByte(&text, fault->role);
    Os_HostTextField(&text, " tid=", fault->thread_id);
    Os_HostTextAppend(&text, " exception=");
    Os_HostTextHex(&text, fault->exception, 8u, 1);
    Os_HostTextField(&text, " sp=", fault->sp);
    Os_HostTextField(&text, " address=", fault->address);
    Os_HostTextAppend(&text, " captured=1 origin=");
    Os_HostTextByte(&text, fault->origin);
    Os_HostTextByte(&text, '\n');
    return Os_HostTextWrite(&text);
}

OS_MEMMAP int Os_StackReport(void)
{
    int result = 0;
    LONG index;
    index = 0;
    while ((index < stack_count) && (index < (LONG)OS_NATIVE_STACKS)) {
        if (stacks[index].thread_id != 0u) {
            if (report_stack(&stacks[index]) == 0) {
                result = -1;
            }
        }
        ++index;
    }
    for (index = 0; index < (LONG)OS_NATIVE_STACKS; ++index) {
        if (fault_records[index].published != 0) {
            if (report_fault(&fault_records[index]) == 0) {
                result = -1;
            }
        }
    }
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
