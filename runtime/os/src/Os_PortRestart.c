/** @file Native two-phase Task restart: GCC DWARF on Linux, SEH on Windows. */
#include "Os_PortRestart.h"
#include "Os_Backend.h"
#include <stdint.h>
#ifndef _WIN32
#include <unwind.h>
#endif

/* All traversed C frames use -fexceptions, including sanitizer-generated cleanup
 * landing pads. Native unwinding also restores the hardware shadow stack. */
#ifndef _WIN32
#define OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL struct _Unwind_Exception
    restart_exception; /**< Per-thread unwind object; no reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/* _Unwind_Personality_Fn fixes the writable exception-pointer signature. */
OS_MEMMAP __attribute__((used)) static _Unwind_Reason_Code Os_PortRestartPersonality(
    int version,
    _Unwind_Action actions,
    _Unwind_Exception_Class cls,
    struct _Unwind_Exception *exception,
    struct _Unwind_Context *context)
{
    _Unwind_Reason_Code result = _URC_CONTINUE_UNWIND;
    if ((version == 1) && (cls == UINT64_C(0x4155544f53525431)) &&
        (exception == &restart_exception)) {
        if ((actions & _UA_SEARCH_PHASE) != 0) {
            result = _URC_HANDLER_FOUND;
        } else if ((actions & _UA_HANDLER_FRAME) != 0) {
            _Unwind_SetIP(context, Os_PortRestartAddress());
            result = _URC_INSTALL_CONTEXT;
        } else {
            /* The native unwinder continues through non-handler cleanup frames. */
        }
    }
    return result;
}
OS_MEMMAP void Os_PortRestart(void)
{
    restart_exception.exception_class = UINT64_C(0x4155544f53525431);
    (void)_Unwind_RaiseException(&restart_exception);
    Os_BackendShutdown(E_OS_STATE);
}
__asm__(
    ".pushsection .os_code,\"ax\",@progbits\n"
    ".global Os_PortTaskLoop\n.type Os_PortTaskLoop,@function\nOs_PortTaskLoop:\n.cfi_startproc\n"
    ".cfi_personality 0x1b,Os_PortRestartPersonality\n"
    "endbr64\npush %r12\n.cfi_def_cfa_offset 16\n.cfi_offset 12,-16\n"
    "push %r13\n.cfi_def_cfa_offset 24\n.cfi_offset 13,-24\n"
    "sub $8,%rsp\n.cfi_def_cfa_offset 32\nmov %rdi,%r12\nmov %rsi,%r13\n"
    ".local Os_PortRestartLanding\n.type "
    "Os_PortRestartLanding,@function\nOs_PortRestartLanding:\nendbr64\nmov %r13,%rdi\ncall "
    "*%r12\nud2\n.size Os_PortRestartLanding,.-Os_PortRestartLanding\n.cfi_endproc\n.size "
    "Os_PortTaskLoop,.-Os_PortTaskLoop\n"
    ".global Os_PortRestartAddress\n.type Os_PortRestartAddress,@function\n"
    "Os_PortRestartAddress:\n.cfi_startproc\nendbr64\n"
    "leaq Os_PortRestartLanding(%rip),%rax\nret\n.cfi_endproc\n"
    ".size Os_PortRestartAddress,.-Os_PortRestartAddress\n.popsection\n");
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP __attribute__((used)) static EXCEPTION_DISPOSITION Os_PortRestartHandler(
    EXCEPTION_RECORD *record, void *frame, CONTEXT *context, DISPATCHER_CONTEXT *dispatch)
{
    (void)context;
    if ((record->ExceptionCode == 0xE0426001u) &&
        ((record->ExceptionFlags & EXCEPTION_UNWINDING) == 0)) {
        RtlUnwindEx(frame,
                    Os_PortRestartAddress(),
                    record,
                    NULL,
                    dispatch->ContextRecord,
                    dispatch->HistoryTable);
        Os_BackendShutdown(E_OS_STATE);
    }
    return ExceptionContinueSearch;
}
OS_MEMMAP void Os_PortRestart(void)
{
    RaiseException(0xE0426001u, EXCEPTION_NONCONTINUABLE, 0, NULL);
    Os_BackendShutdown(E_OS_STATE);
}
__asm__(".section .os_code,\"xr\"\n.global Os_PortTaskLoop\n.def Os_PortTaskLoop; .scl 2; .type "
        "32; .endef\n.seh_proc Os_PortTaskLoop\nOs_PortTaskLoop:\n"
        "endbr64\nsub $56,%rsp\n.seh_stackalloc 56\nmov %r12,40(%rsp)\n.seh_savereg r12,40\n"
        "mov %r13,48(%rsp)\n.seh_savereg r13,48\n.seh_handler "
        "Os_PortRestartHandler,@except\n.seh_endprologue\n"
        "mov %rcx,%r12\nmov %rdx,%r13\n"
        ".def Os_PortRestartLanding; .scl 3; .type 32; .endef\n"
        "Os_PortRestartLanding:\nendbr64\nmov %r13,%rcx\ncall "
        "*%r12\nud2\n.seh_endproc\n"
        ".global Os_PortRestartAddress\n.def Os_PortRestartAddress; .scl 2; .type 32; .endef\n"
        ".seh_proc Os_PortRestartAddress\nOs_PortRestartAddress:\n.seh_endprologue\nendbr64\n"
        "leaq Os_PortRestartLanding(%rip),%rax\nret\n.seh_endproc\n.text\n");
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
