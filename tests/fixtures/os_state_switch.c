/** @file Observe rejected kernel-state queries in the service and ARTI collector. */
#include "Os_Backend.h"
#include <stdio.h>
#include <string.h>

static OS_THREAD_LOCAL unsigned observing;
static OS_THREAD_LOCAL unsigned queries;
static OS_THREAD_LOCAL unsigned guards;
static OS_THREAD_LOCAL unsigned entries;
static OS_THREAD_LOCAL unsigned exits;
static unsigned inject_arti_state;
static OS_THREAD_LOCAL unsigned injected_arti_state;

eTaskState __real_eTaskGetState(TaskHandle_t task);
void __real_vPortEnterCritical(void);
void __real_vPortExitCritical(void);
int Os_StateLifecycleMain(int argc, char **argv);

void Os_StateGuardObserved(void)
{
    if (observing != 0u) {
        ++guards;
    }
}

eTaskState __wrap_eTaskGetState(TaskHandle_t task)
{
    if (observing != 0u) {
        ++queries;
        return eDeleted;
    }
    if ((inject_arti_state != 0u) && (injected_arti_state == 0u)) {
        const Os_NativeStackType *stack = Os_StackCurrent();
        /* The bootstrap's first state query is the ARTI observation after autostart. */
        if ((stack != NULL) && (stack->role == 'B')) {
            injected_arti_state = 1u;
            (void)fprintf(stderr, "ARTI_STATE_QUERY eDeleted\n");
            return eDeleted;
        }
    }
    return __real_eTaskGetState(task);
}

void __wrap_vPortEnterCritical(void)
{
    __real_vPortEnterCritical();
    if (observing != 0u) {
        ++entries;
    }
}

void __wrap_vPortExitCritical(void)
{
    __real_vPortExitCritical();
    if (observing != 0u) {
        ++exits;
    }
}

static StatusType observed_state(TaskType id, TaskStateRefType state)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    if ((stack != NULL) && (stack->role == 'T') && (id < 2u) && (state != NULL)) {
        TaskStateType unchanged = 99u;
        guards = 0u;
        queries = 0u;
        entries = 0u;
        exits = 0u;
        observing = 1u;
        const StatusType result = Os_BackendState(id, &unchanged);
        observing = 0u;
        if ((result != E_OS_STATE) || (unchanged != 99u) || (queries != 1u) || (guards != 2u) ||
            (entries != 1u) || (exits != 1u)) {
            (void)fprintf(
                stderr,
                "STATE_REJECTION result=%u output=%u queries=%u guards=%u enter=%u exit=%u\n",
                (unsigned)result,
                (unsigned)unchanged,
                queries,
                guards,
                entries,
                exits);
            ExitProcess(63u);
        }
    }
    return GetTaskState(id, state);
}

#define GetTaskState observed_state
#define main         Os_StateLifecycleMain
#include "lifecycle.c"
#undef main
#undef GetTaskState

int main(int argc, char **argv)
{
    if ((argc == 2) && (strcmp(argv[1], "invalid-arti-state") == 0)) {
        char normal[] = "normal";
        char *arguments[] = {argv[0], normal, NULL};
        inject_arti_state = 1u;
        return Os_StateLifecycleMain(2, arguments);
    }
    return Os_StateLifecycleMain(argc, argv);
}
