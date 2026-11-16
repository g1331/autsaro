/** @file Native two-phase Task restart: GCC DWARF on Linux, SEH on Windows. */
#include "Os_PortRestart.h"
#include "Os_Backend.h"
#include <stdint.h>
#ifndef _WIN32
#include <unwind.h>
#endif

/* All traversed C frames use -fexceptions, including sanitizer-generated cleanup
 * landing pads. Native unwinding also restores the hardware shadow stack. */
#define OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
OS_MEMMAP static OS_HOST_THREAD_LOCAL struct _Unwind_Exception
    restart_exception; /**< Per-thread unwind object; no reset. */
#define OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static _Unwind_Reason_Code prepare_trace(struct _Unwind_Context *context, void *argument)
{
    (void)context;
    (void)argument;
    return _URC_NO_REASON;
}

OS_MEMMAP int Os_PortRestartPrepare(void)
{
    /* A parked initializer must never block another Task inside the provider's once lock. */
    const _Unwind_Reason_Code status = _Unwind_Backtrace(&prepare_trace, NULL);
    return (status == _URC_END_OF_STACK) ? 1 : 0;
}

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
        if (((unsigned)actions & (unsigned)_UA_SEARCH_PHASE) != 0u) {
            result = _URC_HANDLER_FOUND;
        } else if (((unsigned)actions & (unsigned)_UA_HANDLER_FRAME) != 0u) {
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
