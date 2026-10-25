/** @file Observe real GNU unwind preparation before native OS threads exist. */
#include "Os_PortRestart.h"
#include <pthread.h>
#include <unwind.h>

int lifecycle_main(int argc, char **argv);
#define main lifecycle_main
#include "../../runtime/os/tests/lifecycle.c"
#undef main

static unsigned trace_calls; /**< Startup-thread observations before native creation. */
static unsigned frames;      /**< Actual provider trace callbacks, startup-thread owned. */
static int prepared;         /**< Published by successful preparation before pthread_create. */
static int fail_trace;       /**< Main-selected failure, immutable after StartOS. */
static unsigned creates;     /**< Atomic native creation observations. */
static _Unwind_Trace_Fn selected_trace; /**< Borrowed during the startup backtrace. */
static void *selected_argument;         /**< Borrowed during the startup backtrace. */

extern _Unwind_Reason_Code __real__Unwind_Backtrace(_Unwind_Trace_Fn trace, void *argument);
extern int __real_pthread_create(pthread_t *thread,
                                 const pthread_attr_t *attributes,
                                 void *(*entry)(void *context),
                                 void *argument);
extern void __real_StartupHook(void);

static void reject(unsigned code, const char *stage)
{
    fprintf(stderr, "UNWIND_PREPARE %s trace_calls=%u frames=%u\n", stage, trace_calls, frames);
    (void)fflush(stderr);
    _exit((int)code);
}

static _Unwind_Reason_Code observe_frame(struct _Unwind_Context *context, void *argument)
{
    (void)argument;
    ++frames;
    return selected_trace(context, selected_argument);
}

_Unwind_Reason_Code __wrap__Unwind_Backtrace(_Unwind_Trace_Fn trace, void *argument)
{
    _Unwind_Reason_Code status;
    ++trace_calls;
    if (fail_trace != 0) {
        fputs("UNWIND_PREPARE injected_failure\n", stderr);
        (void)fflush(stderr);
        status = _URC_FATAL_PHASE1_ERROR;
    } else {
        selected_trace = trace;
        selected_argument = argument;
        status = __real__Unwind_Backtrace(observe_frame, NULL);
        prepared = ((status == _URC_END_OF_STACK) && (frames != 0u)) ? 1 : 0;
    }
    return status;
}

int __wrap_pthread_create(pthread_t *thread,
                          const pthread_attr_t *attributes,
                          void *(*entry)(void *context),
                          void *argument)
{
    if (trace_calls == 0u) {
        reject(81u, "thread_before_trace");
    }
    if (prepared == 0) {
        reject(82u, "thread_after_failed_trace");
    }
    (void)__atomic_fetch_add(&creates, 1u, __ATOMIC_RELAXED);
    return __real_pthread_create(thread, attributes, entry, argument);
}

void __wrap_StartupHook(void)
{
    if ((trace_calls != 1u) || (frames == 0u) || (prepared == 0) ||
        (__atomic_load_n(&creates, __ATOMIC_RELAXED) == 0u)) {
        reject(83u, "startup_without_preparation");
    }
    fprintf(stderr, "UNWIND_PREPARE ready trace_calls=1 frames=%u before_threads=1\n", frames);
    (void)fflush(stderr);
    __real_StartupHook();
}

int main(int argc, char **argv)
{
    if ((argc != 2) || ((strcmp(argv[1], "normal") != 0) && (strcmp(argv[1], "error") != 0))) {
        return 90;
    }
    fail_trace = (strcmp(argv[1], "error") == 0) ? 1 : 0;
    return lifecycle_main(1, argv);
}
