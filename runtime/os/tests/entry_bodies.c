#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#define OwnerTask 0u
#define WorkerTask 1u
#define ParentISR 5u
#define ChildISR 6u
TASK(OwnerTask);
TASK(WorkerTask);
ISR(ParentISR);
ISR(ChildISR);
ALARMCALLBACK(AlarmBody);

static const char *scenario;
static unsigned parents;
static unsigned children;
static unsigned workers;
static unsigned alarms;
static unsigned errors;
static char trace[32];
static unsigned length;
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
void ErrorHook(StatusType error) {
    check(errors == 0u);
    if (is("alarm")) {
        check(error == 2u && GetISRID() == INVALID_ISR && OSErrorGetServiceId() == 130u &&
              OSError_ActivateTask_TaskID() == 1u);
    } else {
        check(GetISRID() == 5u && OSErrorGetServiceId() == 252u);
        check(error == (is("cleanup-mask") ? 9u : 6u));
    }
    check(Os_InterruptDisabled() == 0);
    ++errors;
    record('E');
}
void ShutdownHook(StatusType error) {
    printf("entry_bodies scenario=%s parents=%u children=%u workers=%u alarms=%u errors=%u "
           "trace=%s reason=%u\n",
           scenario, parents, children, workers, alarms, errors, trace, error);
}
static uint32_t replacement(void) { return 0u; }
void StartupHook(void) {
    if (is("replace-owned")) {
        vPortSetInterruptHandler(5u, replacement);
    }
}
ISR(ChildISR) {
    check(GetISRID() == 6u);
    ++children;
    record('C');
    check(ActivateTask(WorkerTask) == E_OK);
    check(workers == 0u);
    record('c');
}
ISR(ParentISR) {
    check(GetISRID() == 5u);
    ++parents;
    record('I');
    if (is("nested")) {
        vPortGenerateSimulatedInterrupt(6u);
        check(children == 1u && GetISRID() == 5u && workers == 0u);
    } else {
        check(ActivateTask(WorkerTask) == E_OK);
        if (is("cleanup-mask")) {
            DisableAllInterrupts();
        } else if (is("cleanup-resource")) {
            check(GetResource(0u) == E_OK);
        }
    }
    record('i');
}
ALARMCALLBACK(AlarmBody) {
    check(Os_HookContext() == OS_HOOK_ALARM);
    ++alarms;
    record('A');
    check(ActivateTask(WorkerTask) == E_OS_CALLEVEL);
    check(workers == 0u);
    record('a');
}
TASK(WorkerTask) {
    TaskType task = INVALID_TASK;
    check(GetTaskID(&task) == E_OK && task == 1u && GetISRID() == INVALID_ISR);
    ++workers;
    record('W');
    if (is("cleanup-resource")) {
        check(GetResource(0u) == E_OK && ReleaseResource(0u) == E_OK);
    }
    check(TerminateTask() == E_OK);
    check(false);
}
TASK(OwnerTask) {
    TaskType task = INVALID_TASK;
    check(GetTaskID(&task) == E_OK && task == 0u);
    record('O');
    if (is("alarm")) {
        check(SetRelAlarm(0u, 1u, 0u) == E_OK && IncrementCounter(0u) == E_OK);
        check(alarms == 1u && workers == 0u);
        check(ActivateTask(WorkerTask) == E_OK);
    } else {
        vPortGenerateSimulatedInterrupt(5u);
        check(parents == 1u);
    }
    check(workers == 1u && Os_InterruptDisabled() == 0);
    record('o');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv) {
    check(argc == 2);
    scenario = argv[1];
    Os_TaskConfig tasks[] = {
        {OwnerTask, "OwnerTask", OS_TASK_ENTRY(OwnerTask), 10u, 1u, OS_BASIC_TASK, 1u,
         OS_SCHEDULE_FULL, 0u},
        {WorkerTask, "WorkerTask", OS_TASK_ENTRY(WorkerTask), 15u, 0u, OS_BASIC_TASK, 1u,
         OS_SCHEDULE_FULL, 0u},
    };
    const Os_ResourceConfig resource = {0u, 2u, 3u, UINT32_C(1) << 5u};
    const Os_CounterConfig counter = {0u, 15u, 1u, 1u, 1u};
    const Os_AlarmConfig alarm = {0u, 0u, OS_ALARM_CALLBACK, 0u, 0u, AlarmBody, 0u, 0u, 0u, 0u, 0u};
    const Os_TimeConfig time = {&counter, 1u, &alarm, 1u, 0u, INVALID_TASK, 0u, NULL, NULL, 0u};
    const Os_HookConfig hooks = {ErrorHook, NULL, NULL};
    Os_IsrEntry entries[OS_MAX_INTERRUPTS] = {[5u] = OS_ISR_ENTRY(ParentISR),
                                              [6u] = OS_ISR_ENTRY(ChildISR)};
    Os_IsrConfig interrupts = {.priorities = {[5u] = 1u, [6u] = 2u}, .entries = entries};
    Os_TargetConfig target = {tasks, 2u, 262144u, &resource, 1u,     NULL,       0u,
                              0u,    0u, 0u,      &time,     &hooks, &interrupts};
    if (is("invalid-priority")) {
        interrupts.priorities[5u] = 0u;
    } else if (is("invalid-kernel")) {
        entries[0u] = OS_ISR_ENTRY(ParentISR);
    } else if (is("invalid-tick")) {
        entries[1u] = OS_ISR_ENTRY(ParentISR);
    } else if (is("invalid-cat1")) {
        target.category1_isrs = UINT32_C(1) << 5u;
    } else if (is("invalid-mailbox")) {
        tasks[0u].kind = OS_EXTENDED_TASK;
        target.input_event = 1u;
        interrupts.priorities[OS_INPUT_INTERRUPT] = 1u;
        entries[OS_INPUT_INTERRUPT] = OS_ISR_ENTRY(ParentISR);
    }
    if (strncmp(scenario, "invalid-", 8u) == 0) {
        /* Resource validation must not mask the new callback-catalog error. */
        target.resources = NULL;
        target.resource_count = 0u;
    }
    const StatusType prepared = Os_TargetPrepare(&target);
    if (strncmp(scenario, "invalid-", 8u) == 0) {
        printf("entry_bodies rejection=%s status=%u started=%u\n", scenario, prepared,
               isOsStarted());
        return prepared == E_OS_VALUE && isOsStarted() == FALSE ? 0 : 1;
    }
    check(prepared == E_OK);
    StartOS(1u);
    return 99;
}
