/** @file A Task returning without TerminateTask raises E_OS_MISSINGEND and is cleaned up. */
#include "Os_Target.h"
#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const uint8 task_name_0[] = "owner";   /**< Process-lifetime Task name bytes. */
static const uint8 task_name_1[] = "helper";  /**< Process-lifetime Task name bytes. */
static const uint8 task_name_2[] = "monitor"; /**< Process-lifetime Task name bytes. */

static const char *scenario;  /**< Main-owned scenario selection before StartOS; no Init reset. */
static bool configured;       /**< Main-owned configured selection before StartOS; no Init reset. */
static bool resources;        /**< Main-owned resources selection before StartOS; no Init reset. */
static bool internal;         /**< Main-owned internal selection before StartOS; no Init reset. */
static bool queued;           /**< Main-owned queued selection before StartOS; no Init reset. */
static unsigned entries;      /**< Entries observations; fixture-owned, no Init reset. */
static unsigned errors;       /**< Observed checks/errors; fixture-owned, no Init reset. */
static unsigned helpers;      /**< Helpers observations; fixture-owned, no Init reset. */
static StatusType last_error; /**< Latest ErrorHook result; fixture-owned, no Init reset. */
static OSServiceIdType last_service; /**< Latest ErrorHook service; fixture-owned, no Init reset. */
static char trace[128];              /**< Execution-order bytes; fixture-owned, no Init reset. */
static unsigned length; /**< Used execution-order bytes; fixture-owned, no Init reset. */
static void check(bool condition)
{
    if (!condition) {
        ShutdownOS(E_OS_STATE);
    }
}
static void record(char marker)
{
    check(length + 1u < sizeof(trace));
    trace[length++] = marker;
    trace[length] = '\0';
}
static TaskType running(void)
{
    TaskType id = INVALID_TASK;
    TaskStateType state = 99u;
    check(GetTaskID(&id) == E_OK && GetTaskState(id, &state) == E_OK && state == RUNNING);
    return id;
}
void ErrorHook(StatusType error)
{
    Os_ActivationInfoType info;
    check(error == E_OS_MISSINGEND && running() == 0u);
    check(OSErrorGetServiceId() == OSServiceId_TaskMissingEnd);
    check(Os_TargetInspectActivation(0u, &info) == E_OK && info.state == RUNNING);
    check(info.count == (queued ? 4u - entries : 1u));
    check(info.resource_count == (resources ? 3u : 0u));
    check(info.internal_held == (internal ? 1u : 0u));
    ++errors;
    last_error = error;
    last_service = OSErrorGetServiceId();
    record('e');
}
void PreTaskHook(void)
{
    const TaskType id = running();
    record('p');
    record((char)('0' + id));
}
void PostTaskHook(void)
{
    const TaskType id = running();
    Os_ActivationInfoType info;
    check(Os_TargetInspectActivation(id, &info) == E_OK && info.resource_count == 0u);
    if (id == 0u) {
        check(errors == entries && info.internal_held == (internal ? 1u : 0u));
    }
    record('q');
    record((char)('0' + id));
}
void StartupHook(void) { check(GetActiveApplicationMode() == 1u); }
void ShutdownHook(StatusType error)
{
    printf("returned scenario=%s entries=%u errors=%u helper=%u status=%u service=%u trace=%s "
           "reason=%u\n",
           scenario,
           entries,
           errors,
           helpers,
           last_error,
           last_service,
           trace,
           error);
}
static void owner(void)
{
    ++entries;
    record((char)('A' + entries - 1u));
    if (queued && entries == 1u) {
        check(ActivateTask(0u) == E_OK && ActivateTask(0u) == E_OK);
    }
    if (resources) {
        check(GetResource(0u) == E_OK && GetResource(1u) == E_OK &&
              GetResource(RES_SCHEDULER) == E_OK);
    }
    if ((resources || internal) && entries == 1u) {
        check(ActivateTask(1u) == E_OK && helpers == 0u);
    }
    /* Intentionally return with active requests/resources. The real native
     * trampoline must report, clean, terminate and continue other Tasks. */
}
static void helper(void)
{
    Os_ActivationInfoType info;
    check(Os_TargetInspectActivation(0u, &info) == E_OK);
    check(info.state == (queued ? READY : SUSPENDED));
    check(info.count == (queued ? 2u : 0u) && info.resource_count == 0u &&
          info.internal_held == 0u && info.effective_priority == 2u);
    check(errors == (configured ? 1u : 0u));
    check(GetResource(0u) == E_OK && GetResource(1u) == E_OK && GetResource(RES_SCHEDULER) == E_OK);
    check(ReleaseResource(RES_SCHEDULER) == E_OK && ReleaseResource(1u) == E_OK &&
          ReleaseResource(0u) == E_OK);
    ++helpers;
    record('H');
    (void)TerminateTask();
    ShutdownOS(E_OS_STATE);
}
static void monitor(void)
{
    Os_ActivationInfoType info;
    check(Os_TargetInspectActivation(0u, &info) == E_OK && info.state == SUSPENDED &&
          info.count == 0u && info.resource_count == 0u && info.internal_held == 0u);
    check(entries == (queued ? 3u : 1u) && errors == (configured ? entries : 0u));
    check(helpers == ((resources || internal) ? 1u : 0u));
    record('M');
    ShutdownOS(E_OK);
}
int main(int argc, char **argv)
{
    check(argc == 2);
    scenario = argv[1];
    configured = strcmp(scenario, "unconfigured") != 0;
    resources = strcmp(scenario, "resources") == 0 || strcmp(scenario, "queued-resources") == 0;
    internal = strcmp(scenario, "internal") == 0;
    queued = strcmp(scenario, "queued") == 0 || strcmp(scenario, "queued-resources") == 0;
    const Os_TaskConfigType tasks[3] = {
        {0u,
         task_name_0,
         owner,
         2u,
         1u,
         strcmp(scenario, "extended") == 0 ? OS_EXTENDED_TASK : OS_BASIC_TASK,
         queued ? 3u : 1u,
         internal ? OS_SCHEDULE_NON : OS_SCHEDULE_FULL,
         0u},
        {1u, task_name_1, helper, 3u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {2u, task_name_2, monitor, 1u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
    };
    const Os_ResourceConfigType external[3] = {
        {0u, 3u, 3u, 0u, 0u, 0u}, {1u, 3u, 3u, 0u, 0u, 0u}, {RES_SCHEDULER, 3u, 7u, 0u, 0u, 0u}};
    const Os_HookConfigType hooks = {&ErrorHook, &PreTaskHook, &PostTaskHook};
    const Os_TargetConfigType target = {tasks,
                                        3u,
                                        262144u,
                                        external,
                                        3u,
                                        NULL,
                                        0u,
                                        0u,
                                        0u,
                                        0u,
                                        NULL,
                                        configured ? &hooks : NULL,
                                        NULL,
                                        0u};
    check(Os_TargetPrepare(&target) == E_OK);
    StartOS(1u);
    return 99;
}
