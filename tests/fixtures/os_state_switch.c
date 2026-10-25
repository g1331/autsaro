/** @file Observe the real backend's rejected kernel-state query and critical cleanup. */
#include "Os_Backend.h"
#include <stdio.h>

static OS_THREAD_LOCAL unsigned observing;
static OS_THREAD_LOCAL unsigned queries;
static OS_THREAD_LOCAL unsigned guards;
static OS_THREAD_LOCAL unsigned entries;
static OS_THREAD_LOCAL unsigned exits;

eTaskState __real_eTaskGetState(TaskHandle_t task);
void __real_vPortEnterCritical(void);
void __real_vPortExitCritical(void);

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
#include "lifecycle.c"
