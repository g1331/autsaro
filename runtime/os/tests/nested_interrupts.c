#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static char trace[128];
static unsigned length;
static unsigned children, grandchildren, helpers, errors, depth, maximum;
static unsigned error_isrs[8];
static StatusType error_codes[8];
static OSServiceIdType error_services[8];
static DWORD isr_thread;
static uintptr_t stack_marks[32];
static TaskStateType parent_state = 77u;
static bool is(const char *name) { return strcmp(scenario, name) == 0; }
static void check(bool condition) {
    if (!condition) {
        Os_BackendShutdown(E_OS_STATE);
    }
}
static void record(char marker) {
    check(length + 1u < sizeof(trace));
    trace[length++] = marker;
    trace[length] = '\0';
}
static void enter(unsigned expected, const volatile char *marker) {
    const Os_NativeStack *stack = Os_StackCurrent();
    const uintptr_t address = (uintptr_t)marker;
    check(stack != NULL && stack->role == 'S');
    check(Os_BackendCurrentInterrupt() == expected);
    if (depth == 0u) {
        isr_thread = GetCurrentThreadId();
    } else {
        check(isr_thread == GetCurrentThreadId() && address < stack_marks[depth - 1u]);
    }
    check(depth < 32u);
    stack_marks[depth++] = address;
    if (depth > maximum) {
        maximum = depth;
    }
}
static void leave(unsigned expected) {
    check(depth != 0u && Os_BackendCurrentInterrupt() == expected && helpers == 0u);
    --depth;
}
static void pend(unsigned interrupt) {
    if (is("native-call")) {
        vPortGenerateSimulatedInterrupt(interrupt);
    } else {
        Os_PortPostInterrupt(interrupt);
    }
}
void ErrorHook(StatusType error) {
    const OSServiceIdType service = OSErrorGetServiceId();
    const unsigned interrupt = Os_BackendCurrentInterrupt();
    check(errors < 8u);
    error_isrs[errors] = interrupt;
    error_codes[errors] = error;
    error_services[errors++] = service;
    if (is("hook") && interrupt == 20u) {
        check(error == 3u && service == 0x81u && GetISRID() == 20u);
        check(OSError_GetTaskState_TaskID() == 99u);
        check(OSError_GetTaskState_State() == &parent_state && parent_state == 77u);
        record('E');
        pend(6u);
        check(children == 0u);
        pend(4u);
        check(grandchildren == 1u && children == 0u);
        check(Os_HookContext() == OS_HOOK_ERROR && GetISRID() == 20u);
        check(OSErrorGetServiceId() == service && OSError_GetTaskState_TaskID() == 99u);
        check(OSError_GetTaskState_State() == &parent_state && parent_state == 77u);
        record('e');
    }
}
static uint32_t grandchild(void) {
    volatile char marker = 0;
    enter(4u, &marker);
    ++grandchildren;
    record('G');
    if (is("priority-order") || is("cat1-outer")) {
        leave(4u);
        return 0u;
    }
    if (is("hook")) {
        TaskStateType output = 66u;
        check(Os_HookContext() == OS_HOOK_NONE && OSErrorGetServiceId() == 0u);
        check(GetISRID() == INVALID_ISR);
        check(GetTaskState(99u, &output) == 2u && output == 66u);
        check(errors == 1u && children == 0u);
    } else if (is("mask-os") || is("mask-all") || is("mask-cat1-leak")) {
        check(Os_InterruptDisabled() == 0);
        SuspendAllInterrupts();
        if (!is("mask-cat1-leak")) {
            ResumeAllInterrupts();
        }
    } else {
        check(GetISRID() == 4u && GetResource(2u) == E_OK);
        check(ReleaseResource(2u) == E_OK);
    }
    leave(4u);
    return 0u;
}
static uint32_t child(void) {
    volatile char marker = 0;
    enter(6u, &marker);
    ++children;
    record('C');
    check(GetISRID() == 6u && Os_HookContext() == OS_HOOK_NONE);
    if (is("priority-order")) {
        leave(6u);
        return 0u;
    }
    if (is("resources") || is("resource-leak") || is("ceiling")) {
        check(GetResource(1u) == E_OK);
        check(ReleaseResource(0u) == 5u);
    }
    if (is("nested") || is("native-call") || is("resources") || is("resource-leak") ||
        is("ceiling")) {
        pend(4u);
        check(grandchildren == 1u && GetISRID() == 6u);
    }
    if (is("resources") || is("ceiling")) {
        check(ReleaseResource(1u) == E_OK);
    }
    if (is("task-ceiling")) {
        check(ReleaseResource(0u) == 5u);
    }
    check(ActivateTask(1u) == E_OK && helpers == 0u);
    if (is("child-mask-leak")) {
        SuspendAllInterrupts();
    }
    record('c');
    leave(6u);
    return 0u;
}
static uint32_t parent(void) {
    volatile char marker = 0;
    enter(20u, &marker);
    record('P');
    if (is("priority-order")) {
        leave(20u);
        return 0u;
    }
    if (is("hook")) {
        check(GetTaskState(99u, &parent_state) == 3u && parent_state == 77u);
        check(children == 1u && Os_HookContext() == OS_HOOK_NONE);
    } else if (is("mask-os") || is("mask-all") || is("mask-cat1-leak")) {
        if (is("mask-all")) {
            SuspendAllInterrupts();
        } else {
            SuspendOSInterrupts();
        }
        pend(6u);
        check(children == 0u);
        pend(4u);
        check(grandchildren == (is("mask-all") ? 0u : 1u));
        check(Os_InterruptDisabled() != 0 && children == 0u);
        if (is("mask-all")) {
            ResumeAllInterrupts();
        } else {
            ResumeOSInterrupts();
        }
        check(children == 1u && grandchildren == 1u && Os_InterruptDisabled() == 0);
    } else {
        if (is("resources") || is("resource-leak") || is("ceiling")) {
            check(GetResource(0u) == E_OK);
        }
        if (strncmp(scenario, "source-", 7u) == 0) {
            check(DisableInterruptSource(6u) == E_OK);
        }
        pend(6u);
        if (is("equal") || is("lower")) {
            check(children == 0u);
        } else if (is("ceiling")) {
            check(children == 0u && ReleaseResource(0u) == E_OK && children == 1u);
        } else if (strncmp(scenario, "source-", 7u) == 0) {
            check(children == 0u);
            if (is("source-clear-pending")) {
                check(ClearPendingInterrupt(6u) == E_OK);
            }
            check(EnableInterruptSource(6u, is("source-clear") ? TRUE : FALSE) == E_OK);
            if (!is("source-enable")) {
                check(children == 0u);
                record('D');
                pend(6u);
            }
            check(children == 1u);
        } else {
            check(children == 1u);
        }
        if (is("resources") || is("resource-leak")) {
            check(ReleaseResource(0u) == E_OK);
        }
    }
    check(helpers == 0u && Os_InterruptDisabled() == 0);
    record('p');
    leave(20u);
    return 0u;
}
static uint32_t deep(void) {
    volatile char marker = 0;
    const unsigned interrupt = Os_BackendCurrentInterrupt();
    enter(interrupt, &marker);
    check(GetISRID() == interrupt);
    record('<');
    if (interrupt < 31u) {
        pend(interrupt + 1u);
    } else {
        check(ActivateTask(1u) == E_OK);
    }
    check(GetISRID() == interrupt);
    record('>');
    leave(interrupt);
    return 0u;
}
void StartupHook(void) {
    if (is("maximum")) {
        for (unsigned interrupt = 2u; interrupt < 32u; ++interrupt) {
            vPortSetInterruptHandler(interrupt, &deep);
        }
    } else {
        vPortSetInterruptHandler(20u, &parent);
        vPortSetInterruptHandler(6u, &child);
        vPortSetInterruptHandler(4u, &grandchild);
    }
}
void ShutdownHook(StatusType error) {
    printf("nested scenario=%s children=%u grandchildren=%u helpers=%u errors=%u depth=%u "
           "maximum=%u trace=%s reason=%u\n",
           scenario, children, grandchildren, helpers, errors, depth, maximum, trace, error);
    for (unsigned i = 0u; i < errors; ++i) {
        printf("nested_error isr=%u status=%u service=%u\n", error_isrs[i], error_codes[i],
               error_services[i]);
    }
}
static void helper(void) {
    check(depth == 0u && GetISRID() == INVALID_ISR);
    ++helpers;
    record('H');
    (void)TerminateTask();
    Os_BackendShutdown(E_OS_STATE);
}
static void owner(void) {
    record('A');
    if (is("priority-order")) {
        SuspendAllInterrupts();
        vPortGenerateSimulatedInterrupt(4u);
        vPortGenerateSimulatedInterrupt(20u);
        vPortGenerateSimulatedInterrupt(6u);
        check(children == 0u && grandchildren == 0u);
        ResumeAllInterrupts();
    } else if (is("task-ceiling")) {
        check(GetResource(0u) == E_OK && uxTaskPriorityGet(NULL) == 31u);
        vPortGenerateSimulatedInterrupt(6u);
        check(children == 1u && helpers == 0u && uxTaskPriorityGet(NULL) == 31u);
        check(ReleaseResource(0u) == E_OK && uxTaskPriorityGet(NULL) == 10u);
    } else {
        vPortGenerateSimulatedInterrupt(is("maximum") ? 2u : 20u);
    }
    if (is("cat1-outer")) {
        check(helpers == 0u);
        record('T');
        /* OSEK Schedule is a no-op for a FULL Task without internal resource. */
        check(Schedule() == E_OK && helpers == 0u);
        vPortGenerateSimulatedInterrupt(4u);
    }
    check(depth == 0u && GetISRID() == INVALID_ISR);
    check(helpers == (is("priority-order") ? 0u : 1u));
    record('B');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    Os_TaskConfig tasks[2] = {
        {0u, "owner", owner, 10u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "helper", helper, 20u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    Os_IsrConfig interrupts = {.priorities = {[4u] = 15u, [6u] = 9u, [20u] = 3u}};
    Os_ResourceConfig resources[3] = {
        {0u, 3u, 1u, UINT32_C(1) << 20u},
        {1u, 9u, 0u, UINT32_C(1) << 6u},
        {2u, 15u, 0u, UINT32_C(1) << 4u},
    };
    const Os_HookConfig hooks = {&ErrorHook, NULL, NULL};
    Os_TargetConfig target = {tasks, 2u, 262144u, resources, 3u,     NULL,       0u,
                              0u,    0u, 0u,      NULL,      &hooks, &interrupts};
    if (is("equal") || is("lower")) {
        interrupts.priorities[6u] = is("equal") ? 3u : 2u;
    } else if (is("priority-order")) {
        interrupts.priorities[4u] = 3u;
        interrupts.priorities[6u] = 15u;
        interrupts.priorities[20u] = 9u;
        target.resource_count = 0u;
    } else if (is("maximum")) {
        for (unsigned interrupt = 2u; interrupt < 32u; ++interrupt) {
            interrupts.priorities[interrupt] = (uint8_t)(interrupt - 1u);
        }
        target.resource_count = 0u;
    } else if (is("ceiling")) {
        resources[0].isr_access |= UINT32_C(1) << 6u;
        resources[0].ceiling = 9u;
    } else if (is("hook") || is("mask-os") || is("mask-all") || is("mask-cat1-leak")) {
        target.category1_isrs = UINT32_C(1) << 4u;
        target.resource_count = 2u;
    } else if (is("cat1-outer")) {
        target.category1_isrs = UINT32_C(1) << 20u;
        target.resource_count = 0u;
    }
    if (strncmp(scenario, "reject-", 7u) == 0) {
        if (is("reject-priority")) {
            interrupts.priorities[20u] = 32u;
        } else if (is("reject-yield")) {
            interrupts.priorities[0u] = 1u;
        } else if (is("reject-tick")) {
            interrupts.priorities[1u] = 1u;
        } else if (is("reject-category")) {
            target.category1_isrs = UINT32_C(1) << 5u;
        } else if (is("reject-resource")) {
            resources[0].isr_access |= UINT32_C(1) << 5u;
        } else if (is("reject-ceiling")) {
            resources[0].ceiling = 2u;
        } else if (is("reject-input")) {
            tasks[0].kind = OS_EXTENDED_TASK;
            target.input_event = 1u;
        } else {
            return 98;
        }
        const StatusType status = Os_TargetPrepare(&target);
        printf("nested_rejection scenario=%s status=%u configured=%u\n", scenario, status,
               Os_Config != NULL ? 1u : 0u);
        return status == E_OS_VALUE && Os_Config == NULL ? 0 : 98;
    }
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
