#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static TaskHandle_t observed[4];
static unsigned checks;
static unsigned rejects;
static unsigned consumed;
static unsigned entries;
static uint64_t consumed_ticket;
static volatile LONG request;
static volatile LONG completed;
static volatile LONG notified;
static volatile LONG waiting_seen;
static volatile LONG bridge_rejected;
static bool mailbox;
static void check(bool condition) {
    ++checks;
    if (!condition) {
        Os_TargetTrace('!');
        ShutdownOS(E_OS_STATE);
    }
}
static void refuse(StatusType status, StatusType expected) {
    check(status == expected);
    ++rejects;
}
static EventMaskType bits(TaskType id) {
    EventMaskType value = UINT32_MAX;
    check(GetEvent(id, &value) == E_OK);
    return value;
}
void Os_TestObserve(void) {
    unsigned i;
    for (i = 0u; i < 4u; ++i) {
        Os_ActivationInfo value;
        eTaskState state = eTaskGetState(observed[i]);
        check(Os_TargetInspectActivation((TaskType)i, &value) == E_OK);
        if (value.waiting != 0u) {
            check((state == eSuspended) && (value.state == WAITING) && (value.count == 1u) &&
                  (value.kernel_sequence == 0u));
            if ((i == 0u) && (strcmp(scenario, "mailbox-wait") == 0)) {
                if (InterlockedCompareExchange(&waiting_seen, 1, 0) == 0) {
                    InterlockedExchange(&request, 1);
                }
            }
        } else if (value.count == 0u) {
            check((state == eSuspended) && (value.state == SUSPENDED));
        } else {
            check((state == eRunning) || (state == eReady));
            check(value.state == ((state == eRunning) ? RUNNING : READY));
        }
    }
}
void Os_TestInputNotified(void) { InterlockedIncrement(&notified); }
static void wait_flag(volatile LONG *flag, LONG expected) {
    DWORD started_at = GetTickCount();
    while (InterlockedCompareExchange(flag, 0, 0) < expected) {
        check((GetTickCount() - started_at) < 4000u);
        Sleep(1u);
    }
}
static DWORD WINAPI other_producer(void *argument) {
    uint8_t data = 99u;
    uint64_t ticket = UINT64_C(777);
    (void)argument;
    return ((Os_TargetPostInput(&data, 1u, &ticket) == E_OS_ACCESS) && (ticket == UINT64_C(777)))
               ? 0u
               : 99u;
}
static DWORD WINAPI producer_thread(void *argument) {
    unsigned total = 0u;
    (void)argument;
    for (;;) {
        LONG batch = InterlockedExchange(&request, 0);
        unsigned i;
        if (batch == 0) {
            Sleep(1u);
            continue;
        }
        for (i = 0u; i < (unsigned)batch; ++i) {
            uint8_t data[2];
            uint64_t ticket = 0u;
            data[0] = (uint8_t)((total % 2u) + 1u);
            data[1] = (uint8_t)(total % 256u);
            if ((Os_TargetPostInput(data, 2u, &ticket) != E_OK) ||
                (ticket != ((strcmp(scenario, "mailbox-exhaustion") == 0)
                                ? UINT64_MAX
                                : ((uint64_t)total + UINT64_C(1))))) {
                ExitProcess(99u);
            }
            ++total;
        }
        if (strcmp(scenario, "mailbox-api") == 0) {
            uint8_t data = 99u;
            uint64_t ticket = UINT64_C(777);
            EventMaskType mask = 123u;
            Os_InputRecord record = {0};
            unsigned count = 777u;
            HANDLE handle;
            DWORD result;
            record.ticket = UINT64_C(777);
            if ((Os_TargetPostInput(NULL, 1u, &ticket) != E_OS_ILLEGAL_ADDRESS) ||
                (Os_TargetPostInput(&data, 1u, NULL) != E_OS_ILLEGAL_ADDRESS) ||
                (Os_TargetPostInput(&data, 0u, &ticket) != E_OS_VALUE) ||
                (Os_TargetPostInput(&data, 33u, &ticket) != E_OS_VALUE) ||
                (ticket != UINT64_C(777)) || (SetEvent(0u, 4u) != E_OS_CALLEVEL) ||
                (GetEvent(0u, &mask) != E_OS_CALLEVEL) || (mask != 123u) ||
                (Os_TargetTakeInput(&record) != E_OS_CALLEVEL) ||
                (record.ticket != UINT64_C(777)) ||
                (Os_TargetInputCount(&count) != E_OS_CALLEVEL) || (count != 777u)) {
                ExitProcess(99u);
            }
            handle = CreateThread(NULL, 0u, other_producer, NULL, 0u, NULL);
            if ((handle == NULL) || (WaitForSingleObject(handle, 4000u) != WAIT_OBJECT_0) ||
                (GetExitCodeThread(handle, &result) == 0) || (result != 0u) ||
                (CloseHandle(handle) == 0)) {
                ExitProcess(99u);
            }
            InterlockedExchange(&bridge_rejected, 9);
        }
        if ((strcmp(scenario, "mailbox-full") == 0) ||
            (strcmp(scenario, "mailbox-exhaustion") == 0)) {
            uint8_t data = 99u;
            uint64_t unchanged = UINT64_C(777);
            if ((Os_TargetPostInput(&data, 1u, &unchanged) != E_OS_LIMIT) ||
                (unchanged != UINT64_C(777))) {
                ExitProcess(99u);
            }
            InterlockedIncrement(&bridge_rejected);
        }
        InterlockedExchange(&completed, (LONG)total);
    }
}
static void publish(unsigned count) {
    LONG before = InterlockedCompareExchange(&completed, 0, 0);
    LONG signals = InterlockedCompareExchange(&notified, 0, 0);
    InterlockedExchange(&request, (LONG)count);
    wait_flag(&completed, (before + (LONG)count));
    /* Pending-bit coalescing is allowed. Wait for one real notification, rather
     * than assuming that each record becomes an independent event edge. */
    wait_flag(&notified, (signals + 1));
}
static void drain(void) {
    Os_InputRecord value;
    StatusType status = Os_TargetTakeInput(&value);
    while (status == E_OK) {
        check(value.length == 2u);
        check(value.ticket == ((strcmp(scenario, "mailbox-exhaustion") == 0)
                                   ? UINT64_MAX
                                   : (consumed_ticket + UINT64_C(1))));
        check(value.data[0] == (uint8_t)((consumed % 2u) + 1u));
        check(value.data[1] == (uint8_t)(consumed % 256u));
        consumed_ticket = value.ticket;
        ++consumed;
        status = Os_TargetTakeInput(&value);
    }
    check(status == E_OS_NOFUNC);
}
static void finish(void) {
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
static uint32_t interrupt(void) {
    EventMaskType output = UINT32_C(123);
    TaskStateType state = 99u;
    if ((strcmp(scenario, "category1") == 0) || (strcmp(scenario, "category1-ceiling") == 0)) {
        refuse(SetEvent(0u, 1u), E_OS_CALLEVEL);
        refuse(GetEvent(0u, &output), E_OS_CALLEVEL);
        check(output == UINT32_C(123));
        refuse(GetTaskState(0u, &state), E_OS_CALLEVEL);
        check(state == 99u);
        refuse(ActivateTask(3u), E_OS_CALLEVEL);
        refuse(GetResource(0u), E_OS_CALLEVEL);
        refuse(ReleaseResource(0u), E_OS_CALLEVEL);
    } else {
        check(SetEvent(0u, 1u) == E_OK);
        check(GetEvent(0u, &output) == E_OK);
        check(output == 1u);
        check(GetResource(0u) == E_OK);
        check(ReleaseResource(0u) == E_OK);
    }
    refuse(WaitEvent(1u), E_OS_CALLEVEL);
    refuse(ClearEvent(1u), E_OS_CALLEVEL);
    refuse(TerminateTask(), E_OS_CALLEVEL);
    refuse(ChainTask(3u), E_OS_CALLEVEL);
    Os_TargetTrace('J');
    return 0u;
}
static void owner(void) {
    EventMaskType output = 123u;
    unsigned count;
    ++entries;
    Os_TargetTrace('E');
    if (strcmp(scenario, "mailbox-replace-running") == 0) {
        vPortSetInterruptHandler(OS_INPUT_INTERRUPT, interrupt);
        Os_TargetTrace('X');
        ShutdownOS(E_OS_STATE);
    }
    if (mailbox) {
        if (strcmp(scenario, "mailbox-api") == 0) {
            check(ActivateTask(1u) == E_OK);
            check(WaitEvent(2u) == E_OK);
            check(ClearEvent(2u) == E_OK);
        }
        if (strcmp(scenario, "mailbox-wait") == 0) {
            check(WaitEvent(4u) == E_OK);
            check(InterlockedCompareExchange(&waiting_seen, 0, 0) == 1);
            wait_flag(&completed, 1);
        } else if (strcmp(scenario, "mailbox-between") == 0) {
            check(SetEvent(0u, 4u) == E_OK);
            output = bits(0u);
            publish(2u);
            check(ClearEvent(output) == E_OK);
            check(bits(0u) == 0u);
        } else if (strcmp(scenario, "mailbox-after-clear") == 0) {
            check(SetEvent(0u, 4u) == E_OK);
            check(ClearEvent(bits(0u)) == E_OK);
            publish(2u);
        } else if (strcmp(scenario, "mailbox-empty-wait") == 0) {
            check(Os_TargetInputCount(&count) == E_OK);
            check(count == 0u);
            publish(2u);
            check(WaitEvent(4u) == E_OK);
        } else {
            publish((strcmp(scenario, "mailbox-full") == 0)
                        ? 256u
                        : ((strcmp(scenario, "mailbox-exhaustion") == 0) ? 1u : 2u));
        }
        drain();
        check(Os_TargetInputCount(&count) == E_OK);
        check(count == 0u);
        check(ClearEvent(bits(0u)) == E_OK);
        if (strcmp(scenario, "mailbox-wrap") == 0) {
            unsigned i;
            for (i = 0u; i < 260u; ++i) {
                publish(2u);
                drain();
                check(ClearEvent(bits(0u)) == E_OK);
            }
        }
        Os_TargetTrace('q');
    } else if (strcmp(scenario, "already") == 0) {
        check(SetEvent(0u, 1u) == E_OK);
        check(WaitEvent(1u) == E_OK);
        check(WaitEvent(1u) == E_OK);
        check(bits(0u) == 1u);
        check(ClearEvent(1u) == E_OK);
        check(bits(0u) == 0u);
        Os_TargetTrace('a');
    } else if (strcmp(scenario, "ownership") == 0) {
        check(ActivateTask(1u) == E_OK);
        check(SetEvent(0u, 1u) == E_OK);
        check(SetEvent(1u, 1u) == E_OK);
        check(ClearEvent(1u) == E_OK);
        check(bits(0u) == 0u);
        check(bits(1u) == 1u);
        check(WaitEvent(2u) == E_OK);
        check(bits(0u) == 2u);
        Os_TargetTrace('w');
    } else if ((strcmp(scenario, "wait") == 0) || (strcmp(scenario, "oracle-wait") == 0)) {
        check(ActivateTask(1u) == E_OK);
        check(WaitEvent(1u) == E_OK);
        check(bits(0u) == 1u);
        check(ClearEvent(1u) == E_OK);
        Os_TargetTrace('w');
    } else if (strcmp(scenario, "new-instance") == 0) {
        if (entries == 1u) {
            check(SetEvent(0u, UINT32_MAX) == E_OK);
            (void)ChainTask(0u);
            Os_TargetTrace('X');
            ShutdownOS(E_OS_STATE);
        }
        check(bits(0u) == 0u);
        Os_TargetTrace('n');
    } else if (strcmp(scenario, "errors") == 0) {
        Os_ActivationInfo before;
        Os_ActivationInfo after;
        check(Os_TargetInspectActivation(0u, &before) == E_OK);
        refuse(SetEvent(15u, 1u), E_OS_ID);
        refuse(GetEvent(15u, &output), E_OS_ID);
        check(output == 123u);
        refuse(SetEvent(2u, 1u), E_OS_ACCESS);
        refuse(GetEvent(2u, &output), E_OS_ACCESS);
        refuse(SetEvent(3u, 1u), E_OS_STATE);
        refuse(GetEvent(3u, &output), E_OS_STATE);
        refuse(GetEvent(0u, NULL), E_OS_ILLEGAL_ADDRESS);
        check(GetResource(0u) == E_OK);
        refuse(WaitEvent(1u), E_OS_RESOURCE);
        check(ReleaseResource(0u) == E_OK);
        check(Os_TargetInspectActivation(0u, &after) == E_OK);
        check((before.events == after.events) && (before.count == after.count) &&
              (before.requests[0] == after.requests[0]) &&
              (before.kernel_sequence == after.kernel_sequence) && (after.state == RUNNING));
        Os_TargetTrace('r');
    } else {
        bool category1 =
            (strcmp(scenario, "category1") == 0) || (strcmp(scenario, "category1-ceiling") == 0);
        if (strcmp(scenario, "category1-ceiling") == 0) {
            check(GetResource(0u) == E_OK);
        }
        vPortGenerateSimulatedInterrupt(category1 ? 31u : 29u);
        if (strcmp(scenario, "category1-ceiling") == 0) {
            check(ReleaseResource(0u) == E_OK);
        }
        check(bits(0u) == (category1 ? 0u : 1u));
        Os_TargetTrace('i');
    }
    if (strcmp(scenario, "oracle-wait") == 0) {
        check(ActivateTask(2u) == E_OK);
    }
    finish();
}
static void peer(void) {
    Os_TargetTrace('P');
    if (strcmp(scenario, "mailbox-api") == 0) {
        Os_InputRecord record = {0};
        unsigned count = 777u;
        record.ticket = UINT64_C(777);
        refuse(Os_TargetTakeInput(&record), E_OS_ACCESS);
        check(record.ticket == UINT64_C(777));
        refuse(Os_TargetInputCount(&count), E_OS_ACCESS);
        check(count == 777u);
        check(SetEvent(0u, 2u) == E_OK);
    } else if (strcmp(scenario, "ownership") == 0) {
        check(bits(1u) == 1u);
        check(ClearEvent(1u) == E_OK);
        check(bits(1u) == 0u);
        check(SetEvent(0u, 2u) == E_OK);
    } else {
        check(SetEvent(0u, 1u) == E_OK);
    }
    /* The higher-priority owner already completed before SetEvent returns. */
    {
        TaskStateType state;
        check(GetTaskState(0u, &state) == E_OK);
        check(state == SUSPENDED);
    }
    Os_TargetTrace('p');
    finish();
}
static void monitor(void) {
    if (strcmp(scenario, "mailbox-wait") == 0) {
        TaskStateType state = WAITING;
        DWORD started_at = GetTickCount();
        while (state != SUSPENDED) {
            check(GetTaskState(0u, &state) == E_OK);
            check((GetTickCount() - started_at) < 4000u);
            Sleep(1u);
        }
    }
    refuse(WaitEvent(1u), E_OS_ACCESS);
    refuse(ClearEvent(1u), E_OS_ACCESS);
    Os_TargetTrace('M');
    ShutdownOS(E_OK);
}
static void inactive(void) { ShutdownOS(E_OS_STATE); }
void StartupHook(void) {
    unsigned i;
    for (i = 0u; i < 4u; ++i) {
        observed[i] = xTaskGetHandle(Os_Config->tasks[i].name);
    }
    vPortSetInterruptHandler(29u, interrupt);
    vPortSetInterruptHandler(31u, interrupt);
    if (mailbox) {
        if (strcmp(scenario, "mailbox-exhaustion") == 0) {
            Os_TestInputLastTicket(UINT64_MAX - UINT64_C(1));
        }
        HANDLE handle = CreateThread(NULL, 0u, producer_thread, NULL, 0u, NULL);
        check(handle != NULL);
        check(CloseHandle(handle) != 0);
    }
    Os_TargetTrace('S');
    if (strcmp(scenario, "mailbox-replace-startup") == 0) {
        vPortSetInterruptHandler(OS_INPUT_INTERRUPT, interrupt);
        Os_TargetTrace('X');
        ShutdownOS(E_OS_STATE);
    }
}
void ShutdownHook(StatusType Error) {
    Os_TargetTrace('Z');
    if (printf("event checks=%u rejected=%u records=%u ticket=%llu entries=%u error=%u "
               "bridge_rejected=%d\n",
               checks, rejects, consumed, (unsigned long long)consumed_ticket, entries, Error,
               (int)InterlockedCompareExchange(&bridge_rejected, 0, 0)) < 0) {
        ExitProcess(99u);
    }
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {
        {0u, "E", owner, 3u, 3u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "P", peer, 2u, 0u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {2u, "M", monitor, 1u, 3u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {3u, "N", inactive, 2u, 0u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_ResourceConfig resource = {0u, 31u, 1u, UINT32_C(1) << 29u};
    Os_TargetConfig config = {tasks, 4u, 262144u, &resource, 1u,  NULL, 0u, UINT32_C(1) << 31u,
                              0u,    0u, NULL,    NULL,      NULL};
    scenario = (argc == 2) ? argv[1] : "already";
    mailbox = strncmp(scenario, "mailbox-", 8u) == 0;
    if (strcmp(scenario, "oracle-wait") == 0) {
        tasks[0].priority = 2u;
        tasks[1].priority = 1u;
        tasks[2].autostart_modes = 0u;
    }
    if (mailbox) {
        config.input_event = 4u;
    }
    if (strcmp(scenario, "mailbox-mode") == 0) {
        tasks[0].autostart_modes = 1u;
    }
    if (strcmp(scenario, "input-invalid-task") == 0) {
        config.input_event = 4u;
        config.input_task = 15u;
    } else if (strcmp(scenario, "input-basic") == 0) {
        config.input_event = 4u;
        config.input_task = 2u;
    } else if (strcmp(scenario, "input-inactive") == 0) {
        config.input_event = 4u;
        config.input_task = 3u;
    } else if (strcmp(scenario, "input-category1") == 0) {
        config.input_event = 4u;
        config.category1_isrs |= UINT32_C(1) << OS_INPUT_INTERRUPT;
    } else if (strcmp(scenario, "category1-resource") == 0) {
        resource.isr_access |= UINT32_C(1) << 31u;
    } else {
        /* Runtime vectors keep their valid static configuration. */
    }
    if ((strncmp(scenario, "input-", 6u) == 0) || (strcmp(scenario, "category1-resource") == 0)) {
        StatusType expected = (strcmp(scenario, "input-invalid-task") == 0) ? E_OS_ID : E_OS_VALUE;
        StatusType status = Os_TargetPrepare(&config);
        if (printf("prepare=%u\n", status) < 0) {
            return 99;
        }
        return (status == expected) ? 0 : 99;
    }
    check(Os_TargetPrepare(&config) == E_OK);
    StartOS((strcmp(scenario, "mailbox-mode") == 0) ? 2u : 1u);
    return 99;
}
