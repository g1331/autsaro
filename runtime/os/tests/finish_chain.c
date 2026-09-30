/* Standard task bodies; private observer never chooses a runnable task. */
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static TaskHandle_t observed[7];
static unsigned entries[7];
static unsigned observations;
static unsigned rejected;
static bool chain_committed;
static bool same_info(const Os_ActivationInfo *a, const Os_ActivationInfo *b) {
    unsigned i;
    bool same = (a->state == b->state) && (a->count == b->count) &&
                (a->kernel_sequence == b->kernel_sequence) && (a->events == b->events) &&
                (a->effective_priority == b->effective_priority) &&
                (a->resource_count == b->resource_count);
    for (i = 0u; i < OS_MAX_ACTIVATIONS; ++i) {
        same = same && (a->requests[i] == b->requests[i]);
    }
    for (i = 0u; i < OS_MAX_RESOURCES; ++i) {
        same = same && (a->resources[i] == b->resources[i]);
    }
    return same;
}
static void check(bool condition) {
    if (!condition) {
        Os_TargetTrace('!');
        ShutdownOS(E_OS_STATE);
    }
}
void Os_TestObserve(void) {
    unsigned i;
    ++observations;
    for (i = 0u; i < 7u; ++i) {
        Os_ActivationInfo info;
        eTaskState actual = eTaskGetState(observed[i]);
        check(Os_TargetInspectActivation((TaskType)i, &info) == E_OK);
        check(info.state ==
              ((actual == eRunning) ? RUNNING : ((actual == eReady) ? READY : SUSPENDED)));
        check((actual == eRunning) || (actual == eReady) || (actual == eSuspended));
        if (info.count == 0u) {
            check((actual == eSuspended) && (info.kernel_sequence == 0u));
        } else {
            check((actual == eRunning) || (actual == eReady));
            check((info.requests[0] != 0u) &&
                  (info.requests[0] == ullTaskOsReadySequence(observed[i])));
        }
        check(info.effective_priority == uxTaskPriorityGet(observed[i]));
    }
}
static void reject_finish(TaskType target, bool terminate, StatusType expected) {
    Os_ActivationInfo before[7];
    Os_ActivationInfo after;
    unsigned i;
    for (i = 0u; i < 7u; ++i) {
        check(Os_TargetInspectActivation((TaskType)i, &before[i]) == E_OK);
    }
    check((terminate ? TerminateTask() : ChainTask(target)) == expected);
    for (i = 0u; i < 7u; ++i) {
        check(Os_TargetInspectActivation((TaskType)i, &after) == E_OK);
        check(same_info(&before[i], &after));
    }
    ++rejected;
}
static void finish(void) {
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
static void chain(TaskType id) {
    (void)ChainTask(id);
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
static uint32_t interrupt(void) {
    Os_TargetTrace('J');
    reject_finish(2u, false, E_OS_CALLEVEL);
    reject_finish(0u, true, E_OS_CALLEVEL);
    check(GetResource(0u) == E_OS_ACCESS);
    if ((strcmp(scenario, "isr") == 0) || (strncmp(scenario, "boundary-", 9u) == 0)) {
        check(ActivateTask(1u) == E_OK);
    }
    return 0u;
}
void Os_TestFinishBoundary(unsigned point) {
    static bool injected;
    if (point == 0u) {
        Os_TestObserve();
    }
    if (!injected && (xTaskGetCurrentTaskHandle() == observed[1]) &&
        (((strcmp(scenario, "boundary-pre") == 0) && (point == 0u)) ||
         ((strcmp(scenario, "boundary-post") == 0) && (point == 1u)))) {
        injected = true;
        vPortGenerateSimulatedInterrupt(31u);
    }
}
void StartupHook(void) {
    unsigned i;
    static const char *const names[] = {"L", "A", "B", "H", "D", "E", "M"};
    Os_TargetTrace('S');
    check(ChainTask(1u) == E_OS_CALLEVEL);
    check(TerminateTask() == E_OS_CALLEVEL);
    for (i = 0u; i < 7u; ++i) {
        observed[i] = xTaskGetHandle(names[i]);
        check(observed[i] != NULL);
    }
    vPortSetInterruptHandler(31u, interrupt);
}
void ShutdownHook(StatusType Error) {
    Os_TargetTrace('Z');
    if (printf("finish a=%u b=%u h=%u d=%u e=%u observer=%u rejected=%u error=%u\n", entries[1],
               entries[2], entries[3], entries[4], entries[5], observations, rejected, Error) < 0) {
        ExitProcess(E_OS_STATE);
    }
}
static void launcher(void) {
    ++entries[0];
    Os_TargetTrace('L');
    if (strcmp(scenario, "extended-self") == 0) {
        check(ActivateTask(5u) == E_OK);
        check(SetEvent(5u, 7u) == E_OK);
    } else if (strcmp(scenario, "extended-target") == 0) {
        check(ActivateTask(5u) == E_OK);
        check(SetEvent(5u, 7u) == E_OK);
        check(ActivateTask(1u) == E_OK);
    } else {
        check(ActivateTask(1u) == E_OK);
        if ((strcmp(scenario, "terminate-pending") == 0) ||
            (strcmp(scenario, "self-pending") == 0) || (strcmp(scenario, "low") == 0)) {
            check(ActivateTask(1u) == E_OK);
        }
        if ((strcmp(scenario, "self") == 0) || (strcmp(scenario, "self-pending") == 0) ||
            (strcmp(scenario, "invalid") == 0) || (strcmp(scenario, "same-fifo") == 0)) {
            if (strcmp(scenario, "same-fifo") == 0) {
                check(ActivateTask(3u) == E_OK);
            }
            check(ActivateTask(2u) == E_OK);
        }
        if (strcmp(scenario, "limit") == 0) {
            check(ActivateTask(5u) == E_OK);
            check(SetEvent(5u, 7u) == E_OK);
        }
    }
    finish();
}
static void task_a(void) {
    volatile unsigned local = 0x12345678u;
    Os_ActivationInfo info;
    char marker;
    ++entries[1];
    check(local == 0x12345678u);
    if (entries[1] == 1u) {
        marker = 'A';
    } else if (entries[1] == 2u) {
        marker = 'C';
    } else {
        marker = 'F';
    }
    Os_TargetTrace(marker);
    local = 0x87654321u;
    if ((entries[1] == 1u) &&
        ((strcmp(scenario, "self") == 0) || (strcmp(scenario, "self-pending") == 0))) {
        chain(1u);
    }
    if (strcmp(scenario, "high") == 0) {
        chain_committed = true;
        chain(3u);
    }
    if ((strcmp(scenario, "same") == 0) || (strcmp(scenario, "same-fifo") == 0)) {
        chain_committed = true;
        chain(2u);
    }
    if ((strncmp(scenario, "boundary-", 9u) == 0) && (entries[1] == 1u)) {
        chain_committed = true;
        chain(2u);
    }
    if ((strcmp(scenario, "low") == 0) && (entries[1] == 1u)) {
        chain_committed = true;
        chain(4u);
    }
    if (strcmp(scenario, "extended-target") == 0) {
        EventMaskType events = 99u;
        check((GetEvent(5u, &events) == E_OS_STATE) && (events == 99u));
        chain(5u);
    }
    if (strcmp(scenario, "invalid") == 0) {
        reject_finish(99u, false, E_OS_ID);
    }
    if (strcmp(scenario, "limit") == 0) {
        reject_finish(5u, false, E_OS_LIMIT);
    }
    if (strcmp(scenario, "resource") == 0) {
        check(GetResource(0u) == E_OK);
        check(Os_TargetInspectActivation(1u, &info) == E_OK);
        check((info.effective_priority == 5u) && (info.resource_count == 1u) &&
              (info.resources[0] == 0u));
        reject_finish(2u, false, E_OS_RESOURCE);
        reject_finish(0u, true, E_OS_RESOURCE);
        check(ReleaseResource(0u) == E_OK);
        check(Os_TargetInspectActivation(1u, &info) == E_OK);
        check((info.effective_priority == 3u) && (info.resource_count == 0u));
    }
    if ((strcmp(scenario, "isr") == 0) && (entries[1] == 1u)) {
        vPortGenerateSimulatedInterrupt(31u);
        check(local == 0x87654321u);
        Os_TargetTrace('a');
    }
    if (strcmp(scenario, "missing-end") == 0) {
        return;
    }
    finish();
}
static void check_source_ended(void) {
    Os_ActivationInfo info;
    check(Os_TargetInspectActivation(1u, &info) == E_OK);
    if (strncmp(scenario, "boundary-", 9u) == 0) {
        /* ISR accepted A#2 after Chain; the completed A#1 order3 is gone. */
        check((info.count == 1u) && (info.state == READY) && (info.requests[0] == 5u));
    } else {
        check((info.count == 0u) && (info.state == SUSPENDED));
    }
}
static void task_b(void) {
    ++entries[2];
    if (chain_committed) {
        check_source_ended();
    }
    Os_TargetTrace((entries[2] == 1u) ? 'B' : 'b');
    finish();
}
static void task_h(void) {
    ++entries[3];
    if (chain_committed) {
        check_source_ended();
    }
    Os_TargetTrace('H');
    finish();
}
static void task_d(void) {
    ++entries[4];
    check_source_ended();
    Os_TargetTrace('D');
    finish();
}
static void task_e(void) {
    volatile unsigned local = 7u;
    EventMaskType events;
    ++entries[5];
    check(local == 7u);
    check(GetEvent(5u, &events) == E_OK);
    check(events == ((entries[5] == 1u) ? 7u : 0u));
    local = 99u;
    Os_TargetTrace((entries[5] == 1u) ? 'E' : 'e');
    if ((strcmp(scenario, "extended-self") == 0) && (entries[5] == 1u)) {
        chain(5u);
    }
    finish();
}
static void monitor(void) {
    unsigned i;
    for (i = 0u; i < 6u; ++i) {
        Os_ActivationInfo info;
        check(Os_TargetInspectActivation((TaskType)i, &info) == E_OK);
        check((info.count == 0u) && (info.state == SUSPENDED) && (info.resource_count == 0u));
    }
    Os_TargetTrace('M');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {{0u, "L", launcher, 6u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {1u, "A", task_a, 3u, 0u, OS_BASIC_TASK, 2u, OS_SCHEDULE_FULL, 0u},
                             {2u, "B", task_b, 3u, 0u, OS_BASIC_TASK, 2u, OS_SCHEDULE_FULL, 0u},
                             {3u, "H", task_h, 4u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {4u, "D", task_d, 2u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {5u, "E", task_e, 3u, 0u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {6u, "M", monitor, 1u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_ResourceConfig resources[] = {{0u, 5u, 10u, 0u}, {1u, 5u, 10u, 0u}};
    Os_TargetConfig config = {tasks, 7u, 262144u, resources, 1u,   NULL, 0u,
                              0u,    0u, 0u,      NULL,      NULL, NULL};
    StatusType prepared;
    scenario = (argc == 2) ? argv[1] : "terminate-pending";
    if (strcmp(scenario, "self") == 0) {
        tasks[1].activation_limit = 1u;
    }
    if (strcmp(scenario, "same-fifo") == 0) {
        tasks[3].priority = 3u;
    }
    if (strcmp(scenario, "resource-null") == 0) {
        config.resources = NULL;
    } else if (strcmp(scenario, "resource-capacity") == 0) {
        config.resource_count = 9u;
    } else if (strcmp(scenario, "resource-access") == 0) {
        resources[0].task_access = 0x8000u;
    } else if (strcmp(scenario, "resource-duplicate") == 0) {
        config.resource_count = 2u;
        resources[1].id = 0u;
    } else if (strcmp(scenario, "resource-ceiling-low") == 0) {
        resources[0].ceiling = 3u;
    } else if (strcmp(scenario, "resource-ceiling-high") == 0) {
        resources[0].ceiling = 6u;
    } else {
        /* Behavioral scenarios retain the declared valid resource configuration. */
    }
    prepared = Os_TargetPrepare(&config);
    if (strncmp(scenario, "resource-", 9u) == 0) {
        const StatusType expected =
            (strcmp(scenario, "resource-duplicate") == 0) ? E_OS_ID : E_OS_VALUE;
        if (printf("prepare=%u\n", prepared) < 0) {
            return 99;
        }
        return (prepared == expected) ? 0 : 99;
    }
    check(prepared == E_OK);
    check(ChainTask(1u) == E_OS_CALLEVEL);
    StartOS(1u);
    return 99;
}
