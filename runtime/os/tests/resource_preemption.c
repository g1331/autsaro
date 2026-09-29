/* Standard OS behavior with an independent native-state observer/injector. */
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static TaskHandle_t observed[6];
static unsigned observations;
static unsigned rejected;
static unsigned h_entries;
static unsigned b_entries;
static volatile LONG irq_entries;
static volatile LONG task_progress;
static volatile LONG task_started;
static volatile LONG irq_finished;
static void check(bool condition) {
    if (!condition) {
        Os_TargetTrace('!');
        ShutdownOS(E_OS_STATE);
    }
}
static Os_ActivationInfo info(TaskType id) {
    Os_ActivationInfo value;
    check(Os_TargetInspectActivation(id, &value) == E_OK);
    return value;
}
static bool same_info(const Os_ActivationInfo *a, const Os_ActivationInfo *b) {
    unsigned i;
    bool same = (a->state == b->state) && (a->count == b->count) &&
                (a->kernel_sequence == b->kernel_sequence) && (a->events == b->events) &&
                (a->effective_priority == b->effective_priority) &&
                (a->resource_count == b->resource_count) &&
                (a->internal_held == b->internal_held) &&
                (a->internal_ceiling == b->internal_ceiling) && (a->waiting == b->waiting) &&
                (a->wait_mask == b->wait_mask);
    for (i = 0u; i < OS_MAX_ACTIVATIONS; ++i) {
        same = same && (a->requests[i] == b->requests[i]);
    }
    for (i = 0u; i < OS_MAX_RESOURCES; ++i) {
        same = same && (a->resources[i] == b->resources[i]);
    }
    return same;
}
static void refused(unsigned operation, StatusType expected) {
    Os_ActivationInfo before[6];
    Os_ActivationInfo after;
    StatusType result;
    unsigned i;
    for (i = 0u; i < 6u; ++i) {
        before[i] = info((TaskType)i);
    }
    switch (operation) {
    case 0u:
        result = ReleaseResource(0u);
        break;
    case 1u:
        result = GetResource(0u);
        break;
    case 2u:
        result = Schedule();
        break;
    case 3u:
        result = WaitEvent(1u);
        break;
    case 4u:
        result = TerminateTask();
        break;
    case 5u:
        result = ChainTask(1u);
        break;
    default:
        ShutdownOS(E_OS_STATE);
        return;
    }
    check(result == expected);
    for (i = 0u; i < 6u; ++i) {
        after = info((TaskType)i);
        check(same_info(&before[i], &after));
    }
    ++rejected;
}
void Os_TestObserve(void) {
    unsigned i;
    ++observations;
    for (i = 0u; i < 6u; ++i) {
        Os_ActivationInfo value = info((TaskType)i);
        eTaskState state = eTaskGetState(observed[i]);
        check(value.effective_priority == uxTaskPriorityGet(observed[i]));
        if (value.waiting != 0u) {
            check((value.count == 1u) && (state == eSuspended) && (value.state == WAITING) &&
                  (value.internal_held == 0u) && (value.kernel_sequence == 0u));
        } else if (value.count == 0u) {
            check((state == eSuspended) && (value.state == SUSPENDED) &&
                  (value.internal_held == 0u) && (value.kernel_sequence == 0u));
        } else {
            check((state == eRunning) || (state == eReady));
            check(value.state == ((state == eRunning) ? RUNNING : READY));
            check(value.kernel_sequence == ullTaskOsReadySequence(observed[i]));
        }
        if (value.internal_held != 0u) {
            check(value.effective_priority >= value.internal_ceiling);
        }
    }
}
static void finish(void) {
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
static void check_priority(uint8_t priority, unsigned depth, uint8_t held) {
    Os_ActivationInfo value = info(0u);
    check((value.effective_priority == priority) && (value.resource_count == depth) &&
          (value.internal_held == held));
}
static uint32_t interrupt(void) {
    InterlockedIncrement(&irq_entries);
    Os_TargetTrace('J');
    check(GetResource(5u) == E_OK);
    check(GetResource(4u) == E_OK);
    check(Os_BackendInterruptEnabled(30u) == 0);
    refused(2u, E_OS_CALLEVEL);
    refused(3u, E_OS_CALLEVEL);
    refused(4u, E_OS_CALLEVEL);
    refused(5u, E_OS_CALLEVEL);
    check(ReleaseResource(5u) == E_OS_NOFUNC);
    check(GetResource(5u) == E_OS_ACCESS);
    check(Os_BackendInterruptEnabled(30u) == 0);
    if (strcmp(scenario, "external-isr") == 0) {
        LONG progress;
        /* Private physical-interval probe. The task must be synchronously
         * suspended throughout this bounded ISR observation interval. */
        progress = InterlockedCompareExchange(&task_progress, 0, 0);
        Sleep(50u);
        check(progress == InterlockedCompareExchange(&task_progress, 0, 0));
    }
    check(ReleaseResource(4u) == E_OK);
    check(ReleaseResource(5u) == E_OK);
    check(Os_BackendInterruptEnabled(30u) != 0);
    InterlockedExchange(&irq_finished, 1);
    return 0u;
}
static DWORD WINAPI injector(void *argument) {
    (void)argument;
    while (InterlockedCompareExchange(&task_started, 0, 0) == 0) {
        Sleep(1u);
    }
    vPortGenerateSimulatedInterruptFromWindowsThread(31u);
    return 0u;
}
void StartupHook(void) {
    unsigned i;
    static const char *const names[] = {"A", "B", "H", "D", "E", "M"};
    Os_TargetTrace('S');
    check(Schedule() == E_OS_CALLEVEL);
    for (i = 0u; i < 6u; ++i) {
        observed[i] = xTaskGetHandle(names[i]);
        check(observed[i] != NULL);
    }
    vPortSetInterruptHandler(31u, interrupt);
}
void ShutdownHook(StatusType Error) {
    Os_TargetTrace('Z');
    if (printf("resources h=%u b=%u irq=%ld observer=%u rejected=%u error=%u\n", h_entries,
               b_entries, InterlockedCompareExchange(&irq_entries, 0, 0), observations, rejected,
               Error) < 0) {
        ExitProcess(E_OS_STATE);
    }
}
static void task_a(void) {
    volatile unsigned local = 0x12345678u;
    Os_TargetTrace('A');
    if (strcmp(scenario, "full") == 0) {
        check_priority(2u, 0u, 0u);
        check(ActivateTask(2u) == E_OK);
        check(h_entries == 1u);
    } else if ((strcmp(scenario, "non") == 0) || (strcmp(scenario, "oracle-non") == 0)) {
        check_priority(5u, 0u, 1u);
        check(ActivateTask(2u) == E_OK);
        check(h_entries == 0u);
        Os_TargetTrace('n');
        check(Schedule() == E_OK);
        check(h_entries == 1u);
        check_priority(5u, 0u, 1u);
    } else if ((strcmp(scenario, "internal") == 0) || (strcmp(scenario, "internal-preempt") == 0)) {
        check_priority(3u, 0u, 1u);
        check(ActivateTask(2u) == E_OK);
        check(h_entries == 0u);
        if (strcmp(scenario, "internal-preempt") == 0) {
            check(ActivateTask(3u) == E_OK);
            check_priority(3u, 0u, 1u);
        }
        Os_TargetTrace('g');
        check(Schedule() == E_OK);
        check(h_entries == 1u);
        check_priority(3u, 0u, 1u);
    } else if ((strcmp(scenario, "wait") == 0) || (strcmp(scenario, "already") == 0)) {
        check_priority(3u, 0u, 1u);
        if (strcmp(scenario, "already") == 0) {
            check(SetEvent(0u, 1u) == E_OK);
        }
        check(ActivateTask(1u) == E_OK);
        check(b_entries == 0u);
        check(WaitEvent(1u) == E_OK);
        check_priority(3u, 0u, 1u);
        check(b_entries == ((strcmp(scenario, "wait") == 0) ? 1u : 0u));
        check(ClearEvent(1u) == E_OK);
    } else if ((strcmp(scenario, "nested") == 0) || (strcmp(scenario, "resource-wait") == 0)) {
        check(GetResource(0u) == E_OK);
        check_priority(3u, 1u, 0u);
        check(GetResource(1u) == E_OK);
        check(GetResource(2u) == E_OK);
        check_priority(4u, 3u, 0u);
        check(ActivateTask(2u) == E_OK);
        refused(0u, E_OS_NOFUNC);
        refused(1u, E_OS_ACCESS);
        refused(2u, E_OS_RESOURCE);
        refused(4u, E_OS_RESOURCE);
        refused(5u, E_OS_RESOURCE);
        if (strcmp(scenario, "resource-wait") == 0) {
            refused(3u, E_OS_RESOURCE);
        }
        check(ReleaseResource(2u) == E_OK);
        check_priority(4u, 2u, 0u);
        check(ReleaseResource(1u) == E_OK);
        check_priority(3u, 1u, 0u);
        check(h_entries == 0u);
        check(ReleaseResource(0u) == E_OK);
        check_priority(2u, 0u, 0u);
        check(h_entries == 1u);
    } else if (strcmp(scenario, "internal-resource") == 0) {
        check_priority(3u, 0u, 1u);
        check(GetResource(1u) == E_OK);
        check_priority(4u, 1u, 1u);
        refused(2u, E_OS_RESOURCE);
        refused(3u, E_OS_RESOURCE);
        refused(4u, E_OS_RESOURCE);
        check(ReleaseResource(1u) == E_OK);
        check_priority(3u, 0u, 1u);
    } else if (strcmp(scenario, "oracle-resource") == 0) {
        check(GetResource(0u) == E_OK);
        check_priority(3u, 1u, 0u);
        check(ActivateTask(2u) == E_OK);
        check(h_entries == 0u);
        Os_TargetTrace('r');
        check(ReleaseResource(0u) == E_OK);
        check_priority(1u, 0u, 0u);
        check(h_entries == 1u);
    } else if (strcmp(scenario, "access") == 0) {
        check(ActivateTask(1u) == E_OK);
    } else if (strcmp(scenario, "restore") == 0) {
        check(GetResource(1u) == E_OK);
        check(ActivateTask(1u) == E_OK);
        check(ActivateTask(2u) == E_OK);
        Os_TargetTrace('r');
        check(ReleaseResource(1u) == E_OK);
        check((h_entries == 1u) && (b_entries == 0u));
    } else if (strcmp(scenario, "nonowner") == 0) {
        check(GetResource(1u) == E_OK);
        check(ActivateTask(3u) == E_OK);
        check_priority(4u, 1u, 0u);
        check(ReleaseResource(1u) == E_OK);
    } else if (strcmp(scenario, "scheduler-unconfigured") == 0) {
        check_priority(2u, 0u, 0u);
        check(GetResource(RES_SCHEDULER) == E_OS_ID);
        check(ReleaseResource(RES_SCHEDULER) == E_OS_ID);
        check_priority(2u, 0u, 0u);
    } else if (strcmp(scenario, "scheduler") == 0) {
        check(GetResource(RES_SCHEDULER) == E_OK);
        check_priority(5u, 1u, 0u);
        check(ActivateTask(2u) == E_OK);
        check(h_entries == 0u);
        vPortGenerateSimulatedInterrupt(31u);
        check(InterlockedCompareExchange(&irq_entries, 0, 0) == 1);
        check(h_entries == 0u);
        Os_TargetTrace('r');
        check(ReleaseResource(RES_SCHEDULER) == E_OK);
        check(h_entries == 1u);
    } else if (strcmp(scenario, "masked-isr") == 0) {
        check(GetResource(4u) == E_OK);
        check_priority(31u, 1u, 0u);
        vPortGenerateSimulatedInterrupt(31u);
        check(InterlockedCompareExchange(&irq_entries, 0, 0) == 0);
        Os_TargetTrace('p');
        check(ReleaseResource(4u) == E_OK);
        check(InterlockedCompareExchange(&irq_entries, 0, 0) == 1);
        check_priority(2u, 0u, 0u);
    } else if (strcmp(scenario, "external-isr") == 0) {
        HANDLE thread = CreateThread(NULL, 0u, injector, NULL, 0u, NULL);
        check(thread != NULL);
        InterlockedExchange(&task_started, 1);
        while (InterlockedCompareExchange(&irq_finished, 0, 0) == 0) {
            InterlockedIncrement(&task_progress);
        }
        check(CloseHandle(thread) != FALSE);
    } else {
        ShutdownOS(E_OS_STATE);
    }
    check(local == 0x12345678u);
    Os_TargetTrace('a');
    finish();
}
static void task_b(void) {
    ++b_entries;
    Os_TargetTrace('B');
    if (strcmp(scenario, "wait") == 0) {
        Os_ActivationInfo value = info(0u);
        check((value.state == WAITING) && (value.waiting == 1u) && (value.internal_held == 0u) &&
              (value.effective_priority == 2u));
        check(SetEvent(0u, 1u) == E_OK);
    } else if (strcmp(scenario, "access") == 0) {
        refused(1u, E_OS_ACCESS);
        refused(0u, E_OS_NOFUNC);
    } else {
        /* Other peer entries have no event publication or refusal fixture. */
    }
    finish();
}
static void task_h(void) {
    ++h_entries;
    Os_TargetTrace('H');
    if (strcmp(scenario, "oracle-resource") == 0) {
        Os_ActivationInfo value = info(0u);
        check((value.state == READY) && (value.effective_priority == 1u) &&
              (value.resource_count == 0u));
    }
    finish();
}
static void task_d(void) {
    Os_ActivationInfo value = info(0u);
    Os_TargetTrace('D');
    if (strcmp(scenario, "nonowner") == 0) {
        check((value.resource_count == 1u) && (value.effective_priority == 4u));
        check(ReleaseResource(1u) == E_OS_NOFUNC);
        check(GetResource(1u) == E_OS_ACCESS);
    } else {
        check((value.internal_held == 1u) && (value.effective_priority == 3u));
        value = info(3u);
        check((value.internal_held == 1u) && (value.internal_ceiling == 5u) &&
              (value.effective_priority == 5u));
    }
    finish();
}
static void task_e(void) { ShutdownOS(E_OS_STATE); }
static void monitor(void) {
    Os_ActivationInfo value = info(0u);
    check((value.count == 0u) && (value.resource_count == 0u) && (value.internal_held == 0u));
    Os_TargetTrace('M');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {{0u, "A", task_a, 2u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {1u, "B", task_b, 2u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {2u, "H", task_h, 3u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {3u, "D", task_d, 5u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {4u, "E", task_e, 2u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {5u, "M", monitor, 1u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_ResourceConfig resources[] = {{0u, 3u, 5u, 0u},
                                     {1u, 4u, 5u, 0u},
                                     {2u, 4u, 5u, 0u},
                                     {3u, 2u, 2u, 0u},
                                     {4u, 31u, 1u, 0x80000000u},
                                     {5u, 31u, 0u, 0x80000000u},
                                     {RES_SCHEDULER, 5u, 63u, 0u}};
    Os_InternalResourceConfig internal[] = {{1u, 3u}, {2u, 5u}};
    Os_TargetConfig config = {tasks, 6u, 262144u, resources, 7u,   internal,
                              2u,    0u, 0u,      0u,        NULL, NULL};
    StatusType prepared;
    scenario = (argc == 2) ? argv[1] : "non";
    if ((strcmp(scenario, "non") == 0) || (strcmp(scenario, "oracle-non") == 0)) {
        tasks[0].schedule = OS_SCHEDULE_NON;
    }
    if ((strcmp(scenario, "oracle-non") == 0) || (strcmp(scenario, "oracle-resource") == 0)) {
        tasks[0].priority = 1u;
        tasks[2].priority = 2u;
    }
    if (strcmp(scenario, "internal-preempt") == 0) {
        tasks[3].priority = 4u;
        resources[1].task_access = 13u;
        resources[2].task_access = 13u;
    }
    if ((strncmp(scenario, "internal", 8u) == 0) || (strcmp(scenario, "wait") == 0) ||
        (strcmp(scenario, "already") == 0)) {
        tasks[0].internal_resource = 1u;
        tasks[1].internal_resource = 1u;
        tasks[2].internal_resource = 1u;
        tasks[3].internal_resource = 2u;
    }
    if ((strcmp(scenario, "wait") == 0) || (strcmp(scenario, "already") == 0) ||
        (strcmp(scenario, "resource-wait") == 0) || (strcmp(scenario, "internal-resource") == 0)) {
        tasks[0].kind = OS_EXTENDED_TASK;
    }
    if (strcmp(scenario, "bad-schedule") == 0) {
        tasks[0].schedule = 2u;
    } else if (strcmp(scenario, "internal-null") == 0) {
        config.internal_resources = NULL;
    } else if (strcmp(scenario, "internal-capacity") == 0) {
        config.internal_resource_count = 3u;
    } else if (strcmp(scenario, "internal-duplicate") == 0) {
        internal[1].id = 1u;
    } else if (strcmp(scenario, "internal-missing") == 0) {
        config.internal_resource_count = 1u;
        tasks[0].internal_resource = 2u;
    } else if (strcmp(scenario, "internal-low") == 0) {
        internal[0].ceiling = 1u;
    } else if (strcmp(scenario, "internal-high") == 0) {
        internal[0].ceiling = 5u;
    } else if (strcmp(scenario, "non-internal-low") == 0) {
        tasks[0].schedule = OS_SCHEDULE_NON;
        tasks[0].internal_resource = 1u;
    } else {
        /* Behavior vectors retain the declared valid static configuration. */
    }
    resources[6].ceiling = 0u;
    for (unsigned i = 0u; i < 6u; ++i) {
        if (tasks[i].priority > resources[6].ceiling) {
            resources[6].ceiling = tasks[i].priority;
        }
    }
    if (strcmp(scenario, "scheduler-unconfigured") == 0) {
        config.resource_count = 6u;
    }
    prepared = Os_TargetPrepare(&config);
    if ((strcmp(scenario, "bad-schedule") == 0) || (strcmp(scenario, "non-internal-low") == 0) ||
        ((strncmp(scenario, "internal-", 9u) == 0) && (strcmp(scenario, "internal-preempt") != 0) &&
         (strcmp(scenario, "internal-resource") != 0))) {
        const StatusType expected = ((strcmp(scenario, "internal-duplicate") == 0) ||
                                     (strcmp(scenario, "internal-missing") == 0))
                                        ? E_OS_ID
                                        : E_OS_VALUE;
        if (printf("prepare=%u\n", prepared) < 0) {
            return 99;
        }
        return (prepared == expected) ? 0 : 99;
    }
    check(prepared == E_OK);
    StartOS(1u);
    return 99;
}
