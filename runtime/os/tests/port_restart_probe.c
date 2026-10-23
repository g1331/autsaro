/** @file Temporary native unwind and actual CET execution experiment. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#ifndef _WIN32
#include <unwind.h>
#else
#include <windows.h>
#endif
extern void run(void (*entry)(void *), void *argument);
extern void landing(void);
#ifndef _WIN32
static __thread struct _Unwind_Exception
    restart_exception; /**< Per-thread unwind object; no reset. */
static void cleanup(_Unwind_Reason_Code code, struct _Unwind_Exception *exception)
{
    (void)code;
    (void)exception;
}
_Unwind_Reason_Code personality(int version,
                                _Unwind_Action actions,
                                uint64_t cls,
                                struct _Unwind_Exception *exception,
                                struct _Unwind_Context *context)
{
    (void)exception;
    if (version != 1 || cls != UINT64_C(0x4155544f53525431)) {
        return _URC_CONTINUE_UNWIND;
    }
    if (actions & _UA_SEARCH_PHASE) {
        return _URC_HANDLER_FOUND;
    }
    if (actions & _UA_HANDLER_FRAME) {
        _Unwind_SetIP(context, (uintptr_t)landing);
        return _URC_INSTALL_CONTEXT;
    }
    return _URC_CONTINUE_UNWIND;
}
__attribute__((noinline)) void restart(void)
{
    restart_exception.exception_class = UINT64_C(0x4155544f53525431);
    restart_exception.exception_cleanup = cleanup;
    _Unwind_RaiseException(&restart_exception);
    abort();
}
__asm__(".pushsection .text\n"
        ".global run\n.type run,@function\nrun:\n.cfi_startproc\n"
        ".cfi_personality 0x9b,DW.ref.personality\n"
        "endbr64\npush %r12\n.cfi_def_cfa_offset 16\n.cfi_offset 12,-16\n"
        "push %r13\n.cfi_def_cfa_offset 24\n.cfi_offset 13,-24\n"
        "sub $8,%rsp\n.cfi_def_cfa_offset 32\nmov %rdi,%r12\nmov %rsi,%r13\n"
        ".global landing\nlanding:\nendbr64\nmov %r13,%rdi\ncall *%r12\nud2\n.cfi_endproc\n.size "
        "run,.-run\n.popsection\n"
        ".hidden DW.ref.personality\n.weak DW.ref.personality\n.section "
        ".data.rel.local.DW.ref.personality,\"awG\",@progbits,DW.ref.personality,comdat\n.align "
        "8\n.type DW.ref.personality,@object\n.size "
        "DW.ref.personality,8\nDW.ref.personality:\n.quad personality\n.previous\n");
#else
static EXCEPTION_DISPOSITION handler(EXCEPTION_RECORD *record,
                                     void *frame,
                                     CONTEXT *context,
                                     DISPATCHER_CONTEXT *dispatch)
{
    (void)context;
    if (record->ExceptionCode == 0xE0426001u &&
        (record->ExceptionFlags & EXCEPTION_UNWINDING) == 0) {
        RtlUnwindEx(
            frame, (void *)landing, record, NULL, dispatch->ContextRecord, dispatch->HistoryTable);
        TerminateProcess(GetCurrentProcess(), 90);
    }
    return ExceptionContinueSearch;
}
__attribute__((used)) static EXCEPTION_DISPOSITION (*const handler_address)(EXCEPTION_RECORD *,
                                                                            void *,
                                                                            CONTEXT *,
                                                                            DISPATCHER_CONTEXT *) =
    handler; /**< Link-time handler retention for assembler reference. */
__attribute__((noinline)) void restart(void)
{
    RaiseException(0xE0426001u, EXCEPTION_NONCONTINUABLE, 0, NULL);
    TerminateProcess(GetCurrentProcess(), 91);
}
__asm__(".text\n.global run\n.def run; .scl 2; .type 32; .endef\n.seh_proc run\nrun:\n"
        "endbr64\nsub $56,%rsp\n.seh_stackalloc 56\nmov %r12,40(%rsp)\n.seh_savereg r12,40\n"
        "mov %r13,48(%rsp)\n.seh_savereg r13,48\n.seh_handler handler,@except\n.seh_endprologue\n"
        "mov %rcx,%r12\nmov %rdx,%r13\n.global landing\nlanding:\nendbr64\nmov %r13,%rcx\ncall "
        "*%r12\nud2\n.seh_endproc\n");
#endif
static unsigned count;    /**< Probe entry count; single actor. */
static uintptr_t initial; /**< First physical stack observation; single actor. */
__attribute__((noinline)) void nested(unsigned depth)
{
    volatile unsigned scratch[32];
    scratch[0] = depth;
    if (depth == 100) {
        return;
    }
    if (depth) {
        nested(depth - 1);
    } else {
        restart();
    }
    scratch[0]++;
}
__attribute__((noinline)) static void entry(void *argument)
{
    (void)argument;
    uintptr_t sp;
    __asm__ volatile("mov %%rsp,%0" : "=r"(sp));
    if (!count) {
        initial = sp;
    }
    if (initial != sp) {
        fprintf(stderr,
                "stack initial=%lx actual=%lx count=%u\n",
                (unsigned long)initial,
                (unsigned long)sp,
                count);
        abort();
    }
    if (++count == 10000) {
        puts("restart iterations=10000 stack=stable");
        exit(0);
    }
    nested(count % 31);
}
int main(int argc, char **argv)
{
#ifndef _WIN32
    if (argc > 1 && argv[1][0] == 'c') {
        long result;
        unsigned long status = 0;
        __asm__ volatile("syscall"
                         : "=a"(result)
                         : "a"(158L), "D"(0x5001L), "S"(1L)
                         : "rcx", "r11", "memory");
        printf("CET enable=%ld\n", result);
        fflush(stdout);
        if (result) {
            return 77;
        }
        __asm__ volatile("syscall"
                         : "=a"(result)
                         : "a"(158L), "D"(0x5005L), "S"(&status)
                         : "rcx", "r11", "memory");
        printf("CET status=%lu result=%ld\n", status, result);
        fflush(stdout);
        if (result || !(status & 1)) {
            return 78;
        }
    }
#else
    (void)argc;
    (void)argv;
    (void)handler_address;
#endif
    run(entry, 0);
}
