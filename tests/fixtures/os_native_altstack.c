/** @file Finite host actor alternate-stack ownership across pthread retirement. */
#define _GNU_SOURCE
#include "Os_Host.h"
#include "Os_LinuxNative.h"
#include "Os_Stack.h"
#include "Os.h"
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static const char *scenario;
static stack_t expected;
static unsigned stage;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

static void reject(unsigned code, const char *reason)
{
    (void)fprintf(stderr, "altstack_rejected stage=%u reason=%s\n", stage, reason);
    (void)fflush(stderr);
    _exit((int)code);
}

static int same_stack(const stack_t *left, const stack_t *right)
{
    return (left->ss_sp == right->ss_sp) && (left->ss_size == right->ss_size) &&
           (left->ss_flags == right->ss_flags);
}

int __real_Os_LinuxAltStack(const stack_t *stack, stack_t *previous);
int __wrap_Os_LinuxAltStack(const stack_t *stack, stack_t *previous)
{
    int result;
    if (stack == NULL) {
        if ((stage != 0u) || (previous == NULL) || (sigaltstack(NULL, &expected) != 0)) {
            reject(21u, "query");
        }
        if (strcmp(scenario, "query-error") == 0) {
            (void)fputs("altstack_injected query\n", stderr);
            return -EIO;
        }
        result = __real_Os_LinuxAltStack(stack, previous);
        if ((result != 0) || !same_stack(previous, &expected)) {
            reject(22u, "snapshot");
        }
        stage = 1u;
    } else if (stage == 1u) {
        if ((previous != NULL) || (stack->ss_flags != 0) || (stack->ss_size != 65536u) ||
            (stack->ss_sp == expected.ss_sp)) {
            reject(22u, "registration");
        }
        result = __real_Os_LinuxAltStack(stack, previous);
        if (result != 0) {
            reject(22u, "registration-call");
        }
        stage = 2u;
    } else if (stage == 3u) {
        if ((previous != NULL) || !same_stack(stack, &expected)) {
            reject(23u, "restoration");
        }
        if (strcmp(scenario, "restore-error") == 0) {
            (void)fputs("altstack_injected restore\n", stderr);
            return -EPERM;
        }
        result = __real_Os_LinuxAltStack(stack, previous);
        stack_t actual = {0};
        if ((result != 0) || (sigaltstack(NULL, &actual) != 0) || !same_stack(&actual, &expected)) {
            reject(23u, "restoration-call");
        }
        stage = 4u;
    } else {
        reject(21u, "sequence");
        result = -EINVAL;
    }
    return result;
}

static DWORD entry(void *argument)
{
    stack_t actual = {0};
    (void)argument;
    if (strcmp(scenario, "observer") == 0) {
        if (stage != 0u) {
            reject(24u, "observer");
        }
    } else {
        if ((stage != 2u) || (sigaltstack(NULL, &actual) != 0) || (actual.ss_flags != 0) ||
            (actual.ss_size != 65536u) || (actual.ss_sp == expected.ss_sp)) {
            reject(24u, "callback");
        }
        stage = 3u;
    }
    return 37u;
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        return 20;
    }
    scenario = argv[1];
    Os_StackPrepare();
    const int observer = strcmp(scenario, "observer") == 0;
    HANDLE actor = Os_HostCreateActor(262144u, &entry, NULL, 0u, NULL, observer == 0);
    DWORD code = 0u;
    if ((actor == NULL) || (WaitForSingleObject(actor, 2000u) != WAIT_OBJECT_0) ||
        (GetExitCodeThread(actor, &code) == FALSE) || (code != 37u)) {
        reject(25u, "completion");
    }
    if (stage != (observer != 0 ? 0u : 4u)) {
        reject(24u, "completion-state");
    }
    if (CloseHandle(actor) == FALSE) {
        reject(25u, "close");
    }
    (void)printf("altstack actor=%s stage=%u joined=1 closed=1\n", scenario, stage);
    return 0;
}
