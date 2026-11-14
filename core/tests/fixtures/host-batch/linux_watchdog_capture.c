/** @file Freeze a failing HostBatch before shutdown mutates its handshake state. */
#include "Os_Types.h"
#include <signal.h>
#include <sys/prctl.h>
#include <unistd.h>

enum { CAPTURE_SETUP_FAILED = 126 };

void __real_Os_BackendShutdown(StatusType error);
void __real_Os_BackendRequestShutdown(StatusType error);
void __real_Ecu_TargetAbortNative(StatusType reason);

static void stop_before_close(StatusType error)
{
    if (error == E_OS_STATE) {
        /* Every failing caller stops before delegation; no competing caller closes first. */
        /* Restrict post-failure inspection to this test driver and its debugger. */
        if (prctl(PR_SET_PTRACER, (unsigned long)getppid(), 0UL, 0UL, 0UL) != 0) {
            _exit(CAPTURE_SETUP_FAILED);
        }
        /* Target this caller so it cannot delegate before the group stops. */
        if (raise(SIGSTOP) != 0) {
            _exit(CAPTURE_SETUP_FAILED);
        }
    }
}

void __wrap_Os_BackendShutdown(StatusType error)
{
    stop_before_close(error);
    __real_Os_BackendShutdown(error);
}

void __wrap_Os_BackendRequestShutdown(StatusType error)
{
    stop_before_close(error);
    __real_Os_BackendRequestShutdown(error);
}

void __wrap_Ecu_TargetAbortNative(StatusType reason)
{
    stop_before_close((reason == E_OK) ? E_OS_STATE : reason);
    __real_Ecu_TargetAbortNative(reason);
}
