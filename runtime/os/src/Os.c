#include "Os_Target.h"
#include "Os_Backend.h"
const Os_TargetConfig *Os_Config;
StatusType Os_TargetPrepare(const Os_TargetConfig *config) {
    size_t i, j;
    uint8_t highest_task = 0u;
    if (Os_Config != NULL) {
        return E_OS_STATE;
    }
    if (config == NULL || config->tasks == NULL || config->task_count == 0u ||
        config->task_count > OS_MAX_TASKS || config->host_stack_reserve < 65536u ||
        config->host_stack_reserve > 16777216u || config->host_stack_reserve % 65536u != 0u) {
        return E_OS_VALUE;
    }
    for (i = 0u; i < config->task_count; ++i) {
        const Os_TaskConfig *task = &config->tasks[i];
        if (task->priority > highest_task) {
            highest_task = task->priority;
        }
        if (task->id >= OS_MAX_TASKS) {
            return E_OS_ID;
        }
        if ((task->name == NULL) || (task->entry == NULL) || (task->priority == 0u) ||
            (task->priority > OS_MAX_PRIORITY) || (task->autostart_modes > 3u) ||
            (task->kind > OS_EXTENDED_TASK) || (task->activation_limit == 0u) ||
            (task->activation_limit > OS_MAX_ACTIVATIONS) ||
            ((task->kind == OS_EXTENDED_TASK) && (task->activation_limit != 1u)) ||
            (task->schedule > OS_SCHEDULE_NON) ||
            (task->internal_resource > OS_MAX_INTERNAL_RESOURCES)) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < i; ++j) {
            if (task->id == config->tasks[j].id) {
                return E_OS_ID;
            }
        }
    }
    if ((config->internal_resource_count > OS_MAX_INTERNAL_RESOURCES) ||
        ((config->internal_resource_count != 0u) && (config->internal_resources == NULL))) {
        return E_OS_VALUE;
    }
    for (i = 0u; i < config->internal_resource_count; ++i) {
        const Os_InternalResourceConfig *resource = &config->internal_resources[i];
        uint8_t highest_access = 0u;
        if ((resource->id == 0u) || (resource->id > OS_MAX_INTERNAL_RESOURCES) ||
            (resource->ceiling == 0u) || (resource->ceiling > OS_MAX_PRIORITY)) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < i; ++j) {
            if (resource->id == config->internal_resources[j].id) {
                return E_OS_ID;
            }
        }
        for (j = 0u; j < config->task_count; ++j) {
            const Os_TaskConfig *task = &config->tasks[j];
            if ((task->internal_resource == resource->id) && (task->priority > highest_access)) {
                highest_access = task->priority;
            }
            if ((task->internal_resource == resource->id) &&
                ((task->priority > resource->ceiling) ||
                 ((task->schedule == OS_SCHEDULE_NON) && (resource->ceiling != highest_task)))) {
                return E_OS_VALUE;
            }
        }
        for (j = 0u; j < config->task_count; ++j) {
            const Os_TaskConfig *task = &config->tasks[j];
            if ((highest_access != 0u) && (task->internal_resource != resource->id) &&
                (task->priority > highest_access) && (task->priority <= resource->ceiling)) {
                return E_OS_VALUE;
            }
        }
    }
    for (i = 0u; i < config->task_count; ++i) {
        if (config->tasks[i].internal_resource != 0u) {
            for (j = 0u; j < config->internal_resource_count; ++j) {
                if (config->tasks[i].internal_resource == config->internal_resources[j].id) {
                    break;
                }
            }
            if (j == config->internal_resource_count) {
                return E_OS_ID;
            }
        }
    }
    if ((config->resource_count > OS_MAX_RESOURCES) ||
        ((config->resource_count != 0u) && (config->resources == NULL))) {
        return E_OS_VALUE;
    }
    if (((config->category1_isrs & 1u) != 0u) ||
        ((config->input_event != 0u) &&
         ((config->category1_isrs & (UINT32_C(1) << OS_INPUT_INTERRUPT)) != 0u))) {
        return E_OS_VALUE;
    }
    if (config->input_event != 0u) {
        for (i = 0u; i < config->task_count; ++i) {
            if (config->tasks[i].id == config->input_task) {
                break;
            }
        }
        if (i == config->task_count) {
            return E_OS_ID;
        }
        if ((config->tasks[i].kind != OS_EXTENDED_TASK) ||
            (config->tasks[i].autostart_modes == 0u)) {
            return E_OS_VALUE;
        }
    }
    for (i = 0u; i < config->resource_count; ++i) {
        const Os_ResourceConfig *resource = &config->resources[i];
        uint32_t task_mask = 0u;
        uint8_t highest_access = 0u;
        if ((resource->id >= OS_MAX_RESOURCES) || (resource->ceiling == 0u) ||
            (resource->ceiling > ((resource->isr_access != 0u) ? 31u : OS_MAX_PRIORITY)) ||
            ((resource->isr_access & 1u) != 0u)) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < config->task_count; ++j) {
            task_mask |= 1u << config->tasks[j].id;
            if (((resource->task_access & (1u << config->tasks[j].id)) != 0u) &&
                (config->tasks[j].priority > highest_access)) {
                highest_access = config->tasks[j].priority;
            }
        }
        if (((resource->task_access == 0u) && (resource->isr_access == 0u)) ||
            (((uint32_t)resource->task_access & ~task_mask) != 0u) ||
            ((resource->isr_access & config->category1_isrs) != 0u)) {
            return E_OS_VALUE;
        }
        if ((resource->id == RES_SCHEDULER) &&
            ((resource->task_access != task_mask) || (resource->isr_access != 0u) ||
             (resource->ceiling != highest_task))) {
            return E_OS_VALUE;
        }
        if ((resource->ceiling < highest_access) ||
            ((resource->isr_access != 0u) && (resource->ceiling != 31u))) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < config->task_count; ++j) {
            const Os_TaskConfig *task = &config->tasks[j];
            if ((resource->isr_access == 0u) &&
                ((resource->task_access & (1u << task->id)) == 0u) &&
                (task->priority > highest_access) && (task->priority <= resource->ceiling)) {
                return E_OS_VALUE;
            }
        }
        for (j = 0u; j < i; ++j) {
            if (resource->id == config->resources[j].id) {
                return E_OS_ID;
            }
        }
    }
    {
        StatusType time_status = Os_TimeValidate(config);
        if (time_status != E_OK) {
            return time_status;
        }
    }
    Os_Config = config;
    return E_OK;
}
void StartOS(AppModeType Mode) { Os_BackendStart(Mode); }
void ShutdownOS(StatusType Error) {
    Os_StackCheck();
    Os_BackendShutdown(Error);
}
static StatusType implementation_GetTaskID(TaskRefType TaskID) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (TaskID == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendTaskId(TaskID);
}
AppModeType GetActiveApplicationMode(void) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendApplicationMode();
}
static StatusType implementation_GetTaskState(TaskType TaskID, TaskStateRefType State) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (State == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendState(TaskID, State);
}
static StatusType implementation_ActivateTask(TaskType TaskID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendActivate(TaskID);
}
static StatusType implementation_TerminateTask(void) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendFinish();
}
static StatusType implementation_ChainTask(TaskType TaskID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendChain(TaskID);
}
static StatusType implementation_GetResource(ResourceType ResID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 1);
}
static StatusType implementation_ReleaseResource(ResourceType ResID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 0);
}
static StatusType implementation_Schedule(void) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendSchedule();
}
static StatusType implementation_WaitEvent(EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendWait(Mask);
}
static StatusType implementation_ClearEvent(EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendClear(Mask);
}
static StatusType implementation_SetEvent(TaskType TaskID, EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendEvent(TaskID, Mask, NULL);
}
static StatusType implementation_GetEvent(TaskType TaskID, EventMaskRefType Event) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (Event == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendEvent(TaskID, 0u, Event);
}
StatusType Os_TargetInspectActivation(TaskType id, Os_ActivationInfo *info) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (info == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendInspect(id, info);
}

StatusType GetTaskID(TaskRefType TaskID) {
    const Os_ErrorParameters arguments = {.service_GetTaskID = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_GetTaskID) == 0) ? E_OS_CALLEVEL
                                                                 : implementation_GetTaskID(TaskID);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetTaskID, status, &arguments);
}

StatusType GetTaskState(TaskType TaskID, TaskStateRefType State) {
    const Os_ErrorParameters arguments = {.service_GetTaskState = {TaskID, State}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_GetTaskState) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_GetTaskState(TaskID, State);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetTaskState, status, &arguments);
}

StatusType ActivateTask(TaskType TaskID) {
    const Os_ErrorParameters arguments = {.service_ActivateTask = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_ActivateTask) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_ActivateTask(TaskID);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ActivateTask, status, &arguments);
}

StatusType TerminateTask(void) {
    const Os_ErrorParameters arguments = {.service_TerminateTask = {0u}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_TerminateTask) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_TerminateTask();
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_TerminateTask, status, &arguments);
}

StatusType ChainTask(TaskType TaskID) {
    const Os_ErrorParameters arguments = {.service_ChainTask = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_ChainTask) == 0) ? E_OS_CALLEVEL
                                                                 : implementation_ChainTask(TaskID);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ChainTask, status, &arguments);
}

StatusType GetResource(ResourceType ResID) {
    const Os_ErrorParameters arguments = {.service_GetResource = {ResID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_GetResource) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_GetResource(ResID);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetResource, status, &arguments);
}

StatusType ReleaseResource(ResourceType ResID) {
    const Os_ErrorParameters arguments = {.service_ReleaseResource = {ResID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_ReleaseResource) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_ReleaseResource(ResID);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ReleaseResource, status, &arguments);
}

StatusType Schedule(void) {
    const Os_ErrorParameters arguments = {.service_Schedule = {0u}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_Schedule) == 0) ? E_OS_CALLEVEL
                                                                : implementation_Schedule();
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_Schedule, status, &arguments);
}

StatusType WaitEvent(EventMaskType Mask) {
    const Os_ErrorParameters arguments = {.service_WaitEvent = {Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_WaitEvent) == 0) ? E_OS_CALLEVEL
                                                                 : implementation_WaitEvent(Mask);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_WaitEvent, status, &arguments);
}

StatusType ClearEvent(EventMaskType Mask) {
    const Os_ErrorParameters arguments = {.service_ClearEvent = {Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_ClearEvent) == 0) ? E_OS_CALLEVEL
                                                                  : implementation_ClearEvent(Mask);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ClearEvent, status, &arguments);
}

StatusType SetEvent(TaskType TaskID, EventMaskType Mask) {
    const Os_ErrorParameters arguments = {.service_SetEvent = {TaskID, Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_SetEvent) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_SetEvent(TaskID, Mask);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_SetEvent, status, &arguments);
}

StatusType GetEvent(TaskType TaskID, EventMaskRefType Event) {
    const Os_ErrorParameters arguments = {.service_GetEvent = {TaskID, Event}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = (Os_HookServiceAllowed(OSServiceId_GetEvent) == 0)
                 ? E_OS_CALLEVEL
                 : implementation_GetEvent(TaskID, Event);
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetEvent, status, &arguments);
}
