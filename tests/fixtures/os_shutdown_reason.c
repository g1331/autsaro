/** @file Interleave an ordinary close with a real native H-actor guard fault. */
#define StartupHook             OrdinaryStartupHook
#define Os_StackTestBeforeClose OrdinaryBeforeClose
#define main                    OrdinaryMain
#include "native_stack.c"
#undef StartupHook
#undef Os_StackTestBeforeClose
#undef main
#include "Os_HostIo.h"

static volatile Os_Atomic32Type before_reason; /**< Test-only publication handshake. */
static volatile Os_Atomic32Type after_reason;  /**< Two completed reason writes. */

static boolean reason_writer(StatusType value)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    return (stack != NULL) && (((stack->role == 'B') && (value == E_OS_STATE)) ||
                               ((stack->role == 'H') && (value == E_OS_STACKFAULT)));
}

void Os_TestBeforeReasonStore(StatusType value)
{
    if (reason_writer(value)) {
        const Os_NativeStackType *stack = Os_StackCurrent();
        if (stack->role == 'B') {
            __atomic_store_n(&before_reason, 1, __ATOMIC_SEQ_CST);
            while (__atomic_load_n(&before_reason, __ATOMIC_SEQ_CST) != 2) {
                /* Keep both real stores after the common handshake. */
            }
        } else {
            if (__atomic_load_n(&before_reason, __ATOMIC_SEQ_CST) != 1) {
                ExitProcess(91u);
            }
            __atomic_store_n(&before_reason, 2, __ATOMIC_SEQ_CST);
        }
    }
}

void Os_TestAfterReasonStore(StatusType value)
{
    if (reason_writer(value)) {
        (void)__atomic_add_fetch(&after_reason, 1, __ATOMIC_SEQ_CST);
    }
}

static Os_HostThreadResultType fault_input(void *argument)
{
    (void)argument;
    while (__atomic_load_n(&before_reason, __ATOMIC_SEQ_CST) != 1) {
        /* The ordinary requester must have won admission before the fault. */
    }
    const Os_NativeStackType *stack = Os_StackCurrent();
    if ((stack == NULL) || (stack->role != 'H')) {
        ExitProcess(92u);
    }
    volatile unsigned char *guard = (volatile unsigned char *)(stack->reserve_low + 8u);
    *guard = 42u;
    ExitProcess(93u);
}

void StartupHook(void)
{
    OrdinaryStartupHook();
    HANDLE actor = CreateThread(NULL, 0u, &fault_input, NULL, 0u, NULL);
    if ((actor == NULL) || (CloseHandle(actor) == 0)) {
        ExitProcess(94u);
    }
    Os_BackendRequestShutdown(E_OS_STATE);
    Sleep(INFINITE);
    ExitProcess(95u);
}

void Os_StackTestBeforeClose(void)
{
    while (__atomic_load_n(&after_reason, __ATOMIC_SEQ_CST) != 2) {
        Sleep(1u);
    }
    if ((Os_StackFaultCount() != 1) || (Os_Fault.role != 'H')) {
        ExitProcess(96u);
    }
    const char stage[] = "reason_publication writers=2 fault=H\n";
    if (Os_HostWriteBytes(1, stage, sizeof(stage) - 1u) == 0) {
        ExitProcess(97u);
    }
}

int main(int argc, char **argv) { return OrdinaryMain(argc, argv); }
