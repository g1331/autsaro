/** @file Native actor initialization and caller-owned join/close failure boundaries. */
#include "Os_Host.h"
#include "Os_Stack.h"
#include "Os.h"
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static const char *scenario;
static Os_HostHandleType gate;
static Os_HostHandleType completed;
static Os_HostHandleType observed_thread;
static unsigned join_calls;
static unsigned detach_calls;
static unsigned join_entered;
static unsigned release_join;
static unsigned callback_completed;
static DWORD waiter_result;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int __real_Os_StackRegister(char role);
int __wrap_Os_StackRegister(char role)
{
    if ((role == 'H') && (strcmp(scenario, "registration-error") == 0)) {
        (void)fputs("registration_rejected role=H\n", stderr);
        return 0;
    }
    return __real_Os_StackRegister(role);
}

DWORD __real_Os_LinuxThreadId(void);
DWORD __wrap_Os_LinuxThreadId(void)
{
    return strcmp(scenario, "identity-error") == 0 ? 0u : __real_Os_LinuxThreadId();
}

int __real_pthread_join(pthread_t thread, void **result);
int __wrap_pthread_join(pthread_t thread, void **result)
{
    const unsigned call = __atomic_add_fetch(&join_calls, 1u, __ATOMIC_RELAXED);
    if ((strcmp(scenario, "join-error") == 0) && (call == 1u)) {
        (void)fputs("join_rejected attempt=1\n", stderr);
        return EINVAL;
    }
    if ((strcmp(scenario, "concurrent-wait") == 0) && (call == 1u)) {
        __atomic_store_n(&join_entered, 1u, __ATOMIC_RELEASE);
        while (__atomic_load_n(&release_join, __ATOMIC_ACQUIRE) == 0u) {
            (void)sched_yield();
        }
    }
    return __real_pthread_join(thread, result);
}

int __real_pthread_detach(pthread_t thread);
int __wrap_pthread_detach(pthread_t thread)
{
    ++detach_calls;
    if ((strcmp(scenario, "detach-error") == 0) && (detach_calls == 1u)) {
        (void)fputs("detach_rejected attempt=1\n", stderr);
        return EINVAL;
    }
    return __real_pthread_detach(thread);
}

static void reject(int code, const char *stage) __attribute__((noreturn));
static void reject(int code, const char *stage)
{
    (void)fprintf(stderr, "thread_state_rejected stage=%s\n", stage);
    (void)fflush(stderr);
    /* Intentional mutants can retain joinable threads; preserve the observed failure. */
    _exit(code);
}

static DWORD actor(void *argument)
{
    (void)argument;
    if (strcmp(scenario, "registration-error") == 0) {
        (void)fputs("unexpected_actor_entry\n", stderr);
    }
    if ((gate != NULL) && (WaitForSingleObject(gate, INFINITE) != WAIT_OBJECT_0)) {
        return 40u;
    }
    if ((completed != NULL) && (Os_HostSetEvent(completed) == 0)) {
        return 41u;
    }
    __atomic_store_n(&callback_completed, 1u, __ATOMIC_RELEASE);
    return 37u;
}

static void *waiter(void *argument)
{
    (void)argument;
    waiter_result = WaitForSingleObject(observed_thread, 2000u);
    return NULL;
}

int main(int argc, char **argv)
{
    DWORD identity = 0u;
    DWORD code = 0u;
    if (argc != 2) {
        return 2;
    }
    scenario = argv[1];
    Os_StackPrepare();
    if ((strcmp(scenario, "registration-error") == 0) ||
        (strcmp(scenario, "identity-error") == 0) || (strcmp(scenario, "registered-normal") == 0)) {
        if (strcmp(scenario, "identity-error") == 0) {
            gate = CreateEventA(NULL, 1, 0, NULL);
            if (gate == NULL) {
                return 3;
            }
            (void)fputs("identity_requested\n", stderr);
        }
        HANDLE thread = CreateThread(NULL, 262144u, actor, NULL, 0u, &identity);
        if (thread == NULL) {
            (void)fputs("creation_returned_null\n", stderr);
            return 4;
        }
        if ((WaitForSingleObject(thread, 2000u) != WAIT_OBJECT_0) ||
            (GetExitCodeThread(thread, &code) == 0) || (code != 37u) ||
            (CloseHandle(thread) == 0) || (identity == 0u)) {
            return 5;
        }
        puts("registered_actor entry=completed identity=published");
        return 0;
    }
    gate = CreateEventA(NULL, 1, 0, NULL);
    completed = CreateEventA(NULL, 1, 0, NULL);
    if ((gate == NULL) || (completed == NULL)) {
        return 6;
    }
    const int registered = strcmp(scenario, "detach-error") == 0 ? 1 : 0;
    HANDLE thread = Os_HostCreateActor(262144u, actor, NULL, 0u, NULL, registered);
    if (thread == NULL) {
        return 7;
    }
    if (strcmp(scenario, "detach-error") == 0) {
        if ((CloseHandle(thread) != 0) || (CloseHandle(thread) == 0) || (detach_calls != 2u)) {
            reject(53, "detach-completion");
        }
        if ((Os_HostSetEvent(gate) == 0) ||
            (WaitForSingleObject(completed, 2000u) != WAIT_OBJECT_0)) {
            return 8;
        }
        while (__atomic_load_n(&callback_completed, __ATOMIC_ACQUIRE) == 0u) {
            (void)sched_yield();
        }
    } else {
        if ((strcmp(scenario, "close-live") == 0) && (CloseHandle(thread) != 0)) {
            reject(50, "live-close");
        }
        if ((Os_HostSetEvent(gate) == 0) ||
            (WaitForSingleObject(completed, 2000u) != WAIT_OBJECT_0)) {
            return 9;
        }
        if (strcmp(scenario, "close-ended") == 0) {
            do {
                if (GetExitCodeThread(thread, &code) == 0) {
                    return 10;
                }
                (void)sched_yield();
            } while (code == STILL_ACTIVE);
            if (CloseHandle(thread) != 0) {
                reject(52, "ended-close");
            }
        }
        if (strcmp(scenario, "join-error") == 0) {
            if (WaitForSingleObject(thread, 2000u) != WAIT_FAILED) {
                return 11;
            }
        }
        if (strcmp(scenario, "concurrent-wait") == 0) {
            pthread_t worker;
            observed_thread = thread;
            if (pthread_create(&worker, NULL, waiter, NULL) != 0) {
                return 12;
            }
            while (__atomic_load_n(&join_entered, __ATOMIC_ACQUIRE) == 0u) {
                (void)sched_yield();
            }
            if (WaitForSingleObject(thread, 0u) != WAIT_TIMEOUT) {
                reject(54, "concurrent-completion");
            }
            __atomic_store_n(&release_join, 1u, __ATOMIC_RELEASE);
            if ((pthread_join(worker, NULL) != 0) || (waiter_result != WAIT_OBJECT_0)) {
                return 13;
            }
        }
        if ((WaitForSingleObject(thread, 2000u) != WAIT_OBJECT_0) ||
            ((strcmp(scenario, "join-error") == 0) && (join_calls != 2u))) {
            reject(51, "join-completion");
        }
        if ((GetExitCodeThread(thread, &code) == 0) || (code != 37u) ||
            (CloseHandle(thread) == 0)) {
            return 14;
        }
    }
    if ((CloseHandle(gate) == 0) || (CloseHandle(completed) == 0)) {
        return 15;
    }
    (void)printf("thread_lifecycle scenario=%s ownership=retained completion=observed\n", scenario);
    return 0;
}
