/** @file Registration publication and caller ISR identity without asynchronous suspension. */
#include "Os_Backend.h"
#include <pthread.h>
#include <sched.h>
#include <stdio.h>

static int registered;
static int request;
static int result;

static void *host(void *argument)
{
    (void)argument;
    if (Os_StackRegister('H') == 0) {
        __atomic_store_n(&registered, -1, __ATOMIC_RELAXED);
        return NULL;
    }
    /* Scheduling notification deliberately does not publish the stack fields. */
    __atomic_store_n(&registered, 1, __ATOMIC_RELAXED);
    while (__atomic_load_n(&request, __ATOMIC_ACQUIRE) == 0) {
        sched_yield();
    }
    result = Os_BackendCurrentInterrupt() == 32u && Os_InterruptDisabled() == 0;
    return NULL;
}

int main(void)
{
    pthread_t thread;
    CONTEXT context = {0};
    char marker;
    Os_StackPrepare();
    if (pthread_create(&thread, NULL, &host, NULL) != 0) {
        return 2;
    }
    while (__atomic_load_n(&registered, __ATOMIC_RELAXED) == 0) {
        sched_yield();
    }
    if (__atomic_load_n(&registered, __ATOMIC_RELAXED) != 1) {
        return 3;
    }
    Os_StackInit();
    context.ContextFlags = CONTEXT_CONTROL;
    context.Rsp = (uintptr_t)&marker;
    /* Scan the host's earlier slot before finding this observer's own record. */
    Os_StackObserve(GetCurrentThread(), &context);
    Os_BackendInterruptEnter(OS_KERNEL_YIELD_INTERRUPT);
    if (Os_BackendCurrentInterrupt() != OS_KERNEL_YIELD_INTERRUPT) {
        return 4;
    }
    __atomic_store_n(&request, 1, __ATOMIC_RELEASE);
    if (pthread_join(thread, NULL) != 0 || result == 0) {
        return 5;
    }
    Os_BackendInterruptLeave();
    if (Os_BackendCurrentInterrupt() != 32u) {
        return 6;
    }
    puts("registration=published host_isr=inactive dispatcher_isr=restored");
    return 0;
}
