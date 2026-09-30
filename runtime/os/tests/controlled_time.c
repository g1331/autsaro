#include "Os_Backend.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static const char *scenario;
static volatile LONG producer_done;
static volatile LONG owner_held;
static volatile LONG release_owner;
static volatile LONG owner_entered;
static volatile LONG tick_mask_observed;
static volatile LONG tick_mask_kernel;
static volatile LONG dispatch_release_stage;
static unsigned work_count;
static unsigned app_count;
static unsigned completed_count;
static unsigned native_rejects;
static TaskHandle_t owner_handle;
static uint64_t initial_epoch;
static uint32_t initial_kernel;
static unsigned steps;
static Os_TickCompletion boundary[3];
static bool manual;
static unsigned standard_rejects;
static unsigned context_rejects;
static unsigned alarm_entries;
static unsigned callbacks;
static unsigned error_hooks;
static StatusType last_error;
static TickType final_counter;
typedef struct {
    uint64_t epoch;
    uint32_t kernel;
    TickType value;
    TickType elapsed;
} HardwareRecord;
static HardwareRecord hardware[20];
static unsigned hardware_count;
static TickType hardware_previous;
static void check(bool condition) {
    if (!condition) {
        Os_TargetTrace('!');
        ShutdownOS(E_OS_STATE);
    }
}
static void native_check(bool condition) {
    if (!condition) {
        ExitProcess(99u);
    }
}
static void native_refuse(StatusType status, StatusType expected) {
    native_check(status == expected);
    ++native_rejects;
}
void Os_TimeTestMasked(void) {
    if (strncmp(scenario, "tick-mask-", 10u) == 0 || strncmp(scenario, "tick-owner-", 11u) == 0) {
        InterlockedExchange(&tick_mask_kernel, (LONG)xTaskGetTickCountFromISR());
        InterlockedExchange(&tick_mask_observed, 1);
    }
}
void Os_TimeTestDispatchFinishing(void) {
    if (strncmp(scenario, "tick-owner-", 11u) == 0 &&
        InterlockedCompareExchange(&tick_mask_observed, 0, 0) != 0 &&
        xTaskGetTickCountFromISR() == 1u) {
        (void)InterlockedCompareExchange(&dispatch_release_stage, 1, 0);
    }
}
void Os_TimeTestDispatchUnlocked(void) {
    DWORD started_at = GetTickCount();
    while (InterlockedCompareExchange(&dispatch_release_stage, 0, 0) == 1) {
        native_check((GetTickCount() - started_at) < 4000u);
        Sleep(1u);
    }
}
void Os_TimeTestOnWaiting(void) {
    if (InterlockedCompareExchange(&dispatch_release_stage, 0, 0) == 1) {
        /* The ISR actor is deliberately paused after releasing its mutex.
         * This Task owns the next waiting commit: the old ISR flag must be off. */
        check(xInsideInterrupt == pdFALSE);
        InterlockedExchange(&dispatch_release_stage, 2);
    }
}
static void refuse(StatusType status, StatusType expected) {
    check(status == expected);
    ++standard_rejects;
}
static void alarm_callback(void) {
    ++callbacks;
    Os_TargetTrace('B');
}
static void alarm_error(StatusType error) {
    ++error_hooks;
    last_error = error;
}
static void inc(unsigned count) {
    unsigned i;
    for (i = 0u; i < count; ++i) {
        check(IncrementCounter(0u) == E_OK);
    }
}
static uint32_t service_interrupt(void) {
    TickType value = UINT64_C(777);
    TickType previous = 0u;
    TickType elapsed = UINT64_C(777);
    AlarmBaseType base = {0};
    if (strcmp(scenario, "category1-time") == 0) {
        refuse(IncrementCounter(0u), E_OS_CALLEVEL);
        refuse(GetCounterValue(0u, &value), E_OS_CALLEVEL);
        check(value == UINT64_C(777));
        refuse(GetElapsedValue(0u, &previous, &elapsed), E_OS_CALLEVEL);
        check((previous == 0u) && (elapsed == UINT64_C(777)));
        refuse(GetAlarmBase(0u, &base), E_OS_CALLEVEL);
        check(base.maxallowedvalue == 0u);
        refuse(GetAlarm(0u, &value), E_OS_CALLEVEL);
        check(value == UINT64_C(777));
        refuse(SetRelAlarm(0u, 1u, 0u), E_OS_CALLEVEL);
        refuse(SetAbsAlarm(0u, 0u, 0u), E_OS_CALLEVEL);
        refuse(CancelAlarm(0u), E_OS_CALLEVEL);
    } else {
        check(GetAlarmBase(0u, &base) == E_OK);
        check(base.maxallowedvalue == 15u);
        check(SetRelAlarm(0u, 1u, 0u) == E_OK);
        check(GetAlarm(0u, &value) == E_OK);
        check(value == 1u);
        check(IncrementCounter(0u) == E_OK);
        check(GetCounterValue(0u, &value) == E_OK);
        check(value == 1u);
        check(GetElapsedValue(0u, &previous, &elapsed) == E_OK);
        check((previous == 1u) && (elapsed == 1u));
        refuse(CancelAlarm(0u), E_OS_NOFUNC);
    }
    Os_TargetTrace('J');
    return 0u;
}
static void manual_alarms(void) {
    AlarmBaseType base;
    TickType remaining = UINT64_C(777);
    EventMaskType events;
    check(GetAlarmBase(0u, &base) == E_OK);
    check((base.maxallowedvalue == ((strcmp(scenario, "absolute-wide") == 0) ? UINT32_MAX : 15u)) &&
          (base.ticksperbase == 1u) && (base.mincycle == 1u));
    if (strcmp(scenario, "relative-wrap") == 0) {
        inc(14u);
        check(SetRelAlarm(0u, 3u, 0u) == E_OK);
        check(GetAlarm(0u, &remaining) == E_OK);
        check(remaining == 3u);
        refuse(SetRelAlarm(0u, 1u, 0u), E_OS_STATE);
        check(GetAlarm(0u, &remaining) == E_OK);
        check(remaining == 3u);
        inc(3u);
        check(alarm_entries == 1u);
        remaining = UINT64_C(777);
        refuse(GetAlarm(0u, &remaining), E_OS_NOFUNC);
        check(remaining == UINT64_C(777));
        refuse(CancelAlarm(0u), E_OS_NOFUNC);
    } else if (strcmp(scenario, "absolute-cycle") == 0) {
        TickType previous = 14u;
        TickType elapsed = UINT64_C(777);
        inc(14u);
        check(SetAbsAlarm(0u, 2u, 2u) == E_OK);
        inc(4u);
        check(GetEvent(0u, &events) == E_OK);
        check(events == 1u);
        check(ClearEvent(1u) == E_OK);
        inc(2u);
        check(GetEvent(0u, &events) == E_OK);
        check(events == 1u);
        check(ClearEvent(1u) == E_OK);
        check(GetElapsedValue(0u, &previous, &elapsed) == E_OK);
        check((previous == 4u) && (elapsed == 6u));
        check(CancelAlarm(0u) == E_OK);
        inc(2u);
        check(GetEvent(0u, &events) == E_OK);
        check(events == 0u);
        refuse(SetRelAlarm(0u, 1u, 16u), E_OS_VALUE);
        refuse(GetAlarm(0u, &remaining), E_OS_NOFUNC);
        check(remaining == UINT64_C(777));
    } else if (strcmp(scenario, "absolute-current") == 0) {
        inc(14u);
        check(SetAbsAlarm(0u, 14u, 0u) == E_OK);
        check(GetAlarm(0u, &remaining) == E_OK);
        check(remaining == 16u);
        inc(15u);
        check(callbacks == 0u);
        inc(1u);
        check(callbacks == 1u);
    } else if (strcmp(scenario, "absolute-wide") == 0) {
        TickType previous = UINT32_MAX;
        TickType elapsed = 0u;
        check(SetAbsAlarm(0u, UINT32_MAX, 0u) == E_OK);
        check(GetAlarm(0u, &remaining) == E_OK);
        check(remaining == UINT64_C(4294967296));
        inc(1u);
        check(GetElapsedValue(0u, &previous, &elapsed) == E_OK);
        check((previous == 0u) && (elapsed == 1u));
        check(callbacks == 0u);
        check(CancelAlarm(0u) == E_OK);
    } else if (strcmp(scenario, "action-error") == 0) {
        Os_ActivationInfo before;
        Os_ActivationInfo after;
        check(ActivateTask(2u) == E_OK);
        check(Os_TargetInspectActivation(2u, &before) == E_OK);
        check(SetRelAlarm(0u, 1u, 0u) == E_OK);
        inc(1u);
        check(Os_TargetInspectActivation(2u, &after) == E_OK);
        check((before.count == 1u) && (after.count == 1u) &&
              (before.requests[0] == after.requests[0]) &&
              (before.kernel_sequence == after.kernel_sequence) && (before.state == after.state) &&
              (before.events == after.events) &&
              (before.effective_priority == after.effective_priority) &&
              (before.resource_count == after.resource_count));
        check((error_hooks == 1u) && (last_error == E_OS_LIMIT));
        check(alarm_entries == 0u);
    } else if (strcmp(scenario, "callback") == 0) {
        check(SetRelAlarm(0u, 1u, 2u) == E_OK);
        inc(5u);
        check(callbacks == 3u);
        check(CancelAlarm(0u) == E_OK);
        inc(2u);
        check(callbacks == 3u);
    } else if ((strcmp(scenario, "category1-time") == 0) ||
               (strcmp(scenario, "category2-time") == 0)) {
        vPortGenerateSimulatedInterrupt(31u);
        check(GetEvent(0u, &events) == E_OK);
        check(events == ((strcmp(scenario, "category1-time") == 0) ? 0u : 1u));
    } else {
        TickType previous = 16u;
        TickType elapsed = UINT64_C(777);
        TickType value = UINT64_C(777);
        refuse(IncrementCounter(7u), E_OS_ID);
        refuse(IncrementCounter(1u), E_OS_ID);
        refuse(GetCounterValue(7u, &value), E_OS_ID);
        check(value == UINT64_C(777));
        refuse(GetCounterValue(0u, NULL), E_OS_ILLEGAL_ADDRESS);
        refuse(GetElapsedValue(0u, &previous, &elapsed), E_OS_VALUE);
        check((previous == 16u) && (elapsed == UINT64_C(777)));
        refuse(GetElapsedValue(0u, NULL, &elapsed), E_OS_ILLEGAL_ADDRESS);
        check(elapsed == UINT64_C(777));
        previous = 0u;
        refuse(GetElapsedValue(0u, &previous, NULL), E_OS_ILLEGAL_ADDRESS);
        check(previous == 0u);
        refuse(GetAlarmBase(0u, NULL), E_OS_ILLEGAL_ADDRESS);
        refuse(GetAlarm(0u, NULL), E_OS_ILLEGAL_ADDRESS);
        refuse(GetAlarm(15u, &value), E_OS_ID);
        check(value == UINT64_C(777));
        refuse(SetRelAlarm(0u, 0u, 0u), E_OS_VALUE);
        refuse(SetRelAlarm(0u, 16u, 0u), E_OS_VALUE);
        refuse(SetAbsAlarm(0u, 16u, 0u), E_OS_VALUE);
        refuse(SetRelAlarm(0u, 1u, 16u), E_OS_VALUE);
        refuse(CancelAlarm(0u), E_OS_NOFUNC);
    }
    check(GetCounterValue(0u, &final_counter) == E_OK);
    check(xTaskGetTickCount() == 0u);
    (void)InterlockedExchange(&producer_done, 1);
    Os_TargetTrace('r');
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
void Os_TimeTestBeforePending(void) {
    if (strcmp(scenario, "close-before-pending") == 0) {
        Os_TimeClose();
    }
}
static DWORD WINAPI requester(void *argument) {
    uint64_t ticket = UINT64_C(777);
    unsigned i;
    DWORD started_at;
    (void)argument;
    started_at = GetTickCount();
    while ((Os_TargetReady() == 0) || (InterlockedCompareExchange(&owner_entered, 0, 0) == 0)) {
        native_check((GetTickCount() - started_at) < 4000u);
        Sleep(1u);
    }
    if ((strcmp(scenario, "reset-failure") == 0) ||
        (strcmp(scenario, "close-before-pending") == 0)) {
        if (strcmp(scenario, "reset-failure") == 0) {
            Os_TimeTestCloseEvent();
        }
        native_refuse(Os_TargetAdvanceOneTick(1u, &ticket), E_OS_STATE);
        native_check(ticket == UINT64_C(777));
        if (strcmp(scenario, "close-before-pending") == 0) {
            native_check(xTaskGetTickCount() == 0u);
            Os_BackendRequestShutdown(E_OS_STATE);
        }
        Sleep(INFINITE);
        return 99u;
    }
    native_check(Os_TargetAdvanceOneTick(initial_epoch, &ticket) == E_OK);
    native_check(ticket == initial_epoch);
    Sleep(5u);
    {
        Os_TickCompletion value;
        native_check(Os_TargetTickCompletion(ticket, &value) == E_OK);
        native_check(value.kernel_tick == initial_kernel);
    }
    ticket = UINT64_C(777);
    native_refuse(Os_TargetAdvanceOneTick(initial_epoch + UINT64_C(2), &ticket), E_OS_VALUE);
    native_check(ticket == UINT64_C(777));
    for (i = 1u; i <= steps; ++i) {
        uint64_t epoch = initial_epoch + (uint64_t)i;
        Os_TickCompletion record = {0};
        native_check(Os_TargetAdvanceOneTick(epoch, &ticket) == E_OK);
        native_check(ticket == epoch);
        if (i == 1u && (strncmp(scenario, "tick-mask-", 10u) == 0 ||
                        strncmp(scenario, "tick-owner-", 11u) == 0)) {
            started_at = GetTickCount();
            while (InterlockedCompareExchange(&tick_mask_observed, 0, 0) == 0) {
                native_check((GetTickCount() - started_at) < 4000u);
                Sleep(1u);
            }
            native_check(InterlockedCompareExchange(&tick_mask_kernel, 0, 0) == 0);
            native_check(Os_TargetTickCompletion(ticket, &record) == E_OS_NOFUNC);
            printf("tick_masked observed=1 kernel=0 pending=1\n");
            InterlockedExchange(&release_owner, 1);
        }
        if (strcmp(scenario, "signal-failure") == 0) {
            /* The deliberately closed event has no active native waiter. */
            Sleep(INFINITE);
            return 99u;
        }
        if (strcmp(scenario, "inflight") == 0) {
            started_at = GetTickCount();
            while (InterlockedCompareExchange(&owner_held, 0, 0) == 0) {
                native_check((GetTickCount() - started_at) < 4000u);
                Sleep(1u);
            }
            {
                uint64_t unchanged = UINT64_C(777);
                record.ticket = UINT64_C(777);
                native_refuse(Os_TargetAdvanceOneTick(epoch + UINT64_C(1), &unchanged), E_OS_STATE);
                native_check(unchanged == UINT64_C(777));
                native_refuse(Os_TargetTickCompletion(ticket, &record), E_OS_NOFUNC);
                native_check(record.ticket == UINT64_C(777));
                (void)InterlockedExchange(&release_owner, 1);
            }
        }
        native_check(Os_TargetWaitTick(ticket, 4000u, &record) == E_OK);
        native_check((record.epoch == epoch) && (record.ticket == epoch));
        native_check(record.kernel_tick == (initial_kernel + (uint32_t)i));
        native_check(record.counter == (epoch % UINT64_C(65536)));
        native_check(record.alarm_actions == (UINT64_C(1) + (((epoch % 10u) == 0u) ? 1u : 0u)));
        native_check(record.action_errors == 0u);
        if (i <= 3u) {
            boundary[i - 1u] = record;
        }
        ++completed_count;
        native_check(Os_TargetAdvanceOneTick(epoch, &ticket) == E_OK);
        native_check(ticket == epoch);
    }
    ticket = UINT64_C(777);
    native_refuse(Os_TargetAdvanceOneTick(0u, &ticket),
                  (strcmp(scenario, "epoch-max") == 0) ? E_OS_LIMIT : E_OS_VALUE);
    native_check(ticket == UINT64_C(777));
    (void)InterlockedExchange(&producer_done, 1);
    Sleep(INFINITE);
    return 99u;
}
static void owner(void) {
    Os_TargetTrace('E');
    if (strcmp(scenario, "replace-tick-running") == 0) {
        vPortSetInterruptHandler(1u, service_interrupt);
        Os_TargetTrace('X');
        ShutdownOS(E_OS_STATE);
    }
    {
        uint64_t ticket = UINT64_C(777);
        uint64_t epoch = UINT64_C(888);
        Os_TickCompletion record = {0};
        record.ticket = UINT64_C(777);
        check(Os_TargetAdvanceOneTick(1u, &ticket) == E_OS_CALLEVEL);
        check(ticket == UINT64_C(777));
        check(Os_TargetTickCompletion(ticket, &record) == E_OS_CALLEVEL);
        check(record.ticket == UINT64_C(777));
        check(Os_TargetWaitTick(ticket, 1u, &record) == E_OS_CALLEVEL);
        check(record.ticket == UINT64_C(777));
        check(Os_TargetCurrentTick(&ticket, &epoch) == E_OS_NOFUNC);
        check((ticket == UINT64_C(777)) && (epoch == UINT64_C(888)));
        check(Os_TargetCompleteTick(ticket) == E_OS_STATE);
        check(Os_TargetCurrentTick(NULL, &epoch) == E_OS_ILLEGAL_ADDRESS);
        check(Os_TargetWaitTick(ticket, 0u, &record) == E_OS_VALUE);
        context_rejects = 7u;
    }
    if (strcmp(scenario, "tick-owner-all") == 0) {
        SuspendAllInterrupts();
    } else if (strcmp(scenario, "tick-owner-os") == 0) {
        SuspendOSInterrupts();
    }
    if (strcmp(scenario, "tick-mask-all") != 0 && strcmp(scenario, "tick-mask-os") != 0) {
        (void)InterlockedExchange(&owner_entered, 1);
    }
    if (strncmp(scenario, "tick-owner-", 11u) == 0) {
        DWORD started_at = GetTickCount();
        while (InterlockedCompareExchange(&release_owner, 0, 0) == 0) {
            check((GetTickCount() - started_at) < 4000u);
            Sleep(1u);
        }
        if (strcmp(scenario, "tick-owner-all") == 0) {
            ResumeAllInterrupts();
        } else {
            ResumeOSInterrupts();
        }
    }
    if (manual) {
        manual_alarms();
    }
    for (;;) {
        EventMaskType events;
        uint64_t ticket;
        uint64_t epoch;
        TickType counter;
        check(WaitEvent(7u) == E_OK);
        check(GetEvent(0u, &events) == E_OK);
        check(ClearEvent(events) == E_OK);
        check(Os_TargetCurrentTick(&ticket, &epoch) == E_OK);
        check(GetCounterValue(0u, &counter) == E_OK);
        check(counter == (epoch % UINT64_C(65536)));
        if (strncmp(scenario, "hardware-", 9u) == 0) {
            TickType value;
            TickType elapsed;
            check(hardware_count < 20u);
            check(GetCounterValue(1u, &value) == E_OK);
            check(GetElapsedValue(1u, &hardware_previous, &elapsed) == E_OK);
            check((value == hardware_previous) && (elapsed == 1u));
            hardware[hardware_count++] =
                (HardwareRecord){epoch, (uint32_t)xTaskGetTickCount(), value, elapsed};
        }
        if ((events & 1u) != 0u) {
            ++work_count;
        }
        if ((events & 2u) != 0u) {
            ++app_count;
        }
        if (strcmp(scenario, "inflight") == 0) {
            DWORD started_at = GetTickCount();
            (void)InterlockedExchange(&owner_held, 1);
            while (InterlockedCompareExchange(&release_owner, 0, 0) == 0) {
                check((GetTickCount() - started_at) < 4000u);
                Sleep(1u);
            }
        }
        if (strcmp(scenario, "signal-failure") == 0) {
            Os_TimeTestCloseEvent();
        }
        check(Os_TargetCompleteTick(ticket) == E_OK);
        /* The next real Wait publishes completion after suspended membership. */
    }
}
static void monitor(void) {
    DWORD started_at = GetTickCount();
    if (strcmp(scenario, "tick-mask-all") == 0 || strcmp(scenario, "tick-mask-os") == 0) {
        /* The real owner is already WAITING when this lower-priority Task
         * runs. Hold its tick pending using this Task's own balanced mask. */
        if (strcmp(scenario, "tick-mask-all") == 0) {
            SuspendAllInterrupts();
        } else {
            SuspendOSInterrupts();
        }
        (void)InterlockedExchange(&owner_entered, 1);
        while (InterlockedCompareExchange(&release_owner, 0, 0) == 0) {
            check((GetTickCount() - started_at) < 4000u);
            Sleep(1u);
        }
        if (strcmp(scenario, "tick-mask-all") == 0) {
            ResumeAllInterrupts();
        } else {
            ResumeOSInterrupts();
        }
    }
    while (InterlockedCompareExchange(&producer_done, 0, 0) == 0) {
        check((GetTickCount() - started_at) < 8000u);
        Sleep(1u);
    }
    {
        Os_ActivationInfo value;
        check(Os_TargetInspectActivation(0u, &value) == E_OK);
        check((value.state == (manual ? SUSPENDED : WAITING)) &&
              (value.count == (manual ? 0u : 1u)) && (eTaskGetState(owner_handle) == eSuspended));
    }
    check(work_count == steps);
    check(app_count == (unsigned)(((initial_epoch + steps) / 10u) - (initial_epoch / 10u)));
    Os_TargetTrace('M');
    ShutdownOS(E_OK);
}
static void alarm_task(void) {
    ++alarm_entries;
    Os_TargetTrace('H');
    (void)TerminateTask();
    Os_TargetTrace('X');
    ShutdownOS(E_OS_STATE);
}
void StartupHook(void) {
    owner_handle = xTaskGetHandle("E");
    vPortSetInterruptHandler(31u, service_interrupt);
    if (strcmp(scenario, "absolute-wide") == 0) {
        Os_TimeTestSeed(0u, 0u, UINT32_MAX);
    }
    if (initial_epoch != 0u) {
        Os_TimeTestSeed(initial_epoch, initial_kernel, initial_epoch % UINT64_C(65536));
    }
    if (!manual) {
        HANDLE handle = CreateThread(NULL, 0u, requester, NULL, 0u, NULL);
        check(handle != NULL);
        check(CloseHandle(handle) != 0);
    }
    Os_TargetTrace('S');
    if (strcmp(scenario, "replace-tick-startup") == 0) {
        vPortSetInterruptHandler(1u, service_interrupt);
        Os_TargetTrace('X');
        ShutdownOS(E_OS_STATE);
    }
}
void ShutdownHook(StatusType Error) {
    unsigned i;
    if (strncmp(scenario, "tick-owner-", 11u) == 0) {
        printf("dispatch_release phase=0 waiting_commit=%ld\n",
               InterlockedCompareExchange(&dispatch_release_stage, 0, 0) == 2 ? 1L : 0L);
    }
    for (i = 0u; i < hardware_count; ++i) {
        printf("hardware epoch=%llu kernel=%u value=%llu elapsed=%llu\n",
               (unsigned long long)hardware[i].epoch, (unsigned)hardware[i].kernel,
               (unsigned long long)hardware[i].value, (unsigned long long)hardware[i].elapsed);
    }
    Os_TargetTrace('Z');
    if (printf("time ticks=%u work=%u app=%u rejects=%u error=%u\n", completed_count, work_count,
               app_count, native_rejects, Error) < 0) {
        ExitProcess(99u);
    }
    if (printf("alarm entries=%u callbacks=%u errors=%u last=%u rejected=%u counter=%llu\n",
               alarm_entries, callbacks, error_hooks, last_error, standard_rejects,
               (unsigned long long)final_counter) < 0) {
        ExitProcess(99u);
    }
    if (printf("context rejected=%u\n", context_rejects) < 0) {
        ExitProcess(99u);
    }
    for (i = 0u; (i < steps) && (i < 3u); ++i) {
        if (printf("TICK epoch=%llu kernel=%u counter=%llu actions=%llu\n",
                   (unsigned long long)boundary[i].epoch, boundary[i].kernel_tick,
                   (unsigned long long)boundary[i].counter,
                   (unsigned long long)boundary[i].alarm_actions) < 0) {
            ExitProcess(99u);
        }
    }
}
int main(int argc, char **argv) {
    Os_TaskConfig tasks[] = {
        {0u, "E", owner, 2u, 1u, OS_EXTENDED_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {1u, "M", monitor, 1u, 1u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u},
        {2u, "H", alarm_task, 3u, 0u, OS_BASIC_TASK, 1u, OS_SCHEDULE_FULL, 0u}};
    Os_CounterConfig counters[] = {{0u, 65535u, 1u, 1u, 1u}, {1u, 15u, 1u, 1u, 0u}};
    Os_AlarmConfig alarms[] = {{0u, 0u, OS_ALARM_EVENT, 0u, 1u, NULL, 1u, 0u, 1u, 1u, 0u},
                               {1u, 0u, OS_ALARM_EVENT, 0u, 2u, NULL, 1u, 0u, 10u, 10u, 0u}};
    Os_TimeConfig time = {counters, 1u, alarms, 2u, 0u, 0u, 4u, NULL, NULL, 0u};
    Os_TargetConfig config = {tasks, 3u, 262144u, NULL,  0u,   NULL, 0u,
                              0u,    0u, 0u,      &time, NULL, NULL};
    scenario = (argc == 2) ? argv[1] : "thousand";
    steps = (strcmp(scenario, "thousand") == 0) ? 1000u : 1u;
    if (strncmp(scenario, "hardware-", 9u) == 0) {
        steps = 20u;
        time.counter_count = 2u;
    }
    manual = (strncmp(scenario, "absolute-", 9u) == 0) ||
             (strcmp(scenario, "relative-wrap") == 0) || (strcmp(scenario, "action-error") == 0) ||
             (strcmp(scenario, "callback") == 0) || (strcmp(scenario, "counter-errors") == 0) ||
             (strcmp(scenario, "category1-time") == 0) || (strcmp(scenario, "category2-time") == 0);
    if (strcmp(scenario, "category1-time") == 0) {
        config.category1_isrs = UINT32_C(1) << 31u;
    }
    if (manual) {
        steps = 0u;
        counters[0].maximum = (strcmp(scenario, "absolute-wide") == 0) ? UINT32_MAX : 15u;
        time.counter_count = 2u;
        alarms[0].autostart_modes = 0u;
        alarms[1].autostart_modes = 0u;
        if ((strcmp(scenario, "relative-wrap") == 0) || (strcmp(scenario, "action-error") == 0)) {
            alarms[0].action = OS_ALARM_ACTIVATE;
            alarms[0].task = 2u;
        } else if ((strcmp(scenario, "absolute-current") == 0) ||
                   (strcmp(scenario, "absolute-wide") == 0) ||
                   (strcmp(scenario, "callback") == 0)) {
            alarms[0].action = OS_ALARM_CALLBACK;
            alarms[0].callback = alarm_callback;
        } else {
            /* Event and refusal vectors keep the declared SetEvent action. */
        }
        if (strcmp(scenario, "action-error") == 0) {
            tasks[2].priority = 1u;
            time.error_hook = alarm_error;
        }
    }
    if ((strcmp(scenario, "wrap") == 0) || (strcmp(scenario, "hardware-kernel-wrap") == 0)) {
        initial_epoch = UINT64_C(65534);
        initial_kernel = UINT32_MAX - 2u;
        alarms[1].start = 6u;
        steps = 3u;
    } else if (strcmp(scenario, "epoch-max") == 0) {
        initial_epoch = UINT64_MAX - UINT64_C(1);
        alarms[1].start = 6u;
    } else {
        /* Other vectors use the original reference timing phase. */
    }
    if (strcmp(scenario, "config-counter-null") == 0) {
        time.counters = NULL;
    } else if (strcmp(scenario, "config-counter-capacity") == 0) {
        time.counter_count = 9u;
    } else if (strcmp(scenario, "config-counter-duplicate") == 0) {
        time.counter_count = 2u;
        counters[1].id = 0u;
    } else if (strcmp(scenario, "config-counter-maximum") == 0) {
        counters[0].maximum = UINT64_MAX;
    } else if (strcmp(scenario, "config-counter-base") == 0) {
        counters[0].ticks_per_base = 0u;
    } else if (strcmp(scenario, "config-counter-cycle") == 0) {
        counters[0].minimum_cycle = 0u;
    } else if (strcmp(scenario, "config-system-missing") == 0) {
        time.system_counter = 7u;
    } else if (strcmp(scenario, "config-system-hardware") == 0) {
        counters[0].software = 0u;
    } else if (strcmp(scenario, "config-alarm-null") == 0) {
        time.alarms = NULL;
    } else if (strcmp(scenario, "config-alarm-capacity") == 0) {
        time.alarm_count = 17u;
    } else if (strcmp(scenario, "config-alarm-duplicate") == 0) {
        alarms[1].id = 0u;
    } else if (strcmp(scenario, "config-alarm-counter") == 0) {
        alarms[0].counter = 7u;
    } else if (strcmp(scenario, "config-alarm-action") == 0) {
        alarms[0].action = 3u;
    } else if (strcmp(scenario, "config-alarm-task") == 0) {
        alarms[0].task = 15u;
    } else if (strcmp(scenario, "config-alarm-basic") == 0) {
        alarms[0].task = 1u;
    } else if (strcmp(scenario, "config-alarm-callback") == 0) {
        alarms[0].action = OS_ALARM_CALLBACK;
    } else if (strcmp(scenario, "config-alarm-cycle") == 0) {
        alarms[0].cycle = 70000u;
    } else if (strcmp(scenario, "config-owner-basic") == 0) {
        time.owner = 1u;
    } else if (strcmp(scenario, "config-owner-inactive") == 0) {
        tasks[0].autostart_modes = 0u;
    } else if (strcmp(scenario, "config-wake") == 0) {
        time.wake_event = 0u;
    } else if (strcmp(scenario, "config-tick-category1") == 0) {
        config.category1_isrs = 2u;
    } else {
        /* Behavior vectors retain their valid static time configuration. */
    }
    if (strncmp(scenario, "config-", 7u) == 0) {
        StatusType expected = ((strcmp(scenario, "config-counter-duplicate") == 0) ||
                               (strcmp(scenario, "config-system-missing") == 0) ||
                               (strcmp(scenario, "config-alarm-duplicate") == 0) ||
                               (strcmp(scenario, "config-alarm-counter") == 0))
                                  ? E_OS_ID
                                  : E_OS_VALUE;
        StatusType status = Os_TargetPrepare(&config);
        if (printf("prepare=%u\n", status) < 0) {
            return 99;
        }
        return (status == expected) ? 0 : 99;
    }
    check(Os_TargetPrepare(&config) == E_OK);
    {
        uint64_t unchanged = UINT64_C(777);
        check(Os_TargetAdvanceOneTick(0u, &unchanged) == E_OS_STATE);
        check(unchanged == UINT64_C(777));
    }
    StartOS(1u);
    return 99;
}
