/** @file Native two-phase Task restart: GCC DWARF on Linux, SEH on Windows. */
#include "Os_PortRestart.h"
#include "Os_Backend.h"
#include <stdint.h>
#ifndef _WIN32
#include <unwind.h>
#endif

/* All traversed C frames use -fexceptions, including sanitizer-generated cleanup
 * landing pads. Native unwinding also restores the hardware shadow stack. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP __attribute__((used)) static EXCEPTION_DISPOSITION Os_PortRestartHandler(
    EXCEPTION_RECORD *record, void *frame, CONTEXT *context, DISPATCHER_CONTEXT *dispatch)
{
    (void)context;
    if ((record->ExceptionCode == 0xE0426001u) &&
        ((record->ExceptionFlags & (DWORD)EXCEPTION_UNWINDING) == 0u)) {
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
