#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static bool configured;
static unsigned errors;
static unsigned deliveries;
static unsigned parents;
static char trace[16];
static unsigned trace_length;
static StatusType expected_error;
static OSServiceIdType expected_service;
static ISRType expected_id;
static boolean expected_clear;
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
static bool is(const char *value) { return strcmp(scenario, value) == 0; }
static void record(char value) {
    check(trace_length + 1u < sizeof(trace));
    trace[trace_length++] = value;
    trace[trace_length] = '\0';
}
void ErrorHook(StatusType error) {
    const OSServiceIdType service = OSErrorGetServiceId();
    const ISRType caller = GetISRID();
    check(error == expected_error && service == expected_service && errors == 0u);
    if (service == 48u) {
        check(OSError_DisableInterruptSource_ISRID() == expected_id);
    } else {
        check(service == 49u && OSError_EnableInterruptSource_ISRID() == expected_id &&
              OSError_EnableInterruptSource_ClearPending() == expected_clear);
    }
    check(caller == (strncmp(scenario, "isr-", 4u) == 0 ? 5u : INVALID_ISR));
    ++errors;
    printf("source_repeat_error status=%u service=%u id=%u clear=%u caller=%u\n", error, service,
           expected_id, expected_clear, caller);
}
static uint32_t receiver(void) {
    check(GetISRID() == 6u && Os_PortInterruptSourceEnabled(6u) != 0);
    ++deliveries;
    record('D');
    return 0u;
}
static void repeated_enable(boolean clear) {
    expected_error = 5u;
    expected_service = 49u;
    expected_id = 6u;
    expected_clear = clear;
    check(EnableInterruptSource(6u, clear) == E_OS_NOFUNC);
    check(Os_PortInterruptSourceEnabled(6u) != 0 && deliveries == 0u);
}
static void repeated_disable(void) {
    expected_error = 5u;
    expected_service = 48u;
    expected_id = 6u;
    expected_clear = FALSE;
    check(DisableInterruptSource(6u) == E_OS_NOFUNC);
    check(Os_PortInterruptSourceEnabled(6u) == 0 && deliveries == 0u);
}
static uint32_t parent(void) {
    ++parents;
    record('P');
    if (is("isr-disable")) {
        check(DisableInterruptSource(6u) == E_OK);
        vPortGenerateSimulatedInterrupt(6u);
        repeated_disable();
        check(EnableInterruptSource(6u, FALSE) == E_OK);
    } else {
        vPortGenerateSimulatedInterruptFromWindowsThread(6u);
        repeated_enable(is("isr-enable-true") ? TRUE : FALSE);
    }
    check(deliveries == 0u && GetISRID() == 5u);
    record('p');
    return 0u;
}
void StartupHook(void) {
    vPortSetInterruptHandler(5u, &parent);
    vPortSetInterruptHandler(6u, &receiver);
}
void ShutdownHook(StatusType reason) {
    printf("source_repeat scenario=%s deliveries=%u parents=%u errors=%u trace=%s reason=%u\n",
           scenario, deliveries, parents, errors, trace, reason);
}
static void owner(void) {
    if (strncmp(scenario, "isr-", 4u) == 0) {
        vPortGenerateSimulatedInterrupt(5u);
    } else if (is("clear-repeat")) {
        SuspendAllInterrupts();
        vPortGenerateSimulatedInterrupt(6u);
        check(ClearPendingInterrupt(6u) == E_OK);
        check(ClearPendingInterrupt(6u) == E_OK);
        ResumeAllInterrupts();
        check(deliveries == 0u);
        vPortGenerateSimulatedInterrupt(6u);
    } else if (strncmp(scenario, "invalid-", 8u) == 0) {
        expected_error = 3u;
        expected_service = 49u;
        expected_id = is("invalid-kernel") ? 1u : 31u;
        expected_clear = TRUE;
        check(EnableInterruptSource(expected_id, TRUE) == E_OS_ID);
        check(Os_PortInterruptSourceEnabled(6u) != 0 && deliveries == 0u);
        vPortGenerateSimulatedInterrupt(6u);
    } else if (is("disable") || is("disable-all")) {
        check(DisableInterruptSource(6u) == E_OK);
        if (is("disable-all")) {
            SuspendAllInterrupts();
        }
        vPortGenerateSimulatedInterrupt(6u);
        repeated_disable();
        if (is("disable-all")) {
            ResumeAllInterrupts();
            check(deliveries == 0u);
        }
        check(EnableInterruptSource(6u, FALSE) == E_OK);
    } else if (is("default-enable")) {
        repeated_enable(FALSE);
        vPortGenerateSimulatedInterrupt(6u);
    } else {
        SuspendAllInterrupts();
        vPortGenerateSimulatedInterrupt(6u);
        repeated_enable(is("enable-false") ? FALSE : TRUE);
        ResumeAllInterrupts();
    }
    check(deliveries == 1u);
    check(errors == ((!configured || is("clear-repeat")) ? 0u : 1u));
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    configured = !is("unconfigured");
    const Os_TaskConfig task = {0u, "owner",          owner, 10u, 1u, OS_BASIC_TASK,
                                1u, OS_SCHEDULE_FULL, 0u};
    const Os_HookConfig hooks = {configured ? &ErrorHook : NULL, NULL, NULL};
    const Os_TargetConfig target = {&task, 1u, 262144u, NULL, 0u,     NULL, 0u,
                                    0u,    0u, 0u,      NULL, &hooks, NULL};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
