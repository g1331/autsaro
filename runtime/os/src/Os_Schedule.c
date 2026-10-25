/** @file Schedule table validation, start/stop/chain services and expiry-point processing. */
#include "Os_Backend.h"

typedef Os_ArtiScheduleStateType Os_ScheduleStateType;
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_ScheduleStateType
    tables[OS_MAX_SCHEDULE_TABLES]; /**< Port-locked table state; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Schedule.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Schedule.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CONST_PTR
#include "Os_MemMap.h"
OS_MEMMAP const Os_ArtiScheduleStateType *const Os_ArtiScheduleTables =
    tables; /**< Fixed table state tool view; C init only, never reset. */
#define OS_STOP_SEC_CONST_PTR
#include "Os_MemMap.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static uint32 table_index(ScheduleTableType id)
{
    const Os_TimeConfigType *time_configuration = Os_Config->timing;
    if (time_configuration != NULL) {
        for (uint32 i = 0u; i < time_configuration->schedule_table_count; ++i) {
            if (time_configuration->schedule_tables[i].id == id) {
                return i;
            }
        }
    }
    return OS_MAX_SCHEDULE_TABLES;
}

OS_MEMMAP static const Os_CounterConfigType *counter_config(
    const Os_TimeConfigType *time_configuration, CounterType id)
{
    const Os_CounterConfigType *result = NULL;
    for (uint32 i = 0u; i < time_configuration->counter_count; ++i) {
        if (time_configuration->counters[i].id == id) {
            result = &time_configuration->counters[i];
            break;
        }
    }
    return result;
}

OS_MEMMAP static sint32 valid_action(const Os_TargetConfigType *target,
                                     const Os_ExpiryActionType *action)
{
    for (uint32 i = 0u; i < target->task_count; ++i) {
        const Os_TaskConfigType *task = &target->tasks[i];
        if (task->id == action->task) {
            return ((action->action == OS_ALARM_ACTIVATE) ||
                    ((action->action == OS_ALARM_EVENT) && (action->event != 0u) &&
                     (task->kind == OS_EXTENDED_TASK)))
                       ? 1
                       : 0;
        }
    }
    return 0;
}

OS_MEMMAP StatusType Os_ScheduleValidate(const Os_TargetConfigType *target)
{
    const Os_TimeConfigType *time_configuration = target->timing;
    if ((time_configuration->schedule_table_count > OS_MAX_SCHEDULE_TABLES) ||
        ((time_configuration->schedule_table_count != 0u) &&
         (time_configuration->schedule_tables == NULL))) {
        return E_OS_VALUE;
    }
    for (uint32 i = 0u; i < time_configuration->schedule_table_count; ++i) {
        const Os_ScheduleTableConfigType *table = &time_configuration->schedule_tables[i];
        const Os_CounterConfigType *counter = counter_config(time_configuration, table->counter);
        if (counter == NULL) {
            return E_OS_ID;
        }
        if ((table->id >= OS_MAX_SCHEDULE_TABLES) || (table->point_count == 0u) ||
            (table->point_count > OS_MAX_EXPIRY_POINTS) || (table->points == NULL) ||
            (table->repeating > 1u) || (table->synchronization != OS_SCHEDULE_SYNC_NONE) ||
            (table->autostart_modes > 3u) || (table->absolute > 1u)) {
            return E_OS_VALUE;
        }
        for (uint32 j = 0u; j < i; ++j) {
            if (table->id == time_configuration->schedule_tables[j].id) {
                return E_OS_ID;
            }
        }
        for (uint32 j = 0u; j < table->point_count; ++j) {
            const Os_ExpiryPointType *point = &table->points[j];
            TickType delay = point->offset;
            if (j != 0u) {
                if (point->offset <= table->points[j - 1u].offset) {
                    return E_OS_VALUE;
                }
                delay -= table->points[j - 1u].offset;
            }
            if ((delay > counter->maximum) ||
                (((j != 0u) || (delay != 0u)) && (delay < counter->minimum_cycle)) ||
                (point->offset > table->duration) || (point->action_count == 0u) ||
                (point->action_count > OS_MAX_EXPIRY_ACTIONS) || (point->actions == NULL)) {
                return E_OS_VALUE;
            }
            for (uint32 k = 0u; k < point->action_count; ++k) {
                if (valid_action(target, &point->actions[k]) == 0) {
                    return E_OS_ID;
                }
            }
        }
        {
            TickType final = table->duration - table->points[table->point_count - 1u].offset;
            if ((final > counter->maximum) ||
                ((table->repeating != 0u) && (final < counter->minimum_cycle))) {
                return E_OS_VALUE;
            }
        }
        if ((table->autostart_modes != 0u) &&
            ((table->start > counter->maximum) ||
             ((table->absolute == 0u) &&
              ((table->start == 0u) ||
               (table->start > (counter->maximum - table->points[0].offset)))))) {
            return E_OS_VALUE;
        }
    }
    return E_OK;
}

OS_MEMMAP static void start(uint32 index, TickType delay)
{
    const Os_ScheduleTableConfigType *table = &Os_Config->timing->schedule_tables[index];
    tables[index].status = SCHEDULETABLE_RUNNING;
    tables[index].point = 0u;
    tables[index].remaining = delay + table->points[0].offset;
    tables[index].next = OS_MAX_SCHEDULE_TABLES;
    tables[index].previous = OS_MAX_SCHEDULE_TABLES;
}

OS_MEMMAP static TickType absolute_delay(const Os_ScheduleTableConfigType *table, TickType value)
{
    const Os_CounterConfigType *counter = counter_config(Os_Config->timing, table->counter);
    TickType current = Os_TimeCounterValue(table->counter);
    return (value > current) ? (value - current)
                             : (counter->maximum - (current - value) + UINT64_C(1));
}

OS_MEMMAP void Os_ScheduleAutostart(AppModeType mode)
{
    const Os_TimeConfigType *time_configuration = Os_Config->timing;
    if (time_configuration != NULL) {
        for (uint32 i = 0u; i < time_configuration->schedule_table_count; ++i) {
            const Os_ScheduleTableConfigType *table = &time_configuration->schedule_tables[i];
            tables[i].status = SCHEDULETABLE_STOPPED;
            tables[i].next = OS_MAX_SCHEDULE_TABLES;
            tables[i].previous = OS_MAX_SCHEDULE_TABLES;
            if ((table->autostart_modes & mode) != 0u) {
                start(i,
                      (table->absolute != 0u) ? absolute_delay(table, table->start) : table->start);
            }
        }
    }
}

OS_MEMMAP static void expiry(const Os_ExpiryPointType *point)
{
    Os_ArtiInternalEnter();
    /* All activations precede all event settings; declaration order is not
     * permitted to set an event on a still-suspended destination Task. */
    for (uint32 pass = 0u; pass < 2u; ++pass) {
        for (uint32 i = 0u; i < point->action_count; ++i) {
            const Os_ExpiryActionType *action = &point->actions[i];
            if ((uint32)action->action == pass) {
                StatusType status = (pass == OS_ALARM_ACTIVATE)
                                        ? ActivateTask(action->task)
                                        : SetEvent(action->task, action->event);
                Os_TimeReportAction(status);
            }
        }
    }
    Os_ArtiInternalLeave();
}

OS_MEMMAP static void process(uint32 initial)
{
    uint32 index = initial;
    uint32 transitions = 0u;
    while ((tables[index].status == SCHEDULETABLE_RUNNING) && (tables[index].remaining == 0u)) {
        const Os_ScheduleTableConfigType *table = &Os_Config->timing->schedule_tables[index];
        Os_ScheduleStateType *state = &tables[index];
        if (++transitions > (OS_MAX_SCHEDULE_TABLES * (OS_MAX_EXPIRY_POINTS + 1u))) {
            Os_BackendShutdown(E_OS_STATE);
        }
        if (state->point < table->point_count) {
            TickType offset = table->points[state->point].offset;
            expiry(&table->points[state->point]);
            ++state->point;
            state->remaining =
                ((state->point < table->point_count) ? table->points[state->point].offset
                                                     : table->duration) -
                offset;
        } else {
            uint32 next = state->next;
            state->next = OS_MAX_SCHEDULE_TABLES;
            state->status = SCHEDULETABLE_STOPPED;
            if (next != OS_MAX_SCHEDULE_TABLES) {
                index = next;
                start(index, 0u);
            } else if (table->repeating != 0u) {
                start(index, 0u);
            } else {
                /* Single-shot final delay completed; no further actions. */
            }
        }
    }
}

OS_MEMMAP void Os_ScheduleIncrement(CounterType counter)
{
    const Os_TimeConfigType *time_configuration = Os_Config->timing;
    uint8 running[OS_MAX_SCHEDULE_TABLES] = {0};
    for (uint32 i = 0u; i < time_configuration->schedule_table_count; ++i) {
        running[i] = ((time_configuration->schedule_tables[i].counter == counter) &&
                      (tables[i].status == SCHEDULETABLE_RUNNING))
                         ? 1u
                         : 0u;
    }
    for (uint32 i = 0u; i < time_configuration->schedule_table_count; ++i) {
        if (running[i] != 0u) {
            configASSERT(tables[i].remaining != 0u);
            --tables[i].remaining;
            process(i);
        }
    }
}

OS_MEMMAP static StatusType context(void)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return (Os_BackendServiceContext() != 0) ? E_OK : E_OS_CALLEVEL;
}

OS_MEMMAP static StatusType start_table(ScheduleTableType id, TickType value, sint32 absolute)
{
    uint32 index;
    const Os_ScheduleTableConfigType *table;
    const Os_CounterConfigType *counter;
    StatusType status = context();
    if (status != E_OK) {
        return status;
    }
    index = table_index(id);
    if (index == OS_MAX_SCHEDULE_TABLES) {
        return E_OS_ID;
    }
    table = &Os_Config->timing->schedule_tables[index];
    counter = counter_config(Os_Config->timing, table->counter);
    if ((value > counter->maximum) ||
        ((absolute == 0) &&
         ((value == 0u) || (value > (counter->maximum - table->points[0].offset))))) {
        return E_OS_VALUE;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (tables[index].status != SCHEDULETABLE_STOPPED) {
        status = E_OS_STATE;
    } else {
        start(index, (absolute != 0) ? absolute_delay(table, value) : value);
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}

OS_MEMMAP static StatusType implementation_StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                                 TickType Offset)
{
    return start_table(ScheduleTableID, Offset, 0);
}

OS_MEMMAP static StatusType implementation_StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                                 TickType Start)
{
    return start_table(ScheduleTableID, Start, 1);
}

OS_MEMMAP static StatusType implementation_StopScheduleTable(ScheduleTableType ScheduleTableID)
{
    uint32 index;
    StatusType status = context();
    if (status != E_OK) {
        return status;
    }
    index = table_index(ScheduleTableID);
    if (index == OS_MAX_SCHEDULE_TABLES) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (tables[index].status == SCHEDULETABLE_STOPPED) {
        status = E_OS_NOFUNC;
    } else {
        if (tables[index].next != OS_MAX_SCHEDULE_TABLES) {
            tables[tables[index].next].status = SCHEDULETABLE_STOPPED;
            tables[tables[index].next].previous = OS_MAX_SCHEDULE_TABLES;
        }
        if (tables[index].previous != OS_MAX_SCHEDULE_TABLES) {
            tables[tables[index].previous].next = OS_MAX_SCHEDULE_TABLES;
        }
        tables[index].status = SCHEDULETABLE_STOPPED;
        tables[index].next = OS_MAX_SCHEDULE_TABLES;
        tables[index].previous = OS_MAX_SCHEDULE_TABLES;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}

OS_MEMMAP static StatusType implementation_NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                                             ScheduleTableType ScheduleTableID_To)
{
    uint32 from;
    uint32 to;
    StatusType status = context();
    if (status != E_OK) {
        return status;
    }
    from = table_index(ScheduleTableID_From);
    to = table_index(ScheduleTableID_To);
    if ((from == OS_MAX_SCHEDULE_TABLES) || (to == OS_MAX_SCHEDULE_TABLES) ||
        (Os_Config->timing->schedule_tables[from].counter !=
         Os_Config->timing->schedule_tables[to].counter)) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    if (tables[from].status != SCHEDULETABLE_RUNNING) {
        status = E_OS_NOFUNC;
    } else if (tables[to].status != SCHEDULETABLE_STOPPED) {
        status = E_OS_STATE;
    } else {
        if (tables[from].next != OS_MAX_SCHEDULE_TABLES) {
            tables[tables[from].next].status = SCHEDULETABLE_STOPPED;
            tables[tables[from].next].previous = OS_MAX_SCHEDULE_TABLES;
        }
        tables[from].next = to;
        tables[to].previous = from;
        tables[to].status = SCHEDULETABLE_NEXT;
    }
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return status;
}

OS_MEMMAP static StatusType implementation_GetScheduleTableStatus(
    ScheduleTableType ScheduleTableID, ScheduleTableStatusRefType ScheduleStatus)
{
    uint32 index;
    StatusType status;
    if (ScheduleStatus == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = context();
    if (status != E_OK) {
        return status;
    }
    index = table_index(ScheduleTableID);
    if (index == OS_MAX_SCHEDULE_TABLES) {
        return E_OS_ID;
    }
    taskENTER_CRITICAL();
    Os_BackendGuardService();
    *ScheduleStatus = tables[index].status;
    taskEXIT_CRITICAL();
    Os_BackendGuardService();
    return E_OK;
}

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableRel(ScheduleTableType ScheduleTableID,
                                                             TickType Offset)
{
    const Os_ErrorParametersType arguments = {
        .service_StartScheduleTableRel = {ScheduleTableID, Offset}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_StartScheduleTableRel);
    if (status == E_OK) {
        status = implementation_StartScheduleTableRel(ScheduleTableID, Offset);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_StartScheduleTableRel, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_StartScheduleTableAbs(ScheduleTableType ScheduleTableID,
                                                             TickType Start)
{
    const Os_ErrorParametersType arguments = {
        .service_StartScheduleTableAbs = {ScheduleTableID, Start}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_StartScheduleTableAbs);
    if (status == E_OK) {
        status = implementation_StartScheduleTableAbs(ScheduleTableID, Start);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_StartScheduleTableAbs, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_StopScheduleTable(ScheduleTableType ScheduleTableID)
{
    const Os_ErrorParametersType arguments = {.service_StopScheduleTable = {ScheduleTableID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_StopScheduleTable);
    if (status == E_OK) {
        status = implementation_StopScheduleTable(ScheduleTableID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_StopScheduleTable, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                                         ScheduleTableType ScheduleTableID_To)
{
    const Os_ErrorParametersType arguments = {
        .service_NextScheduleTable = {ScheduleTableID_From, ScheduleTableID_To}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_NextScheduleTable);
    if (status == E_OK) {
        status = implementation_NextScheduleTable(ScheduleTableID_From, ScheduleTableID_To);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_NextScheduleTable, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_GetScheduleTableStatus(
    ScheduleTableType ScheduleTableID, ScheduleTableStatusRefType ScheduleStatus)
{
    const Os_ErrorParametersType arguments = {
        .service_GetScheduleTableStatus = {ScheduleTableID, ScheduleStatus}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetScheduleTableStatus);
    if (status == E_OK) {
        status = implementation_GetScheduleTableStatus(ScheduleTableID, ScheduleStatus);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetScheduleTableStatus, status, &arguments);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
