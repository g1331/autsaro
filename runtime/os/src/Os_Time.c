#include "Os_Backend.h"
#include "Os_IntegrationHooks.h"

static TickType values[OS_MAX_COUNTERS];
const TickType *const Os_ArtiCounters = values;
typedef Os_ArtiAlarmState AlarmState;
static AlarmState alarms[OS_MAX_ALARMS];
const Os_ArtiAlarmState *const Os_ArtiAlarms = alarms;
static volatile Os_Atomic32 tick_state;
static uint64_t requested_epoch;
static uint64_t confirmed_epoch;
static uint64_t action_count;
static uint64_t error_count;
static Os_TickCompletion completion;
static HANDLE completion_event;
static volatile Os_Atomic32 signal_failed;
#define TIME_CLOSED 256L
#define TIME_RESERVED 1L
#define TIME_PENDING 2L
#define TIME_PROCESSING 3L
#define TIME_DELIVERED 4L
#define TIME_MARKED 5L
#define TIME_COMPLETE 6L
#define TIME_PUBLISHING 7L

static size_t counter_index(CounterType id) {
    const Os_TimeConfig *config = Os_Config->time;
    if (config != NULL) {
        for (size_t i = 0u; i < config->counter_count; ++i) {
            if (config->counters[i].id == id) {
                return i;
            }
        }
    }
    return OS_MAX_COUNTERS;
}
static size_t alarm_index(AlarmType id) {
    const Os_TimeConfig *config = Os_Config->time;
    if (config != NULL) {
        for (size_t i = 0u; i < config->alarm_count; ++i) {
            if (config->alarms[i].id == id) {
                return i;
            }
        }
    }
    return OS_MAX_ALARMS;
}
static const Os_TaskConfig *configured_task(const Os_TargetConfig *config, TaskType id) {
    size_t i;
    const Os_TaskConfig *result = NULL;
    for (i = 0u; i < config->task_count; ++i) {
        if (config->tasks[i].id == id) {
            result = &config->tasks[i];
            break;
        }
    }
    return result;
}
static int valid_cycle(const Os_CounterConfig *counter, TickType cycle) {
    return (cycle == 0u) || ((cycle >= counter->minimum_cycle) && (cycle <= counter->maximum));
}
StatusType Os_TimeValidate(const Os_TargetConfig *target) {
    const Os_TimeConfig *config = target->time;
    const Os_TaskConfig *owner;
    size_t i;
    size_t j;
    int system_found = 0;
    uint8_t increments[OS_MAX_COUNTERS][OS_MAX_COUNTERS] = {{0}};
    if (config == NULL) {
        return E_OK;
    }
    owner = configured_task(target, config->owner);
    if ((config->counter_count == 0u) || (config->counter_count > OS_MAX_COUNTERS) ||
        (config->counters == NULL) || (config->alarm_count > OS_MAX_ALARMS) ||
        ((config->alarm_count != 0u) && (config->alarms == NULL)) ||
        ((config->wake_event != 0u) &&
         ((owner == NULL) || (owner->kind != OS_EXTENDED_TASK) || (owner->autostart_modes == 0u) ||
          ((target->category1_isrs & 2u) != 0u))) ||
        ((config->wake_event == 0u) && (config->owner != INVALID_TASK))) {
        return E_OS_VALUE;
    }
    for (i = 0u; i < config->counter_count; ++i) {
        const Os_CounterConfig *counter = &config->counters[i];
        if ((counter->id >= OS_MAX_COUNTERS) || (counter->maximum == 0u) ||
            (counter->maximum > UINT32_MAX) || (counter->ticks_per_base == 0u) ||
            (counter->minimum_cycle == 0u) || (counter->minimum_cycle > counter->maximum) ||
            (counter->software > 1u) || ((config->wake_event == 0u) && (counter->software == 0u))) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < i; ++j) {
            if (counter->id == config->counters[j].id) {
                return E_OS_ID;
            }
        }
        if (counter->id == config->system_counter) {
            if (counter->software == 0u) {
                return E_OS_VALUE;
            }
            system_found = 1;
        }
    }
    if (system_found == 0) {
        return E_OS_ID;
    }
    for (i = 0u; i < config->alarm_count; ++i) {
        const Os_AlarmConfig *alarm = &config->alarms[i];
        const Os_TaskConfig *task = configured_task(target, alarm->task);
        const Os_CounterConfig *counter = NULL;
        for (j = 0u; j < config->counter_count; ++j) {
            if (config->counters[j].id == alarm->counter) {
                counter = &config->counters[j];
            }
        }
        if ((alarm->id >= OS_MAX_ALARMS) || (alarm->action > OS_ALARM_INCREMENT_COUNTER) ||
            (alarm->autostart_modes > 3u) || (alarm->absolute > 1u)) {
            return E_OS_VALUE;
        }
        if (counter == NULL) {
            return E_OS_ID;
        }
        if (((alarm->action <= OS_ALARM_EVENT) && (task == NULL)) ||
            ((alarm->action == OS_ALARM_EVENT) &&
             ((task->kind != OS_EXTENDED_TASK) || (alarm->event == 0u))) ||
            ((alarm->action == OS_ALARM_CALLBACK) && (alarm->callback == NULL))) {
            return E_OS_VALUE;
        }
        if (alarm->action == OS_ALARM_INCREMENT_COUNTER) {
            size_t from = OS_MAX_COUNTERS;
            size_t to = OS_MAX_COUNTERS;
            for (j = 0u; j < config->counter_count; ++j) {
                if (config->counters[j].id == alarm->counter) {
                    from = j;
                }
                if ((config->counters[j].id == alarm->increment_counter) &&
                    (config->counters[j].software != 0u)) {
                    to = j;
                }
            }
            if ((from == OS_MAX_COUNTERS) || (to == OS_MAX_COUNTERS)) {
                return E_OS_ID;
            }
            increments[from][to] = 1u;
        }
        if ((alarm->autostart_modes != 0u) && ((alarm->start > counter->maximum) ||
                                               ((alarm->absolute == 0u) && (alarm->start == 0u)) ||
                                               (valid_cycle(counter, alarm->cycle) == 0))) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < i; ++j) {
            if (alarm->id == config->alarms[j].id) {
                return E_OS_ID;
            }
        }
    }
    /* An increment action graph must not admit recursive counter cycles. */
    for (size_t k = 0u; k < config->counter_count; ++k) {
        for (i = 0u; i < config->counter_count; ++i) {
            for (j = 0u; j < config->counter_count; ++j) {
                if ((increments[i][k] != 0u) && (increments[k][j] != 0u)) {
                    increments[i][j] = 1u;
                }
            }
        }
    }
    for (i = 0u; i < config->counter_count; ++i) {
        if (increments[i][i] != 0u) {
            return E_OS_VALUE;
        }
    }
    return Os_ScheduleValidate(target);
}
static TickType absolute_distance(size_t counter, TickType start) {
    TickType current = values[counter];
    TickType modulus = Os_Config->time->counters[counter].maximum + UINT64_C(1);
    return (start > current) ? (start - current) : (modulus - (current - start));
}
void Os_TimeInit(AppModeType mode) {
    const Os_TimeConfig *config = Os_Config->time;
    if ((config != NULL) && (config->wake_event != 0u)) {
        const Os_TaskConfig *owner = configured_task(Os_Config, config->owner);
        if ((owner->autostart_modes & mode) == 0u) {
            Os_BackendShutdown(E_OS_VALUE);
        }
        completion_event = Os_PortEvent(NULL, TRUE, FALSE, NULL);
    }
}
void Os_TimeAutostart(AppModeType mode) {
    const Os_TimeConfig *config = Os_Config->time;
    if (config != NULL) {
        for (size_t i = 0u; i < config->alarm_count; ++i) {
            const Os_AlarmConfig *alarm = &config->alarms[i];
            if ((alarm->autostart_modes & mode) != 0u) {
                size_t counter = counter_index(alarm->counter);
                alarms[i].active = 1u;
                alarms[i].cycle = alarm->cycle;
                alarms[i].remaining = (alarm->absolute != 0u)
                                          ? absolute_distance(counter, alarm->start)
                                          : alarm->start;
            }
        }
    }
}
void Os_TimeClose(void) {
    (void)InterlockedOr(&tick_state, TIME_CLOSED);
    if ((completion_event != NULL) && (Os_HostSetEvent(completion_event) == 0)) {
        (void)InterlockedExchange(&signal_failed, 1);
    }
}
int Os_TimeSignalFailed(void) { return InterlockedCompareExchange(&signal_failed, 0, 0) != 0; }
void Os_TimeReportAction(StatusType status) {
    if (action_count == UINT64_MAX) {
        Os_BackendShutdown(E_OS_STATE);
    }
    ++action_count;
    if (status != E_OK) {
        if (error_count == UINT64_MAX) {
            Os_BackendShutdown(E_OS_STATE);
        }
        ++error_count;
        if ((Os_Config->time->error_hook != NULL) && (Os_ErrorHookConfigured() == 0)) {
            Os_Config->time->error_hook(status);
        }
    }
}
static void action(size_t index) {
    const Os_AlarmConfig *alarm = &Os_Config->time->alarms[index];
    StatusType status = E_OK;
    Os_ArtiInternalEnter();
    switch (alarm->action) {
    case OS_ALARM_ACTIVATE:
        status = ActivateTask(alarm->task);
        break;
    case OS_ALARM_EVENT:
        status = SetEvent(alarm->task, alarm->event);
        break;
    case OS_ALARM_CALLBACK:
        Os_HookInvoke(alarm->callback, OS_HOOK_ALARM);
        break;
    case OS_ALARM_INCREMENT_COUNTER:
        /* Counter actions are handled by the bounded iterative work stack. */
        Os_BackendShutdown(E_OS_STATE);
        break;
    default:
        Os_BackendShutdown(E_OS_STATE);
        break;
    }
    Os_ArtiInternalLeave();
    Os_TimeReportAction(status);
}
TickType Os_TimeCounterValue(CounterType counter) { return values[counter_index(counter)]; }
static void increment(size_t counter) {
    typedef struct {
        size_t counter;
        size_t alarm;
        uint8_t report;
    } CounterWork;
    CounterWork work[OS_MAX_COUNTERS];
    size_t depth = 1u;
    const Os_TimeConfig *config = Os_Config->time;
    work[0] = (CounterWork){counter, 0u, 0u};
    values[counter] = (values[counter] == config->counters[counter].maximum)
                          ? 0u
                          : (values[counter] + UINT64_C(1));
    while (depth != 0u) {
        CounterWork *current = &work[depth - 1u];
        if (current->alarm < config->alarm_count) {
            size_t i = current->alarm;
            ++current->alarm;
            if ((config->alarms[i].counter == config->counters[current->counter].id) &&
                (alarms[i].active != 0u)) {
                configASSERT(alarms[i].remaining != 0u);
                --alarms[i].remaining;
                if (alarms[i].remaining == 0u) {
                    alarms[i].remaining = alarms[i].cycle;
                    alarms[i].active = (alarms[i].cycle == 0u) ? 0u : 1u;
                    if (config->alarms[i].action == OS_ALARM_INCREMENT_COUNTER) {
                        size_t child = counter_index(config->alarms[i].increment_counter);
                        configASSERT(depth < OS_MAX_COUNTERS);
                        work[depth] = (CounterWork){child, 0u, 1u};
                        ++depth;
                        values[child] = (values[child] == config->counters[child].maximum)
                                            ? 0u
                                            : values[child] + UINT64_C(1);
                    } else {
                        action(i);
                    }
                }
            }
        } else {
            uint8_t report = current->report;
            Os_ScheduleIncrement(config->counters[current->counter].id);
            --depth;
            if (report != 0u) {
                Os_TimeReportAction(E_OK);
            }
        }
    }
}
static StatusType implementation_IncrementCounter(CounterType CounterID) {
    size_t index;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = counter_index(CounterID);
    if ((index == OS_MAX_COUNTERS) || (Os_Config->time->counters[index].software == 0u)) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    increment(index);
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}
static StatusType implementation_GetCounterValue(CounterType CounterID, TickRefType Value) {
    size_t index;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Value == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = counter_index(CounterID);
    if (index == OS_MAX_COUNTERS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    *Value = values[index];
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}
static StatusType implementation_GetElapsedValue(CounterType CounterID, TickRefType Value,
                                                 TickRefType ElapsedValue) {
    size_t index;
    StatusType status = E_OK;
    Os_StackCheck();
    Os_BackendGuardService();
    if ((Value == NULL) || (ElapsedValue == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = counter_index(CounterID);
    if (index == OS_MAX_COUNTERS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (*Value > Os_Config->time->counters[index].maximum) {
        status = E_OS_VALUE;
    } else {
        *ElapsedValue =
            (values[index] >= *Value)
                ? (values[index] - *Value)
                : (Os_Config->time->counters[index].maximum - *Value + UINT64_C(1) + values[index]);
        *Value = values[index];
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
static StatusType implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info) {
    size_t index;
    const Os_CounterConfig *counter;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Info == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = alarm_index(AlarmID);
    if (index == OS_MAX_ALARMS) {
        return E_OS_ID;
    }
    counter = &Os_Config->time->counters[counter_index(Os_Config->time->alarms[index].counter)];
    Info->maxallowedvalue = counter->maximum;
    Info->ticksperbase = counter->ticks_per_base;
    Info->mincycle = counter->minimum_cycle;
    Os_BackendGuardService();
    return E_OK;
}
static StatusType implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick) {
    size_t index;
    StatusType status = E_OK;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Tick == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = alarm_index(AlarmID);
    if (index == OS_MAX_ALARMS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (alarms[index].active == 0u) {
        status = E_OS_NOFUNC;
    } else {
        *Tick = alarms[index].remaining;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
static StatusType set_alarm(AlarmType id, TickType start, TickType cycle, int absolute) {
    size_t index;
    size_t counter;
    StatusType status = E_OK;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = alarm_index(id);
    if (index == OS_MAX_ALARMS) {
        return E_OS_ID;
    }
    counter = counter_index(Os_Config->time->alarms[index].counter);
    if ((start > Os_Config->time->counters[counter].maximum) ||
        ((absolute == 0) && (start == 0u)) ||
        (valid_cycle(&Os_Config->time->counters[counter], cycle) == 0)) {
        return E_OS_VALUE;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (alarms[index].active != 0u) {
        status = E_OS_STATE;
    } else {
        alarms[index].remaining = (absolute != 0) ? absolute_distance(counter, start) : start;
        alarms[index].cycle = cycle;
        alarms[index].active = 1u;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
static StatusType implementation_SetRelAlarm(AlarmType AlarmID, TickType Increment,
                                             TickType Cycle) {
    return set_alarm(AlarmID, Increment, Cycle, 0);
}
static StatusType implementation_SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle) {
    return set_alarm(AlarmID, Start, Cycle, 1);
}
static StatusType implementation_CancelAlarm(AlarmType AlarmID) {
    size_t index;
    StatusType status = E_OK;
    Os_StackCheck();
    Os_BackendGuardService();
    if (Os_BackendServiceContext() == 0) {
        return E_OS_CALLEVEL;
    }
    index = alarm_index(AlarmID);
    if (index == OS_MAX_ALARMS) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (alarms[index].active == 0u) {
        status = E_OS_NOFUNC;
    } else {
        alarms[index].active = 0u;
        alarms[index].remaining = 0u;
        alarms[index].cycle = 0u;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
int Os_TimeBeginTick(void) {
    if (Os_Config->time == NULL) {
        return Os_TargetReady();
    }
    if (Os_Config->time->wake_event == 0u) {
        return 0;
    }
    return InterlockedCompareExchange(&tick_state, TIME_PROCESSING, TIME_PENDING) == TIME_PENDING;
}
void Os_TimeTick(void) {
    const Os_TimeConfig *config = Os_Config->time;
    if ((config != NULL) && (config->wake_event != 0u)) {
        if (InterlockedCompareExchange(&tick_state, 0, 0) != TIME_PROCESSING) {
            Os_BackendShutdown(E_OS_STATE);
        }
        action_count = 0u;
        error_count = 0u;
        /* Host timer counters are registers driven by the actual controlled
         * kernel tick ISR. Standard IncrementCounter cannot write them. */
        for (size_t i = 0u; i < config->counter_count; ++i) {
            if (config->counters[i].software == 0u) {
                increment(i);
            }
        }
        increment(counter_index(config->system_counter));
        Os_ArtiInternalEnter();
        const StatusType status = SetEvent(config->owner, config->wake_event);
        Os_ArtiInternalLeave();
        if (status != E_OK) {
            Os_BackendShutdown(E_OS_STATE);
        }
        (void)InterlockedCompareExchange(&tick_state, TIME_DELIVERED, TIME_PROCESSING);
    }
}
void vApplicationTickHook(void) { Os_TimeTick(); }
static StatusType native_time_context(void) {
    if (Os_BridgeActorAllowed() == 0) {
        return E_OS_CALLEVEL;
    }
    if ((Os_TargetReady() == 0) || (Os_Config->time == NULL) ||
        (Os_Config->time->wake_event == 0u)) {
        return E_OS_STATE;
    }
    return Os_BridgeContext();
}
StatusType Os_TargetAdvanceOneTick(uint64_t epoch, uint64_t *ticket) {
    StatusType status;
    LONG state;
    if (ticket == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_time_context();
    if (status != E_OK) {
        return status;
    }
    state = InterlockedCompareExchange(&tick_state, 0, 0);
    if ((state != 0) && (state != TIME_COMPLETE)) {
        return E_OS_STATE;
    }
    if (epoch == confirmed_epoch) {
        *ticket = completion.ticket;
        return E_OK;
    }
    if (confirmed_epoch == UINT64_MAX) {
        return E_OS_LIMIT;
    }
    if (epoch != (confirmed_epoch + UINT64_C(1))) {
        return E_OS_VALUE;
    }
    if (InterlockedCompareExchange(&tick_state, TIME_RESERVED, state) != state) {
        return E_OS_STATE;
    }
    if (ResetEvent(completion_event) == 0) {
        (void)InterlockedExchange(&signal_failed, 1);
        Os_BackendRequestShutdown(E_OS_STATE);
        return E_OS_STATE;
    }
    requested_epoch = epoch;
#ifdef OS_TIME_TESTS
    Os_TimeTestBeforePending();
#endif
    if (InterlockedCompareExchange(&tick_state, TIME_PENDING, TIME_RESERVED) != TIME_RESERVED) {
        return E_OS_STATE;
    }
    *ticket = epoch;
    Os_PortPostInterrupt(1u);
    return E_OK;
}
StatusType Os_TargetTickCompletion(uint64_t ticket, Os_TickCompletion *record) {
    StatusType status;
    LONG state;
    if (record == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_time_context();
    if (status != E_OK) {
        return status;
    }
    state = InterlockedCompareExchange(&tick_state, 0, 0);
    if ((state & TIME_CLOSED) != 0) {
        return E_OS_STATE;
    }
    if ((state != 0) && (state != TIME_COMPLETE)) {
        return E_OS_NOFUNC;
    }
    if (ticket != completion.ticket) {
        return E_OS_ID;
    }
    *record = completion;
    return E_OK;
}
StatusType Os_TargetWaitTick(uint64_t ticket, uint32_t timeout_ms, Os_TickCompletion *record) {
    DWORD started_at;
    if ((timeout_ms == 0u) || (timeout_ms > 5000u)) {
        return E_OS_VALUE;
    }
    started_at = GetTickCount();
    for (;;) {
        DWORD elapsed;
        DWORD waited;
        StatusType status = Os_TargetTickCompletion(ticket, record);
        if (status != E_OS_NOFUNC) {
            return status;
        }
        elapsed = GetTickCount() - started_at;
        if (elapsed >= timeout_ms) {
            return E_OS_NOFUNC;
        }
        waited = WaitForSingleObject(completion_event, timeout_ms - elapsed);
        if (waited == WAIT_TIMEOUT) {
            /* A simultaneous close wins over a timeout report. */
            status = Os_TargetTickCompletion(ticket, record);
            return ((status == E_OS_NOFUNC) && (Os_TargetReady() == 0)) ? E_OS_STATE : status;
        }
        if (waited != WAIT_OBJECT_0) {
            (void)InterlockedExchange(&signal_failed, 1);
            Os_BackendRequestShutdown(E_OS_STATE);
            return E_OS_STATE;
        }
    }
}
static StatusType time_owner(void) {
    int owner;
    Os_StackCheck();
    Os_BackendGuardService();
    if ((Os_TargetReady() == 0) || (Os_Config->time == NULL) ||
        (Os_Config->time->wake_event == 0u)) {
        return E_OS_STATE;
    }
    owner = Os_BackendTaskOwner(Os_Config->time->owner);
    return (owner == 0) ? E_OS_CALLEVEL : ((owner < 0) ? E_OS_ACCESS : E_OK);
}
StatusType Os_TargetCurrentTick(uint64_t *ticket, uint64_t *epoch) {
    StatusType status;
    LONG state;
    if ((ticket == NULL) || (epoch == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = time_owner();
    if (status != E_OK) {
        return status;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    state = InterlockedCompareExchange(&tick_state, 0, 0);
    if ((state == TIME_DELIVERED) || (state == TIME_MARKED)) {
        *ticket = requested_epoch;
        *epoch = requested_epoch;
    } else {
        status = ((state & TIME_CLOSED) != 0) ? E_OS_STATE : E_OS_NOFUNC;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
StatusType Os_TargetCompleteTick(uint64_t ticket) {
    StatusType status = time_owner();
    LONG state;
    if (status != E_OK) {
        return status;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    state = InterlockedCompareExchange(&tick_state, 0, 0);
    if ((state != TIME_DELIVERED) && (state != TIME_MARKED)) {
        status = E_OS_STATE;
    } else if (ticket != requested_epoch) {
        status = E_OS_ID;
    } else if (state == TIME_DELIVERED) {
        if (InterlockedCompareExchange(&tick_state, TIME_MARKED, TIME_DELIVERED) !=
            TIME_DELIVERED) {
            status = E_OS_STATE;
        }
    } else {
        /* Marking is idempotent until the real waiting/empty publication. */
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}
void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate) {
    const Os_TimeConfig *config = Os_Config->time;
#ifdef OS_TIME_TESTS
    Os_TimeTestOnWaiting();
#endif
    Os_IntegrationOnWaiting(id, pending, predicate);
    if ((config != NULL) && (id == config->owner) && (pending == 0u) &&
        ((predicate & config->wake_event) != 0u) && (Os_MailboxQuiescent() != 0) &&
        (InterlockedCompareExchange(&tick_state, TIME_PUBLISHING, TIME_MARKED) == TIME_MARKED)) {
        completion.epoch = requested_epoch;
        completion.ticket = requested_epoch;
        completion.kernel_tick = (uint32_t)xTaskGetTickCount();
        completion.counter = values[counter_index(config->system_counter)];
        completion.alarm_actions = action_count;
        completion.action_errors = error_count;
        if (Os_HostSetEvent(completion_event) == 0) {
            (void)InterlockedExchange(&signal_failed, 1);
            Os_BackendRequestShutdown(E_OS_STATE);
        } else {
            /* The manual event remains signalled if the native reader wakes
             * before this release publication. It cannot lose the notification.
             * Only the same bridge producer resets it while reserving a new tick. */
            confirmed_epoch = requested_epoch;
            (void)InterlockedCompareExchange(&tick_state, TIME_COMPLETE, TIME_PUBLISHING);
        }
    }
}
#ifdef OS_TIME_TESTS
void Os_TimeTestCloseEvent(void) {
    configASSERT(completion_event != NULL);
    configASSERT(CloseHandle(completion_event) != 0);
}
void Os_TimeTestSeed(uint64_t epoch, uint32_t kernel_tick, TickType value) {
    void vTaskOsTestTick(TickType_t tick);
    configASSERT(Os_Config->time != NULL);
    configASSERT(value <=
                 Os_Config->time->counters[counter_index(Os_Config->time->system_counter)].maximum);
    configASSERT(InterlockedCompareExchange(&tick_state, 0, 0) == 0);
    vTaskOsTestTick((TickType_t)kernel_tick);
    values[counter_index(Os_Config->time->system_counter)] = value;
    confirmed_epoch = epoch;
    completion.epoch = epoch;
    completion.ticket = epoch;
    completion.kernel_tick = kernel_tick;
    completion.counter = value;
}
#endif

StatusType Os_Implementation_IncrementCounter(CounterType CounterID) {
    const Os_ErrorParameters arguments = {.service_IncrementCounter = {CounterID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_IncrementCounter);
    if (status == E_OK) {
        status = implementation_IncrementCounter(CounterID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_IncrementCounter, status, &arguments);
}

StatusType Os_Implementation_GetCounterValue(CounterType CounterID, TickRefType Value) {
    const Os_ErrorParameters arguments = {.service_GetCounterValue = {CounterID, Value}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetCounterValue);
    if (status == E_OK) {
        status = implementation_GetCounterValue(CounterID, Value);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetCounterValue, status, &arguments);
}

StatusType Os_Implementation_GetElapsedValue(CounterType CounterID, TickRefType Value,
                                             TickRefType ElapsedValue) {
    const Os_ErrorParameters arguments = {
        .service_GetElapsedValue = {CounterID, Value, ElapsedValue}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetElapsedValue);
    if (status == E_OK) {
        status = implementation_GetElapsedValue(CounterID, Value, ElapsedValue);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetElapsedValue, status, &arguments);
}

StatusType Os_Implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info) {
    const Os_ErrorParameters arguments = {.service_GetAlarmBase = {AlarmID, Info}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetAlarmBase);
    if (status == E_OK) {
        status = implementation_GetAlarmBase(AlarmID, Info);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetAlarmBase, status, &arguments);
}

StatusType Os_Implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick) {
    const Os_ErrorParameters arguments = {.service_GetAlarm = {AlarmID, Tick}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetAlarm);
    if (status == E_OK) {
        status = implementation_GetAlarm(AlarmID, Tick);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetAlarm, status, &arguments);
}

StatusType Os_Implementation_SetRelAlarm(AlarmType AlarmID, TickType Increment, TickType Cycle) {
    const Os_ErrorParameters arguments = {.service_SetRelAlarm = {AlarmID, Increment, Cycle}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_SetRelAlarm);
    if (status == E_OK) {
        status = implementation_SetRelAlarm(AlarmID, Increment, Cycle);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_SetRelAlarm, status, &arguments);
}

StatusType Os_Implementation_SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle) {
    const Os_ErrorParameters arguments = {.service_SetAbsAlarm = {AlarmID, Start, Cycle}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_SetAbsAlarm);
    if (status == E_OK) {
        status = implementation_SetAbsAlarm(AlarmID, Start, Cycle);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_SetAbsAlarm, status, &arguments);
}

StatusType Os_Implementation_CancelAlarm(AlarmType AlarmID) {
    const Os_ErrorParameters arguments = {.service_CancelAlarm = {AlarmID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_CancelAlarm);
    if (status == E_OK) {
        status = implementation_CancelAlarm(AlarmID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_CancelAlarm, status, &arguments);
}
