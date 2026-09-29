#include "Os_Target.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    TaskType task;
    unsigned entry;
    TaskStateType state;
    uint8_t priority;
    uint8_t internal;
    EventMaskType events;
    EventMaskType final_events;
    TickType counter;
} Observation;
static Observation observations[32];
static unsigned observation_count;
static unsigned entries[16];
static unsigned initial_active;
static unsigned acquired_resources;
static unsigned queued;
static unsigned modes;
static size_t task_count;
static size_t resource_count;
static uint8_t highest_priority;
static bool extended;
static bool multiplicity;
static bool nonpreemptive;
static AppModeType selected_mode = 1u;
static Os_TaskConfig tasks[16];
static Os_ResourceConfig resources[8];
static const char *scenario;

static void check(bool condition) {
    if (!condition) {
        ShutdownOS(E_OS_STATE);
    }
}
void StartupHook(void) {
    check(GetActiveApplicationMode() == selected_mode);
    ++modes;
}
void ShutdownHook(StatusType Error) {
    printf("capacity case=%s tasks=%zu active=%u priorities=%u resources=%u internal=2 alarms=1 "
           "modes=%u queued=%u reason=%u\n",
           scenario, task_count, initial_active, highest_priority, acquired_resources, modes,
           queued, Error);
    for (unsigned i = 0u; i < observation_count; ++i) {
        const Observation *o = &observations[i];
        printf("task id=%u entry=%u state=%u priority=%u internal=%u events_seen=%u "
               "events_final=%u counter=%llu\n",
               o->task, o->entry, o->state, o->priority, o->internal, o->events, o->final_events,
               (unsigned long long)o->counter);
    }
}
static void entry(void) {
    TaskType id = INVALID_TASK;
    Os_ActivationInfo info;
    EventMaskType events = 0u;
    EventMaskType observed_events = 0u;
    TickType counter = UINT64_MAX;
    check(GetTaskID(&id) == E_OK && id < task_count);
    check(GetActiveApplicationMode() == selected_mode);
    check(GetTaskID(NULL) == E_OS_ILLEGAL_ADDRESS);
    if (observation_count == 0u) {
        for (size_t i = 0u; i < task_count; ++i) {
            TaskStateType state = SUSPENDED;
            check(GetTaskState((TaskType)i, &state) == E_OK);
            check(state == RUNNING || state == READY);
            ++initial_active;
        }
    }
    ++entries[id];
    if (tasks[id].kind == OS_EXTENDED_TASK) {
        check(SetEvent(id, UINT32_C(255)) == E_OK);
        check(WaitEvent(UINT32_C(128)) == E_OK);
        check(GetEvent(id, &events) == E_OK && events == UINT32_C(255));
        observed_events = events;
        for (unsigned bit = 0u; bit < 8u; ++bit) {
            check(ClearEvent(UINT32_C(1) << bit) == E_OK);
            check(GetEvent(id, &events) == E_OK);
            check(events == (UINT32_C(255) & ~((UINT32_C(1) << (bit + 1u)) - 1u)));
        }
    }
    check(GetCounterValue(0u, &counter) == E_OK);
    check(Os_TargetInspectActivation(id, &info) == E_OK && info.state == RUNNING &&
          info.events == 0u);
    check(info.effective_priority ==
          ((tasks[id].schedule == OS_SCHEDULE_NON) ? highest_priority : tasks[id].priority));
    check(info.internal_held ==
          ((tasks[id].internal_resource != 0u || tasks[id].schedule == OS_SCHEDULE_NON) ? 1u : 0u));
    check(observation_count < 32u);
    observations[observation_count++] = (Observation){id,
                                                      entries[id],
                                                      info.state,
                                                      info.effective_priority,
                                                      info.internal_held,
                                                      observed_events,
                                                      info.events,
                                                      counter};
    if (id != 0u) {
        (void)TerminateTask();
        ShutdownOS(E_OS_STATE);
    }
    for (size_t i = 0u; i < resource_count; ++i) {
        check(GetResource(resources[i].id) == E_OK);
        ++acquired_resources;
    }
    check(Os_TargetInspectActivation(0u, &info) == E_OK && info.resource_count == resource_count &&
          info.effective_priority == highest_priority);
    for (size_t i = resource_count; i > 0u; --i) {
        check(ReleaseResource(resources[i - 1u].id) == E_OK);
    }
    {
        AlarmBaseType base;
        TickType remaining = UINT64_MAX;
        uint64_t ticket = UINT64_MAX;
        uint64_t epoch = UINT64_MAX;
        check(GetAlarmBase(0u, &base) == E_OK && base.maxallowedvalue == 31u &&
              base.ticksperbase == 1u && base.mincycle == 1u);
        check(SetRelAlarm(0u, 1u, 0u) == E_OK);
        check(GetAlarm(0u, &remaining) == E_OK && remaining == 1u);
        check(IncrementCounter(0u) == E_OK);
        check(entries[1] == 2u);
        check(GetCounterValue(0u, &counter) == E_OK && counter == 1u);
        check(Os_TargetCurrentTick(&ticket, &epoch) == E_OS_STATE && ticket == UINT64_MAX &&
              epoch == UINT64_MAX);
    }
    if (multiplicity) {
        check(GetResource(RES_SCHEDULER) == E_OK);
        for (unsigned i = 0u; i < 3u; ++i) {
            check(ActivateTask(1u) == E_OK);
        }
        check(ActivateTask(1u) == E_OS_LIMIT);
        check(Os_TargetInspectActivation(1u, &info) == E_OK && info.count == 3u &&
              info.state == READY);
        queued = info.count;
        check(ActivateTask((TaskType)(task_count - 1u)) == E_OK);
        check(ReleaseResource(RES_SCHEDULER) == E_OK);
        check(entries[1] == 5u && entries[task_count - 1u] == 2u);
    }
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    Os_InternalResourceConfig internal[2] = {{1u, 1u}, {2u, 2u}};
    Os_CounterConfig counters[2] = {{0u, 31u, 1u, 1u, 1u}, {1u, 31u, 1u, 1u, 0u}};
    Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_ACTIVATE, 1u, 0u, NULL, 0u, 0u, 0u, 0u, 0u};
    Os_TimeConfig time = {counters, 1u, &alarm, 1u, 0u, INVALID_TASK, 0u, NULL, NULL, 0u};
    Os_TargetConfig target = {tasks, 0u, 262144u, resources, 0u,    internal,
                              2u,    0u, 0u,      0u,        &time, NULL};
    StatusType expected = E_OK;
    scenario = (argc == 2) ? argv[1] : "bcc1";
    extended = strncmp(scenario, "ecc", 3u) == 0;
    multiplicity = strstr(scenario, "multiplicity") != NULL;
    nonpreemptive = strstr(scenario, "nonpreemptive") != NULL;
    selected_mode = strstr(scenario, "mode2") != NULL ? 2u : 1u;
    task_count = extended ? 16u : 8u;
    highest_priority = (uint8_t)(task_count - (multiplicity ? 1u : 0u));
    resource_count = strncmp(scenario, "bcc1", 4u) == 0 ? 1u : 8u;
    target.task_count = task_count;
    target.resource_count = resource_count;
    for (size_t i = 0u; i < task_count; ++i) {
        tasks[i] = (Os_TaskConfig){(TaskType)i,
                                   "capacity",
                                   entry,
                                   (uint8_t)(i + 1u),
                                   selected_mode,
                                   extended ? OS_EXTENDED_TASK : OS_BASIC_TASK,
                                   1u,
                                   OS_SCHEDULE_FULL,
                                   (i < 2u) ? (uint8_t)(i + 1u) : 0u};
    }
    if (multiplicity) {
        tasks[1].kind = OS_BASIC_TASK;
        tasks[1].activation_limit = 3u;
        tasks[task_count - 1u].kind = OS_BASIC_TASK;
        tasks[task_count - 1u].priority = 2u;
    }
    if (nonpreemptive) {
        tasks[2].schedule = OS_SCHEDULE_NON;
    }
    for (size_t i = 0u; i < resource_count; ++i) {
        const bool scheduler = i == resource_count - 1u;
        resources[i] = (Os_ResourceConfig){
            scheduler ? RES_SCHEDULER : (ResourceType)i, scheduler ? highest_priority : 1u,
            scheduler ? (uint16_t)((UINT32_C(1) << task_count) - 1u) : 1u, 0u};
    }
    if (strcmp(scenario, "bad-manual-owner") == 0) {
        time.owner = 0u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-manual-hardware") == 0) {
        time.counter_count = 2u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-task-capacity") == 0) {
        target.task_count = 17u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-resource-capacity") == 0) {
        target.resource_count = 9u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-internal-capacity") == 0) {
        target.internal_resource_count = 3u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-alarm-capacity") == 0) {
        time.alarm_count = 17u;
        expected = E_OS_VALUE;
    } else if (strcmp(scenario, "bad-priority") == 0) {
        tasks[0].priority = 31u;
        expected = E_OS_VALUE;
    }
    if (Os_TargetPrepare(&target) != expected) {
        return 99;
    }
    if (expected != E_OK) {
        printf("capacity rejection=%s reason=%u ready=%d\n", scenario, expected, Os_TargetReady());
        return expected;
    }
    check(GetActiveApplicationMode() == 0u);
    StartOS(selected_mode);
    return 99;
}
