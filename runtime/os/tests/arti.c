/** @file ARTI trace of task transitions, hook events and event-buffer bounds. */
#include <limits.h>
#include "Os_Arti.h"
static void observed_hook(void);
#define TASK_HOOK_OBSERVER observed_hook
#define main               transition_main
#include "task_hooks.c"
#undef main

static bool
    stack_published[3]; /**< Observed Task stack publication; fixture-owned, no Init reset. */
static void observed_hook(void)
{
    TaskType id = INVALID_TASK;
    Os_ActivationInfo actual;
    check(GetTaskID(&id) == E_OK && id < 3u);
    check(GetCoreID() == OS_CORE_ID_0);
    check(Os_TargetInspectActivation(id, &actual) == E_OK);
    check(Os_ArtiTasks[id].state == actual.state && actual.state == RUNNING);
    check(Os_ArtiTasks[id].priority == actual.effective_priority);
    check(Os_ArtiTasks[id].activations == actual.count);
    if (Os_ArtiTaskStacks[id] == NULL) {
        /* A task thread publishes its stack when it first runs, after its first PreTaskHook. */
        check(!stack_published[id]);
        stack_published[id] = true;
        return;
    }
    stack_published[id] = true;
    check(Os_ArtiTaskStacks[id] != NULL && Os_ArtiTaskStacks[id]->role == 'T');
    check(Os_ArtiTaskStacks[id]->sp >= Os_ArtiTaskStacks[id]->reserve_low);
    check(Os_ArtiTaskStacks[id]->sp < Os_ArtiTaskStacks[id]->high);
    check(Os_ArtiTaskContexts[id] != NULL && Os_ArtiNativeContextSize == sizeof(CONTEXT));
#ifdef __linux__
    {
        const Os_NativeStack *native = Os_ArtiTaskStacks[id];
        const CONTEXT *saved = Os_ArtiTaskContexts[id];
        check(native->context_valid == 1u && native->altstack_size >= 65536u);
        check(native->kernel_buffer_low < native->kernel_buffer_high);
        check(native->kernel_buffer_high <= native->reserve_low ||
              native->kernel_buffer_low >= native->high);
        check(saved->ContextFlags == CONTEXT_CONTROL && saved->Rsp >= native->committed_low &&
              saved->Rsp < native->high);
        check((uintptr_t)saved->native.uc_mcontext.gregs[REG_RSP] == saved->Rsp);
        check(saved->native.uc_mcontext.fpregs == &saved->native.__fpregs_mem);
    }
#endif
}

static void arti_check(int accepted)
{
    if (accepted == 0) {
        fprintf(stderr, "ARTI consumer assertion failed after actor shutdown\n");
        ExitProcess(7u);
    }
}
void Os_ArtiTestShutdownObserved(void)
{
    char observed[128] = {0};
    size_t used = 0u;
    unsigned pre_start = 0u, pre_return = 0u, post_start = 0u, post_return = 0u;
    unsigned startup_start = 0u, startup_return = 0u, shutdown_start = 0u, shutdown_return = 0u;
    unsigned rejected_getters = 0u;
    unsigned core_start = 0u, core_return = 0u;
    const char *expected = "A0A2S0A1P0S1T1S0W0S2R0P2S0T0A1S1T1S2A0P2S0";
    if (strcmp(scenario, "self-chain") == 0 || strcmp(scenario, "queued") == 0) {
        expected = "A0A2S0T0A0S0T0S2";
    } else if (strcmp(scenario, "shutdown") == 0 || strcmp(scenario, "noop-yield") == 0 ||
               strcmp(scenario, "wait-satisfied") == 0) {
        expected = "A0A2S0";
    }
    arti_check(Arti_EventCount > 0 && Arti_EventCount <= (long)ARTI_EVENT_CAPACITY);
    arti_check(Arti_EventsDropped == 0 && Arti_DevelopmentError == 0);
    for (long i = 0; i < Arti_EventCount; ++i) {
        const Arti_Event *event = &Arti_Events[i];
        arti_check(event->published == 1L);
        arti_check(strcmp(event->instance, "Os") == 0 && event->instance_parameter == 0u);
        if (strcmp(event->class_name, "AR_CP_OS_TASK") == 0) {
            char code = 0;
            if (strcmp(event->event, "OsTask_Activate") == 0) {
                code = 'A';
            } else if (strcmp(event->event, "OsTask_Start") == 0) {
                code = 'S';
            } else if (strcmp(event->event, "OsTask_Preempt") == 0) {
                code = 'P';
            } else if (strcmp(event->event, "OsTask_Wait") == 0) {
                code = 'W';
            } else if (strcmp(event->event, "OsTask_Release") == 0) {
                code = 'R';
            } else if (strcmp(event->event, "OsTask_Terminate") == 0) {
                code = 'T';
            }
            arti_check(code != 0 && event->event_parameter < 3u && used + 2u < sizeof(observed));
            arti_check(strcmp(event->context, "NOSUSP") == 0);
            observed[used++] = code;
            observed[used++] = (char)('0' + event->event_parameter);
        } else if (strcmp(event->class_name, "AR_CP_OS_HOOK") == 0) {
            arti_check(strcmp(event->context, "SPRVSR") == 0 && event->event_parameter == 0u);
            if (strcmp(event->event, "OsHook_PreTaskHook_Start") == 0) {
                ++pre_start;
            } else if (strcmp(event->event, "OsHook_PreTaskHook_Return") == 0) {
                ++pre_return;
            } else if (strcmp(event->event, "OsHook_PostTaskHook_Start") == 0) {
                ++post_start;
            } else if (strcmp(event->event, "OsHook_PostTaskHook_Return") == 0) {
                ++post_return;
            } else if (strcmp(event->event, "OsHook_StartupHook_Start") == 0) {
                ++startup_start;
            } else if (strcmp(event->event, "OsHook_StartupHook_Return") == 0) {
                ++startup_return;
            } else if (strcmp(event->event, "OsHook_ShutdownHook_Start") == 0) {
                ++shutdown_start;
            } else if (strcmp(event->event, "OsHook_ShutdownHook_Return") == 0) {
                ++shutdown_return;
            } else {
                arti_check(false);
            }
        } else {
            arti_check(strcmp(event->class_name, "AR_CP_OS_SERVICECALLS") == 0);
            if (strcmp(event->event, "OsServiceCall_GetCounterValue_Return") == 0) {
                arti_check(event->event_parameter == UINT32_MAX && event->status_valid == 1u &&
                           event->service_status == 2u);
                ++rejected_getters;
            }
            if (strcmp(event->event, "OsServiceCall_GetCoreID_Start") == 0) {
                arti_check(event->event_parameter == 0u);
                ++core_start;
            } else if (strcmp(event->event, "OsServiceCall_GetCoreID_Return") == 0) {
                arti_check(event->event_parameter == OS_CORE_ID_0 && core_return < core_start);
                ++core_return;
            } else {
                /* Other application service calls are checked by their own vectors. */
            }
            arti_check(strcmp(event->event, "OsServiceCall_StartOS_Return") != 0);
            arti_check(strcmp(event->event, "OsServiceCall_ShutdownOS_Return") != 0);
            arti_check(strcmp(event->event, "OsServiceCall_TerminateTask_Return") != 0);
            arti_check(strcmp(event->event, "OsServiceCall_ChainTask_Return") != 0);
        }
    }
    arti_check(strcmp(observed, expected) == 0);
    arti_check(startup_start == 1u && startup_return == 1u && shutdown_start == 1u &&
               shutdown_return == 1u);
    arti_check(pre_start == pre_return && post_start == post_return);
    if (configured) {
        unsigned pre_expected = 0u, post_expected = 0u;
        for (size_t i = 0u; i < length; ++i) {
            if (trace[i] == 'p') {
                ++pre_expected;
            }
            if (trace[i] == 'q') {
                ++post_expected;
            }
        }
        arti_check(pre_start == pre_expected && post_start == post_expected);
        arti_check(rejected_getters == pre_start + post_start);
        arti_check(core_start == pre_start + post_start && core_return == core_start);
    } else {
        arti_check(pre_start == 0u && post_start == 0u && rejected_getters == 0u);
        arti_check(core_start == 0u && core_return == 0u);
    }
    printf("arti scenario=%s transitions=%s hooks=%u rejected_getters=%u dropped=0\n",
           scenario,
           observed,
           pre_start + post_start + 2u,
           rejected_getters);
}
int main(int argc, char **argv)
{
    if (argc == 2 && strcmp(argv[1], "buffer") == 0) {
        Std_VersionInfoType version = {99u, 99u, 99u, 99u, 99u};
        uint32_t core = 0u, value = 7u;
        Arti_Init();
        Arti_GetVersionInfo(&version);
        if (version.vendorID != 0u || version.moduleID != 0u || version.sw_major_version != 1u ||
            version.sw_minor_version != 0u || version.sw_patch_version != 0u) {
            return 7;
        }
        Arti_GetVersionInfo(NULL);
        if (Arti_DevelopmentError != 2L) {
            return 7;
        }
        ARTI_TRACE(USER, TOOL_TEST, TestInstance, core++, Once, value++);
        if (core != 1u || value != 8u || Arti_Events[0].event_parameter != 7u) {
            return 7;
        }
        for (unsigned i = 1u; i < ARTI_EVENT_CAPACITY + 3u; ++i) {
            ARTI_TRACE(USER, TOOL_TEST, TestInstance, 0u, Fill, i);
        }
        if (Arti_EventCount != (long)ARTI_EVENT_CAPACITY || Arti_EventsDropped != 3L ||
            Arti_Events[ARTI_EVENT_CAPACITY - 1u].event_parameter != ARTI_EVENT_CAPACITY - 1u) {
            return 7;
        }
        Arti_EventsDropped = INT32_MAX;
        ARTI_TRACE(USER, TOOL_TEST, TestInstance, 0u, Saturated, 0u);
        if (Arti_EventsDropped != INT32_MAX) {
            return 7;
        }
        printf("arti buffer=4096 dropped=3 once=1 version=1 error=2 saturated=1\n");
        return 0;
    }
    return transition_main(argc, argv);
}
