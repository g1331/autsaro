/** @file Counters, alarms and the controlled tick with its completion handshake. */
#include "Os_Backend.h"
#include "Os_IntegrationHooks.h"

#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static TickType
    values[OS_MAX_COUNTERS]; /**< Port-locked Counter ticks; TimeInit does not reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Time.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Time.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const TickType *const Os_ArtiCounters =
    values; /**< Fixed Counter ticks tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
typedef Os_ArtiAlarmStateType Os_AlarmStateType;
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_AlarmStateType
    alarms[OS_MAX_ALARMS]; /**< Port-locked Alarm state; TimeInit does not reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const Os_ArtiAlarmStateType *const Os_ArtiAlarms =
    alarms; /**< Fixed Alarm state tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    tick_state; /**< Atomic handshake/close state; TimeInit does not reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static uint64
    requested_epoch; /**< Producer-owned epoch, tick_state ordered; no Init reset. */

OS_MEMMAP static uint64
    confirmed_epoch; /**< Owner-Task epoch, tick_state ordered; no Init reset. */

OS_MEMMAP static uint64 action_count; /**< Port-locked action total; TimeInit does not reset. */

OS_MEMMAP static uint64
    error_count; /**< Port-locked action-error total; TimeInit does not reset. */

OS_MEMMAP static Os_TickCompletionType
    completion; /**< Owner-Task snapshot, tick_state published; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE completion_event; /**< Manual event created by TimeInit; then fixed. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
/* Auto-reset: one early native wake per notification, never a completion. */
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP static HANDLE attention_event; /**< Auto-reset event created by TimeInit; then fixed. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    signal_failed; /**< Atomic host-signal failure latch; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define TIME_CLOSED     256L
#define TIME_RESERVED   1L
#define TIME_PENDING    2L
#define TIME_PROCESSING 3L
#define TIME_DELIVERED  4L
#define TIME_MARKED     5L
#define TIME_COMPLETE   6L
#define TIME_PUBLISHING 7L

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static uint32 counter_index(CounterType id)
{
    const Os_TimeConfigType *config = Os_Config->time;
    if (config != NULL) {
        for (uint32 i = 0u; i < config->counter_count; ++i) {
            if (config->counters[i].id == id) {
                return i;
            }
        }
    }
    return OS_MAX_COUNTERS;
}

OS_MEMMAP static uint32 alarm_index(AlarmType id)
{
    const Os_TimeConfigType *config = Os_Config->time;
    if (config != NULL) {
        for (uint32 i = 0u; i < config->alarm_count; ++i) {
            if (config->alarms[i].id == id) {
                return i;
            }
        }
    }
    return OS_MAX_ALARMS;
}

OS_MEMMAP static const Os_TaskConfigType *configured_task(const Os_TargetConfigType *config,
                                                          TaskType id)
{
    uint32 i;
    const Os_TaskConfigType *result = NULL;
    for (i = 0u; i < config->task_count; ++i) {
        if (config->tasks[i].id == id) {
            result = &config->tasks[i];
            break;
        }
    }
    return result;
}

OS_MEMMAP static sint32 valid_cycle(const Os_CounterConfigType *counter, TickType cycle)
{
    return ((cycle == 0u) || ((cycle >= counter->minimum_cycle) && (cycle <= counter->maximum)))
               ? 1
               : 0;
}

OS_MEMMAP StatusType Os_TimeValidate(const Os_TargetConfigType *target)
{
    const Os_TimeConfigType *config = target->time;
    const Os_TaskConfigType *owner;
    uint32 i;
    uint32 j;
    sint32 system_found = 0;
    uint8 increments[OS_MAX_COUNTERS][OS_MAX_COUNTERS] = {0};
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
        const Os_CounterConfigType *counter = &config->counters[i];
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
        const Os_AlarmConfigType *alarm = &config->alarms[i];
        const Os_TaskConfigType *task = configured_task(target, alarm->task);
        const Os_CounterConfigType *counter = NULL;
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
            uint32 from = OS_MAX_COUNTERS;
            uint32 to = OS_MAX_COUNTERS;
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
    for (uint32 k = 0u; k < config->counter_count; ++k) {
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

OS_MEMMAP static TickType absolute_distance(uint32 counter, TickType start)
{
    TickType current = values[counter];
    TickType modulus = Os_Config->time->counters[counter].maximum + UINT64_C(1);
    return (start > current) ? (start - current) : (modulus - (current - start));
}

OS_MEMMAP void Os_TimeInit(AppModeType mode)
{
    const Os_TimeConfigType *config = Os_Config->time;
    if ((config != NULL) && (config->wake_event != 0u)) {
        const Os_TaskConfigType *owner = configured_task(Os_Config, config->owner);
        if ((owner->autostart_modes & mode) == 0u) {
            Os_BackendShutdown(E_OS_VALUE);
        }
        completion_event = Os_PortEvent(NULL, 1, 0, NULL);
        attention_event = Os_PortEvent(NULL, 0, 0, NULL);
    }
}

OS_MEMMAP void Os_TimeAutostart(AppModeType mode)
{
    const Os_TimeConfigType *config = Os_Config->time;
    if (config != NULL) {
        for (uint32 i = 0u; i < config->alarm_count; ++i) {
            const Os_AlarmConfigType *alarm = &config->alarms[i];
            if ((alarm->autostart_modes & mode) != 0u) {
                uint32 counter = counter_index(alarm->counter);
                alarms[i].active = 1u;
                alarms[i].cycle = alarm->cycle;
                alarms[i].remaining = (alarm->absolute != 0u)
                                          ? absolute_distance(counter, alarm->start)
                                          : alarm->start;
            }
        }
    }
}

OS_MEMMAP void Os_TimeClose(void)
{
    (void)InterlockedOr(&tick_state, TIME_CLOSED);
    if (completion_event != NULL) {
        if (Os_HostSetEvent(completion_event) == 0) {
            (void)InterlockedExchange(&signal_failed, 1);
        }
    }
}

OS_MEMMAP sint32 Os_TimeSignalFailed(void)
{
    return (InterlockedCompareExchange(&signal_failed, 0, 0) != 0) ? 1 : 0;
}

OS_MEMMAP void Os_TimeReportAction(StatusType status)
{
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

OS_MEMMAP static void action(uint32 index)
{
    const Os_AlarmConfigType *alarm = &Os_Config->time->alarms[index];
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

OS_MEMMAP TickType Os_TimeCounterValue(CounterType counter)
{
    return values[counter_index(counter)];
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void increment(uint32 counter)
{
    typedef struct {
        uint32 counter;
        uint32 alarm;
        uint8 report;
    } Os_CounterWorkType;
    Os_CounterWorkType work[OS_MAX_COUNTERS];
    uint32 depth = 1u;
    const Os_TimeConfigType *config = Os_Config->time;
    work[0] = (Os_CounterWorkType){counter, 0u, 0u};
    values[counter] = (values[counter] == config->counters[counter].maximum)
                          ? 0u
                          : (values[counter] + UINT64_C(1));
    while (depth != 0u) {
        Os_CounterWorkType *current = &work[depth - 1u];
        if (current->alarm < config->alarm_count) {
            uint32 i = current->alarm;
            ++current->alarm;
            if ((config->alarms[i].counter == config->counters[current->counter].id) &&
                (alarms[i].active != 0u)) {
                configASSERT(alarms[i].remaining != 0u);
                --alarms[i].remaining;
                if (alarms[i].remaining == 0u) {
                    alarms[i].remaining = alarms[i].cycle;
                    alarms[i].active = (alarms[i].cycle == 0u) ? 0u : 1u;
                    if (config->alarms[i].action == OS_ALARM_INCREMENT_COUNTER) {
                        uint32 child = counter_index(config->alarms[i].increment_counter);
                        configASSERT(depth < OS_MAX_COUNTERS);
                        work[depth] = (Os_CounterWorkType){child, 0u, 1u};
                        ++depth;
                        values[child] = (values[child] == config->counters[child].maximum)
                                            ? 0u
                                            : (values[child] + UINT64_C(1));
                    } else {
                        action(i);
                    }
                }
            }
        } else {
            uint8 report = current->report;
            Os_ScheduleIncrement(config->counters[current->counter].id);
            --depth;
            if (report != 0u) {
                Os_TimeReportAction(E_OK);
            }
        }
    }
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static StatusType implementation_IncrementCounter(CounterType CounterID)
{
    uint32 index;
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

OS_MEMMAP static StatusType implementation_GetCounterValue(CounterType CounterID, TickRefType Value)
{
    uint32 index;
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

OS_MEMMAP static StatusType implementation_GetElapsedValue(CounterType CounterID,
                                                           TickRefType Value,
                                                           TickRefType ElapsedValue)
{
    uint32 index;
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

OS_MEMMAP static StatusType implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info)
{
    uint32 index;
    const Os_CounterConfigType *counter;
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

OS_MEMMAP static StatusType implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick)
{
    uint32 index;
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

OS_MEMMAP static StatusType set_alarm(AlarmType id, TickType start, TickType cycle, sint32 absolute)
{
    uint32 index;
    uint32 counter;
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

OS_MEMMAP static StatusType implementation_SetRelAlarm(AlarmType AlarmID,
                                                       TickType Increment,
                                                       TickType Cycle)
{
    return set_alarm(AlarmID, Increment, Cycle, 0);
}

OS_MEMMAP static StatusType implementation_SetAbsAlarm(AlarmType AlarmID,
                                                       TickType Start,
                                                       TickType Cycle)
{
    return set_alarm(AlarmID, Start, Cycle, 1);
}

OS_MEMMAP static StatusType implementation_CancelAlarm(AlarmType AlarmID)
{
    uint32 index;
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

OS_MEMMAP sint32 Os_TimeBeginTick(void)
{
    if (Os_Config->time == NULL) {
        return Os_TargetReady();
    }
    if (Os_Config->time->wake_event == 0u) {
        return 0;
    }
    return (InterlockedCompareExchange(&tick_state, TIME_PROCESSING, TIME_PENDING) == TIME_PENDING)
               ? 1
               : 0;
}

OS_MEMMAP void Os_TimeTick(void)
{
    const Os_TimeConfigType *config = Os_Config->time;
    if ((config != NULL) && (config->wake_event != 0u)) {
        if (InterlockedCompareExchange(&tick_state, 0, 0) != TIME_PROCESSING) {
            Os_BackendShutdown(E_OS_STATE);
        }
        action_count = 0u;
        error_count = 0u;
        /* Host timer counters are registers driven by the actual controlled
         * kernel tick ISR. Standard IncrementCounter cannot write them. */
        for (uint32 i = 0u; i < config->counter_count; ++i) {
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

OS_MEMMAP void vApplicationTickHook(void) { Os_TimeTick(); }

OS_MEMMAP static StatusType native_time_context(void)
{
    if (Os_BridgeActorAllowed() == 0) {
        return E_OS_CALLEVEL;
    }
    if ((Os_TargetReady() == 0) || (Os_Config->time == NULL) ||
        (Os_Config->time->wake_event == 0u)) {
        return E_OS_STATE;
    }
    return Os_BridgeContext();
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_pending(void) { Os_TimeTestBeforePending(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_pending(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket)
{
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
    observe_tick_pending();
    if (InterlockedCompareExchange(&tick_state, TIME_PENDING, TIME_RESERVED) != TIME_RESERVED) {
        return E_OS_STATE;
    }
    *ticket = epoch;
    Os_PortPostInterrupt(1u);
    return E_OK;
}

OS_MEMMAP StatusType Os_TargetTickCompletion(uint64 ticket, Os_TickCompletionType *record)
{
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

OS_MEMMAP StatusType Os_TargetWaitTick(uint64 ticket,
                                       uint32 timeout_ms,
                                       Os_TickCompletionType *record)
{
    StatusType status = E_OS_VALUE;
    if ((timeout_ms != 0u) && (timeout_ms <= 5000u)) {
        const DWORD started_at = GetTickCount();
        boolean waiting = TRUE;
        while (waiting) {
            const HANDLE observed[2] = {completion_event, attention_event};
            status = Os_TargetTickCompletion(ticket, record);
            if (status != E_OS_NOFUNC) {
                waiting = FALSE;
            } else {
                const DWORD elapsed = GetTickCount() - started_at;
                if (elapsed >= timeout_ms) {
                    waiting = FALSE;
                } else {
                    /* Native timed waits can round up to the host timer period;
                     * pending native work wakes this producer explicitly. */
                    const DWORD waited =
                        WaitForMultipleObjects(2u, observed, 0, timeout_ms - elapsed);
                    if ((waited == WAIT_TIMEOUT) || (waited == (WAIT_OBJECT_0 + 1u))) {
                        /* A simultaneous close wins over timeout or attention. */
                        status = Os_TargetTickCompletion(ticket, record);
                        if (status == E_OS_NOFUNC) {
                            if (Os_TargetReady() == 0) {
                                status = E_OS_STATE;
                            }
                        }
                        waiting = FALSE;
                    } else if (waited != WAIT_OBJECT_0) {
                        (void)InterlockedExchange(&signal_failed, 1);
                        Os_BackendRequestShutdown(E_OS_STATE);
                        status = E_OS_STATE;
                        waiting = FALSE;
                    } else {
                        /* Completion publication is rechecked before returning. */
                    }
                }
            }
        }
    }
    return status;
}

OS_MEMMAP static StatusType time_owner(void)
{
    sint32 owner;
    Os_StackCheck();
    Os_BackendGuardService();
    if ((Os_BackendReady() == 0) || (Os_Config->time == NULL) ||
        (Os_Config->time->wake_event == 0u)) {
        return E_OS_STATE;
    }
    owner = Os_BackendTaskOwner(Os_Config->time->owner);
    return (owner == 0) ? E_OS_CALLEVEL : ((owner < 0) ? E_OS_ACCESS : E_OK);
}

OS_MEMMAP StatusType Os_TargetCurrentTick(uint64 *ticket, uint64 *epoch)
{
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

OS_MEMMAP StatusType Os_TargetNotifyNative(void)
{
    StatusType status = time_owner();
    if (status == E_OK) {
        if (Os_HostSetEvent(attention_event) == 0) {
            (void)InterlockedExchange(&signal_failed, 1);
            Os_BackendRequestShutdown(E_OS_STATE);
            status = E_OS_STATE;
        }
        Os_BackendGuardService();
    }
    return status;
}

OS_MEMMAP StatusType Os_TargetCompleteTick(uint64 ticket)
{
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
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_waiting(void) { Os_TimeTestOnWaiting(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_tick_waiting(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TimeOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate)
{
    const Os_TimeConfigType *config = Os_Config->time;
    observe_tick_waiting();
    Os_IntegrationOnWaiting(id, pending, predicate);
    boolean publish = FALSE;
    if ((config != NULL) && (id == config->owner) && (pending == 0u) &&
        ((predicate & config->wake_event) != 0u)) {
        if (Os_MailboxQuiescent() != 0) {
            publish = InterlockedCompareExchange(&tick_state, TIME_PUBLISHING, TIME_MARKED) ==
                      TIME_MARKED;
        }
    }
    if (publish) {
        completion.epoch = requested_epoch;
        completion.ticket = requested_epoch;
        completion.kernel_tick = (uint32)xTaskGetTickCount();
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
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_TIME_TESTS) && (OS_TIME_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TimeTestCloseEvent(void)
{
    configASSERT(completion_event != NULL);
    configASSERT(CloseHandle(completion_event) != 0);
}

OS_MEMMAP void Os_TimeTestSeed(uint64 epoch, uint32 kernel_tick, TickType value)
{
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
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_Implementation_IncrementCounter(CounterType CounterID)
{
    const Os_ErrorParametersType arguments = {.service_IncrementCounter = {CounterID}};
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

OS_MEMMAP StatusType Os_Implementation_GetCounterValue(CounterType CounterID, TickRefType Value)
{
    const Os_ErrorParametersType arguments = {.service_GetCounterValue = {CounterID, Value}};
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

OS_MEMMAP StatusType Os_Implementation_GetElapsedValue(CounterType CounterID,
                                                       TickRefType Value,
                                                       TickRefType ElapsedValue)
{
    const Os_ErrorParametersType arguments = {
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

OS_MEMMAP StatusType Os_Implementation_GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info)
{
    const Os_ErrorParametersType arguments = {.service_GetAlarmBase = {AlarmID, Info}};
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

OS_MEMMAP StatusType Os_Implementation_GetAlarm(AlarmType AlarmID, TickRefType Tick)
{
    const Os_ErrorParametersType arguments = {.service_GetAlarm = {AlarmID, Tick}};
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

OS_MEMMAP StatusType Os_Implementation_SetRelAlarm(AlarmType AlarmID,
                                                   TickType Increment,
                                                   TickType Cycle)
{
    const Os_ErrorParametersType arguments = {.service_SetRelAlarm = {AlarmID, Increment, Cycle}};
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

OS_MEMMAP StatusType Os_Implementation_SetAbsAlarm(AlarmType AlarmID,
                                                   TickType Start,
                                                   TickType Cycle)
{
    const Os_ErrorParametersType arguments = {.service_SetAbsAlarm = {AlarmID, Start, Cycle}};
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

OS_MEMMAP StatusType Os_Implementation_CancelAlarm(AlarmType AlarmID)
{
    const Os_ErrorParametersType arguments = {.service_CancelAlarm = {AlarmID}};
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
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
