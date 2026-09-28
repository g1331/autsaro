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
    Os_Config = config;
    return E_OK;
}
void StartOS(AppModeType Mode) { Os_BackendStart(Mode); }
void ShutdownOS(StatusType Error) {
    Os_StackCheck();
    Os_BackendShutdown(Error);
}
StatusType GetTaskState(TaskType TaskID, TaskStateRefType State) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (State == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendState(TaskID, State);
}
StatusType ActivateTask(TaskType TaskID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendActivate(TaskID);
}
StatusType TerminateTask(void) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendFinish();
}
StatusType ChainTask(TaskType TaskID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendChain(TaskID);
}
StatusType GetResource(ResourceType ResID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 1);
}
StatusType ReleaseResource(ResourceType ResID) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 0);
}
StatusType Schedule(void) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendSchedule();
}
StatusType WaitEvent(EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendWait(Mask);
}
StatusType ClearEvent(EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendClear(Mask);
}
StatusType SetEvent(TaskType TaskID, EventMaskType Mask) {
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendEvent(TaskID, Mask, NULL);
}
StatusType GetEvent(TaskType TaskID, EventMaskRefType Event) {
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
