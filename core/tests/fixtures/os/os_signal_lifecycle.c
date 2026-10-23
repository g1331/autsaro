/** @file Real actor suspend/resume, captured context and irreversible terminal stop. */
#include "Os_Backend.h"
#include <asm/sigcontext.h>
#include <asm/ucontext.h>
#include <string.h>
#include <stdio.h>

static HANDLE ready;
static HANDLE continued;
static volatile LONG phase;
static HANDLE detach_gate;
static HANDLE detach_done;
static pthread_t detached_native;
static volatile LONG detach_observed;

int __real_pthread_detach(pthread_t thread);

/* Linker observer retains the real call and the detector's pthread bookkeeping. */
int __wrap_pthread_detach(pthread_t thread)
{
    const int result = __real_pthread_detach(thread);
    if (result == 0) {
        detached_native = thread;
        (void)InterlockedExchange(&detach_observed, 1);
    }
    return result;
}

static DWORD WINAPI actor(void *argument)
{
    (void)argument;
    if (Os_HostSetEvent(ready) == 0) {
        return 2u;
    }
    while (InterlockedCompareExchange(&phase, 0, 0) == 0) {
        Sleep(1u); /* Actor workload; readiness and progress use events. */
    }
    if (Os_HostSetEvent(continued) == 0) {
        return 3u;
    }
    for (;;) {
        if (InterlockedCompareExchange(&phase, 0, 0) > 1 && Os_HostSetEvent(continued) == 0) {
            return 3u;
        }
        Sleep(1u);
    }
    return 0u;
}

static int valid_context(HANDLE thread, CONTEXT *context)
{
    uintptr_t low;
    uintptr_t high;
    size_t guard;
    return GetThreadContext(thread, context) != 0 && context->ContextFlags == CONTEXT_CONTROL &&
           Os_HostStackRange(thread, &low, &high, &guard) != 0 && context->Rsp >= low + guard &&
           context->Rsp < high;
}

static DWORD WINAPI detached_actor(void *argument)
{
    (void)argument;
    if (WaitForSingleObject(detach_gate, 2000u) != WAIT_OBJECT_0) {
        return 2u;
    }
    if (InterlockedCompareExchange(&detach_observed, 0, 0) != 1 ||
        pthread_equal(detached_native, pthread_self()) == 0 || Os_HostSetEvent(detach_done) == 0) {
        return 4u;
    }
    return 0u;
}

static int detached_close(void)
{
    HANDLE thread;
    detach_gate = CreateEventA(NULL, TRUE, FALSE, NULL);
    detach_done = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (detach_gate == NULL || detach_done == NULL) {
        return 0;
    }
    thread = CreateThread(NULL, 262144u, detached_actor, NULL, 0u, NULL);
    return thread != NULL && CloseHandle(thread) != FALSE && Os_HostSetEvent(detach_gate) != 0 &&
           WaitForSingleObject(detach_done, 2000u) == WAIT_OBJECT_0;
}

int main(void)
{
    HANDLE thread;
    HANDLE running;
    CONTEXT paused = {0};
    CONTEXT stopped = {0};
    Os_StackPrepare();
    Os_StackInit();
    ready = CreateEventA(NULL, TRUE, FALSE, NULL);
    continued = CreateEventA(NULL, TRUE, FALSE, NULL);
    thread = CreateThread(NULL, 262144u, actor, NULL, 0u, NULL);
    if (ready == NULL || continued == NULL || thread == NULL ||
        WaitForSingleObject(ready, 2000u) != WAIT_OBJECT_0) {
        return 3;
    }
    if (SuspendThread(thread) != 0u || valid_context(thread, &paused) == 0) {
        return 4;
    }
    (void)InterlockedExchange(&phase, 1);
    if (ResumeThread(thread) != 1u || WaitForSingleObject(continued, 2000u) != WAIT_OBJECT_0 ||
        SuspendThread(thread) != 0u || valid_context(thread, &paused) == 0) {
        return 5;
    }
    for (unsigned round = 0u; round < 4u; ++round) {
        if (ResetEvent(continued) == 0) {
            return 18;
        }
        (void)InterlockedExchange(&phase, (LONG)round + 2);
        if (WaitForSingleObject(continued, 100u) != WAIT_TIMEOUT) {
            return 17;
        }
        if (ResumeThread(thread) != 1u || WaitForSingleObject(continued, 2000u) != WAIT_OBJECT_0 ||
            SuspendThread(thread) != 0u || valid_context(thread, &paused) == 0) {
            return 14;
        }
    }
    if (Os_HostStopActor(thread, 2000u) == 0 || valid_context(thread, &stopped) == 0) {
        return 6;
    }
    struct ucontext paused_native;
    struct ucontext stopped_native;
    memcpy(&paused_native, &paused.native, sizeof(paused_native));
    memcpy(&stopped_native, &stopped.native, sizeof(stopped_native));
    if (paused.Rsp != stopped.Rsp ||
        paused_native.uc_mcontext.rip != stopped_native.uc_mcontext.rip) {
        return 6;
    }
    if (ResumeThread(thread) != UINT32_MAX || SuspendThread(thread) != UINT32_MAX ||
        Os_HostStopActor(thread, 2000u) == 0 || WaitForSingleObject(thread, 0u) != WAIT_TIMEOUT) {
        return 7;
    }
    if (ResetEvent(ready) == 0) {
        return 8;
    }
    running = CreateThread(NULL, 262144u, actor, NULL, 0u, NULL);
    if (running == NULL || WaitForSingleObject(ready, 2000u) != WAIT_OBJECT_0 ||
        Os_HostStopActor(running, 2000u) == 0 || valid_context(running, &stopped) == 0 ||
        ResumeThread(running) != UINT32_MAX) {
        return 9;
    }
    if (detached_close() == 0) {
        return 10;
    }
    puts("suspend=acknowledged resume=progress context=preserved terminal=irreversible "
         "running=stopped closed=detached");
    return 0;
}
