/** @file Freeze a failing HostBatch before shutdown mutates its handshake state. */
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#include "Os_Types.h"
#include <stdint.h>
#include <time.h>
#include <signal.h>
#include <sys/prctl.h>
#include <unistd.h>

enum { CAPTURE_NO_CLOSE = -1, CAPTURE_SETUP_FAILED = 126 };
static int capture_first_reason = CAPTURE_NO_CLOSE;
static int capture_reason;
static volatile uint64_t capture_freeze_ns;

void Os_HostCaptureBeforeClose(StatusType error);
void __real_Os_BackendShutdown(StatusType error);
void __real_Os_BackendRequestShutdown(StatusType error);
void __real_Ecu_TargetAbortNative(StatusType reason);

static void stop_before_close(StatusType error)
{
    int expected = CAPTURE_NO_CLOSE;
    (void)__atomic_compare_exchange_n(
        &capture_first_reason, &expected, (int)error, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    const int first = __atomic_load_n(&capture_first_reason, __ATOMIC_SEQ_CST);
    /* Normal close can elicit secondary STATE requests from exiting actors.
     * Once a failing close starts, every later caller must stop before delegation. */
    if (first == E_OS_STATE) {
        __atomic_store_n(&capture_reason, first, __ATOMIC_SEQ_CST);
        struct timespec now;
        if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
            _exit(CAPTURE_SETUP_FAILED);
        }
        uint64_t expected = 0u;
        const uint64_t stamp =
            ((uint64_t)now.tv_sec * UINT64_C(1000000000)) + (uint64_t)now.tv_nsec;
        (void)__atomic_compare_exchange_n(
            &capture_freeze_ns, &expected, stamp, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
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

void Os_HostCaptureBeforeClose(StatusType error) { stop_before_close(error); }

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
