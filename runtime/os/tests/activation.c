/* Standard Task bodies plus a private, independent kernel observer/injector. */
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static const Os_TaskConfig *configuration;
static unsigned a_entries;
static unsigned b_entries;
static unsigned e_entries;
static unsigned h_entries;
static unsigned expected_a;
static unsigned expected_b;
static unsigned expected_e;
static unsigned observations;
static TaskHandle_t observer_handles[6];
static unsigned rejected;
static bool preempt;
void Os_TestBeforeActivationLock(void) {
    static bool closing_injected;
    if ((strcmp(scenario, "close-admission") == 0) && !closing_injected) {
        closing_injected = true;
        Os_TargetTrace('Q');
        Os_BackendRequestShutdown(E_OK);
    }
}
void Os_TestAtActivationAdmission(unsigned point) {
    if (((strcmp(scenario, "close-ready") == 0) && (point == 0u)) ||
        ((strcmp(scenario, "close-accepted") == 0) && (point == 1u))) {
        Os_TargetTrace('Q');
        Os_BackendRequestShutdown(E_OK);
    }
}
void Os_TestBeforeClose(void) {
    if ((strcmp(scenario, "close-admission") == 0) || (strcmp(scenario, "close-ready") == 0) ||
        (strcmp(scenario, "close-accepted") == 0)) {
        /* A bounded, normal-close race injection; never enabled in production
         * or the native stack-fault harness, and never selects a runnable task. */
        Sleep(100u);
    }
}

static void check(bool condition) {
    if (!condition) {
        Os_TargetTrace('!');
        ShutdownOS(E_OS_STATE);
    }
}
static bool same_info(const Os_ActivationInfo *left, const Os_ActivationInfo *right) {
    unsigned i;
    bool same = (left->state == right->state) && (left->count == right->count) &&
                (left->kernel_sequence == right->kernel_sequence) &&
                (left->events == right->events) &&
                (left->effective_priority == right->effective_priority) &&
                (left->resource_count == right->resource_count);
    for (i = 0u; i < OS_MAX_RESOURCES; ++i) {
        if (left->resources[i] != right->resources[i]) {
            same = false;
        }
    }
    for (i = 0u; i < 32u; ++i) {
        if (left->requests[i] != right->requests[i]) {
            same = false;
        }
    }
    return same;
}

/* This observer reads actual TCB state/keys independently of automotive APIs. */
void Os_TestObserve(void) {
    unsigned i;
    ++observations;
    for (i = 0u; i < 6u; ++i) {
        Os_ActivationInfo info;
        TaskStateType expected;
        TaskHandle_t handle = observer_handles[i];
        eTaskState actual = eTaskGetState(handle);
        switch (actual) {
        case eRunning:
            expected = RUNNING;
            break;
        case eReady:
            expected = READY;
            break;
        case eSuspended:
            expected = SUSPENDED;
            break;
        default:
            ShutdownOS(E_OS_STATE);
            return;
        }
        check(Os_TargetInspectActivation(configuration[i].id, &info) == E_OK);
        check((info.state == expected) && (info.count <= configuration[i].activation_limit));
        if (info.count == 0u) {
            check((actual == eSuspended) && (info.kernel_sequence == 0u));
        } else {
            unsigned j;
            check((actual == eReady) || (actual == eRunning));
            check((info.requests[0] != 0u) &&
                  (info.requests[0] == ullTaskOsReadySequence(handle)) &&
                  (info.kernel_sequence == info.requests[0]));
            for (j = 1u; j < info.count; ++j) {
                check(info.requests[j] > info.requests[j - 1u]);
            }
        }
    }
}

static void expect_rejection(TaskType id, StatusType expected) {
    Os_ActivationInfo before[6];
    Os_ActivationInfo after;
    unsigned i;
    for (i = 0u; i < 6u; ++i) {
        check(Os_TargetInspectActivation(configuration[i].id, &before[i]) == E_OK);
    }
    check(ActivateTask(id) == expected);
    for (i = 0u; i < 6u; ++i) {
        check(Os_TargetInspectActivation(configuration[i].id, &after) == E_OK);
        check(same_info(&before[i], &after));
    }
    ++rejected;
}

static void finish(void) {
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}

static uint32_t test_isr(void) {
    Os_ActivationInfo before;
    Os_ActivationInfo after;
    Os_TargetTrace('J');
    check(ActivateTask(1u) == E_OK);
    expect_rejection(99u, E_OS_ID);
    expect_rejection(1u, E_OS_LIMIT);
    check(Os_TargetInspectActivation(1u, &before) == E_OK);
    check(TerminateTask() == E_OS_CALLEVEL);
    check(Os_TargetInspectActivation(1u, &after) == E_OK);
    check(same_info(&before, &after));
    ++rejected;
    check(ActivateTask(2u) == E_OK);
    check(Os_TargetInspectActivation(2u, &after) == E_OK);
    check((after.count == 1u) && (after.requests[0] == 4u));
    /* The backend must request rescheduling even though this shim returns false. */
    return 0u;
}

void StartupHook(void) {
    unsigned i;
    TaskStateType state = 99u;
    for (i = 0u; i < 6u; ++i) {
        observer_handles[i] = xTaskGetHandle(configuration[i].name);
        check(observer_handles[i] != NULL);
    }
    Os_TargetTrace('S');
    check(ActivateTask(1u) == E_OS_CALLEVEL);
    check(TerminateTask() == E_OS_CALLEVEL);
    check(GetTaskState(1u, &state) == E_OS_CALLEVEL);
    check(state == 99u);
    if (strcmp(scenario, "isr") == 0) {
        vPortSetInterruptHandler(31u, test_isr);
    }
}

void ShutdownHook(StatusType Error) {
    TaskStateType state = 99u;
    if ((ActivateTask(1u) != E_OS_CALLEVEL) || (TerminateTask() != E_OS_CALLEVEL) ||
        (GetTaskState(1u, &state) != E_OS_CALLEVEL) || (state != 99u)) {
        Os_TargetTrace('X');
    }
    Os_TargetTrace('Z');
    if ((strcmp(scenario, "close-admission") == 0) || (strcmp(scenario, "close-ready") == 0) ||
        (strcmp(scenario, "close-accepted") == 0)) {
        if (printf("close_pending=%u\n", Os_TargetTestPending(1u)) < 0) {
            ExitProcess(E_OS_STATE);
        }
    }
    if (printf("activation a=%u b=%u e=%u h=%u observer=%u rejected=%u error=%u\n", a_entries,
               b_entries, e_entries, h_entries, observations, rejected, Error) < 0) {
        ExitProcess(E_OS_STATE);
    }
}

static void inspect_requests(TaskType id, unsigned count, uint64_t first, uint64_t second) {
    Os_ActivationInfo info;
    check(Os_TargetInspectActivation(id, &info) == E_OK);
    check(info.count == count);
    if (count > 0u) {
        check(info.requests[0] == first);
    }
    if (count > 1u) {
        check(info.requests[1] == second);
    }
}

static void launcher(void) {
    TaskStateType state = 99u;
    Os_TargetTrace('L');
    check(GetTaskState(1u, NULL) == E_OS_ILLEGAL_ADDRESS);
    check(GetTaskState(99u, &state) == E_OS_ID);
    check(state == 99u);
    if ((strcmp(scenario, "close-admission") == 0) || (strcmp(scenario, "close-ready") == 0) ||
        (strcmp(scenario, "close-accepted") == 0)) {
        (void)ActivateTask(1u);
        Os_TargetTrace('X');
        ShutdownOS(E_OS_STATE);
    } else if (strcmp(scenario, "isr") == 0) {
        vPortGenerateSimulatedInterrupt(31u);
        check(a_entries == 1u);
        Os_TargetTrace('l');
    } else if (strcmp(scenario, "extended") == 0) {
        check(ActivateTask(4u) == E_OK);
        expect_rejection(4u, E_OS_LIMIT);
    } else if (strcmp(scenario, "overflow") == 0) {
        Os_TargetTestSequence(UINT64_MAX - 1u);
        check(ActivateTask(1u) == E_OK);
        inspect_requests(1u, 1u, UINT64_MAX, 0u);
        check(ActivateTask(2u) == E_OK);
        inspect_requests(1u, 1u, 3u, 0u);
        inspect_requests(2u, 1u, 4u, 0u);
    } else if (strcmp(scenario, "rebase-fifo") == 0) {
        Os_TargetTestSequence(UINT64_MAX - 3u);
        check(ActivateTask(1u) == E_OK);
        check(ActivateTask(2u) == E_OK);
        check(ActivateTask(1u) == E_OK);
        check(ActivateTask(2u) == E_OK);
        inspect_requests(1u, 2u, 3u, 5u);
        inspect_requests(2u, 2u, 4u, 6u);
    } else if ((strcmp(scenario, "capacity") == 0) || (strcmp(scenario, "wrap") == 0)) {
        Os_ActivationInfo info;
        unsigned i;
        for (i = 0u; i < 32u; ++i) {
            check(ActivateTask(1u) == E_OK);
        }
        check(Os_TargetInspectActivation(1u, &info) == E_OK);
        check(info.count == 32u);
        for (i = 0u; i < 32u; ++i) {
            check(info.requests[i] == (3u + i));
        }
        expect_rejection(1u, E_OS_LIMIT);
    } else {
        check(ActivateTask(1u) == E_OK);
        if ((strcmp(scenario, "aba") == 0) || (strcmp(scenario, "abab") == 0)) {
            check(ActivateTask(2u) == E_OK);
            check(ActivateTask(1u) == E_OK);
            inspect_requests(1u, 2u, 3u, 5u);
            inspect_requests(2u, 1u, 4u, 0u);
        } else {
            check(ActivateTask(1u) == E_OK);
            check(ActivateTask(2u) == E_OK);
            inspect_requests(1u, 2u, 3u, 4u);
            inspect_requests(2u, 1u, 5u, 0u);
        }
        if ((strcmp(scenario, "aabb") == 0) || (strcmp(scenario, "abab") == 0)) {
            check(ActivateTask(2u) == E_OK);
        }
        if (strcmp(scenario, "limits") == 0) {
            expect_rejection(1u, E_OS_LIMIT);
        }
        if (strcmp(scenario, "invalid") == 0) {
            expect_rejection(99u, E_OS_ID);
        }
    }
    finish();
}

static void task_a(void) {
    volatile unsigned local = 0x12345678u;
    const volatile unsigned *const address = &local;
    ++a_entries;
    Os_TargetTrace((a_entries == 1u) ? 'A' : 'C');
    check(local == 0x12345678u);
    if (strcmp(scenario, "autostart") == 0) {
        inspect_requests(1u, 1u, 1u, 0u);
        inspect_requests(2u, 1u, 2u, 0u);
    }
    if ((a_entries == 1u) && (strcmp(scenario, "limits") == 0)) {
        expect_rejection(1u, E_OS_LIMIT);
    }
    if ((a_entries == 1u) && preempt) {
        TaskStateType state;
        check(ActivateTask(3u) == E_OK);
        check((local == 0x12345678u) && (address == &local) && (h_entries == 1u));
        check((GetTaskState(1u, &state) == E_OK) && (state == RUNNING));
        Os_TargetTrace('a');
    }
    if ((strcmp(scenario, "wrap") == 0) && (a_entries == 17u)) {
        Os_ActivationInfo info;
        unsigned i;
        for (i = 0u; i < 16u; ++i) {
            check(ActivateTask(1u) == E_OK);
        }
        check(Os_TargetInspectActivation(1u, &info) == E_OK);
        check(info.count == 32u);
        for (i = 0u; i < 32u; ++i) {
            check(info.requests[i] == (19u + i));
        }
    }
    finish();
}
static void task_b(void) {
    ++b_entries;
    Os_TargetTrace((b_entries == 1u) ? 'B' : 'D');
    finish();
}
static void task_h(void) {
    Os_ActivationInfo info;
    ++h_entries;
    check(Os_TargetInspectActivation(3u, &info) == E_OK);
    check((info.count == 1u) && (info.requests[0] == (expected_a + expected_b + 3u)));
    Os_TargetTrace('H');
    finish();
}
static void task_e(void) {
    ++e_entries;
    expect_rejection(4u, E_OS_LIMIT);
    Os_TargetTrace('E');
    finish();
}
static void monitor(void) {
    unsigned i;
    check((a_entries == expected_a) && (b_entries == expected_b) && (e_entries == expected_e));
    check(h_entries == (preempt ? 1u : 0u));
    check(observations != 0u);
    for (i = 0u; i < 5u; ++i) {
        Os_ActivationInfo info;
        check(Os_TargetInspectActivation((TaskType)i, &info) == E_OK);
        check((info.count == 0u) && (info.state == SUSPENDED));
    }
    Os_TargetTrace('M');
    ShutdownOS(E_OK);
}

int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {{0u, "L", launcher, 4u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {1u, "A", task_a, 2u, 0u, OS_BASIC_TASK, 2u, OS_SCHEDULE_FULL, 0u},
                             {2u, "B", task_b, 2u, 0u, OS_BASIC_TASK, 2u, OS_SCHEDULE_FULL, 0u},
                             {3u, "H", task_h, 3u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {4u, "E", task_e, 2u, 0u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
                             {5u, "M", monitor, 1u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    const Os_TargetConfig config = {tasks, 6u, 262144u, NULL, 0u,   NULL, 0u,
                                    0u,    0u, 0u,      NULL, NULL, NULL};
    StatusType prepared;
    scenario = (argc == 2) ? argv[1] : "aab";
    configuration = tasks;
    preempt = true;
    expected_a = 2u;
    expected_b = 1u;
    if ((strcmp(scenario, "aabb") == 0) || (strcmp(scenario, "abab") == 0)) {
        expected_b = 2u;
    }
    if (strcmp(scenario, "extended") == 0) {
        expected_a = 0u;
        expected_b = 0u;
        expected_e = 1u;
        preempt = false;
    }
    if ((strcmp(scenario, "overflow") == 0) || (strcmp(scenario, "isr") == 0)) {
        expected_a = 1u;
        expected_b = 1u;
        preempt = false;
    }
    if (strcmp(scenario, "rebase-fifo") == 0) {
        expected_b = 2u;
        preempt = false;
    }
    if (strcmp(scenario, "autostart") == 0) {
        tasks[0].autostart_modes = 0u;
        tasks[1].autostart_modes = 1u;
        tasks[2].autostart_modes = 1u;
        expected_a = 1u;
        expected_b = 1u;
        preempt = false;
    }
    if (strcmp(scenario, "isr") == 0) {
        tasks[0].priority = 2u;
        tasks[1].priority = 3u;
        tasks[1].activation_limit = 1u;
    }
    if ((strcmp(scenario, "capacity") == 0) || (strcmp(scenario, "wrap") == 0)) {
        tasks[1].activation_limit = 32u;
        expected_a = (strcmp(scenario, "wrap") == 0) ? 48u : 32u;
        expected_b = 0u;
        preempt = false;
    }
    if (strcmp(scenario, "zero-limit") == 0) {
        tasks[1].activation_limit = 0u;
    }
    if (strcmp(scenario, "bad-limit") == 0) {
        tasks[1].activation_limit = 33u;
    }
    if (strcmp(scenario, "extended-multiple") == 0) {
        tasks[4].activation_limit = 2u;
    }
    if (strcmp(scenario, "bad-kind") == 0) {
        tasks[1].kind = 2u;
    }
    prepared = Os_TargetPrepare(&config);
    if (prepared != E_OK) {
        if (printf("prepare=%u\n", prepared) < 0) {
            return 99;
        }
        return (prepared == E_OS_VALUE) ? 0 : 99;
    }
    check(ActivateTask(1u) == E_OS_CALLEVEL);
    check(TerminateTask() == E_OS_CALLEVEL);
    StartOS(1u);
    return 99;
}
