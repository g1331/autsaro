/** @file OS startup, shutdown and target validation; Task, resource and event service entries. */
#include "Os_Target.h"
#include "Os_Backend.h"
#define OS_START_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
OS_MEMMAP const Os_TargetConfigType
    *Os_Config; /**< Startup-only retained graph; no Init replacement/reset. */
#define OS_STOP_SEC_VAR_CLEARED_PTR
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    start_called; /**< Atomic StartOS-entry latch; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP uint32 Os_ResourceIndex(const Os_TargetConfigType *config, ResourceType id)
{
    uint32 index = 0u;
    while ((index < config->resource_count) && (config->resources[index].id != id)) {
        ++index;
    }
    return index;
}

OS_MEMMAP uint32 Os_ResourceRoot(const Os_TargetConfigType *config, uint32 index)
{
    uint32 root = index;
    uint32 steps = 0u;
    while ((root < config->resource_count) && (config->resources[root].linked != 0u) &&
           (steps < config->resource_count)) {
        root = Os_ResourceIndex(config, config->resources[root].link);
        ++steps;
    }
    return ((root < config->resource_count) && (config->resources[root].linked == 0u))
               ? root
               : config->resource_count;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* Highest Task or ISR priority among the accessors of every member of the
 * LINKED group rooted at `root`, and the Tasks among them. Every ISR level is
 * above every Task level on this target. */
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static StatusType group_access(const Os_TargetConfigType *config,
                                         uint32 root,
                                         uint32 *tasks,
                                         uint8 *highest)
{
    uint32 isrs = 0u;
    uint32 i;
    *tasks = 0u;
    *highest = 0u;
    for (i = 0u; i < config->resource_count; ++i) {
        if (Os_ResourceRoot(config, i) == root) {
            *tasks |= config->resources[i].task_access;
            isrs |= config->resources[i].isr_access;
        }
    }
    for (i = 0u; (isrs == 0u) && (i < config->task_count); ++i) {
        if (((*tasks & ((uint32)1u << config->tasks[i].id)) != 0u) &&
            (config->tasks[i].priority > *highest)) {
            *highest = config->tasks[i].priority;
        }
    }
    for (i = 2u; i < OS_MAX_INTERRUPTS; ++i) {
        if ((isrs & ((uint32)1u << i)) != 0u) {
            const uint8 priority = (config->interrupts == NULL) ? OS_MAX_ISR_PRIORITY
                                                                : config->interrupts->priorities[i];
            if (priority == 0u) {
                return E_OS_VALUE;
            }
            if (priority > *highest) {
                *highest = priority;
            }
        }
    }
    return E_OK;
}

OS_MEMMAP static StatusType validate_resource(const Os_TargetConfigType *config,
                                              uint32 index,
                                              uint8 highest_task)
{
    const Os_ResourceConfigType *resource = &config->resources[index];
    const uint32 root = Os_ResourceRoot(config, index);
    uint32 task_mask = 0u;
    uint32 group_tasks = 0u;
    uint8 highest_access = 0u;
    uint32 j;
    StatusType status = E_OK;
    for (j = 0u; j < config->task_count; ++j) {
        task_mask |= (uint32)1u << config->tasks[j].id;
    }
    if ((resource->id >= OS_MAX_RESOURCES) || (resource->ceiling == 0u) ||
        (resource->ceiling > ((resource->isr_access != 0u) ? 31u : OS_MAX_PRIORITY)) ||
        ((resource->isr_access & 3u) != 0u) || (root == config->resource_count) ||
        (config->resources[root].ceiling != resource->ceiling) ||
        ((config->resources[root].isr_access == 0u) != (resource->isr_access == 0u)) ||
        ((resource->task_access == 0u) && (resource->isr_access == 0u)) ||
        (((uint32)resource->task_access & ~task_mask) != 0u) ||
        ((resource->isr_access & config->category1_isrs) != 0u)) {
        status = E_OS_VALUE;
    }
    if (status == E_OK) {
        status = group_access(config, root, &group_tasks, &highest_access);
    }
    if ((status == E_OK) && (resource->ceiling < highest_access)) {
        status = E_OS_VALUE;
    }
    if ((status == E_OK) && (config->use_res_scheduler != 0u) && (resource->id == RES_SCHEDULER) &&
        ((resource->linked != 0u) || (resource->task_access != task_mask) ||
         (resource->isr_access != 0u) || (resource->ceiling != highest_task))) {
        status = E_OS_VALUE;
    }
    j = 0u;
    while ((status == E_OK) && (j < config->task_count)) {
        const Os_TaskConfigType *task = &config->tasks[j];
        if ((resource->isr_access == 0u) && ((group_tasks & ((uint32)1u << task->id)) == 0u) &&
            (task->priority > highest_access) && (task->priority <= resource->ceiling)) {
            status = E_OS_VALUE;
        }
        ++j;
    }
    j = 0u;
    while ((status == E_OK) && (j < index)) {
        if (resource->id == config->resources[j].id) {
            status = E_OS_ID;
        }
        ++j;
    }
    return status;
}

OS_MEMMAP StatusType Os_TargetPrepare(const Os_TargetConfigType *config)
{
    uint32 i, j;
    uint8 highest_task = 0u;
    if (Os_Config != NULL) {
        return E_OS_STATE;
    }
    if ((config == NULL) || (config->tasks == NULL) || (config->task_count == 0u) ||
        (config->task_count > OS_MAX_TASKS) || (config->host_stack_reserve < 65536u) ||
        (config->host_stack_reserve > 16777216u) || ((config->host_stack_reserve % 65536u) != 0u)) {
        return E_OS_VALUE;
    }
    for (i = 0u; i < config->task_count; ++i) {
        const Os_TaskConfigType *task = &config->tasks[i];
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
        const Os_InternalResourceConfigType *resource = &config->internal_resources[i];
        uint8 highest_access = 0u;
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
            const Os_TaskConfigType *task = &config->tasks[j];
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
            const Os_TaskConfigType *task = &config->tasks[j];
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
    if (((config->category1_isrs & 3u) != 0u) ||
        ((config->input_event != 0u) &&
         ((config->category1_isrs & ((uint32)1u << OS_INPUT_INTERRUPT)) != 0u))) {
        return E_OS_VALUE;
    }
    if (config->interrupts != NULL) {
        for (i = 0u; i < OS_MAX_INTERRUPTS; ++i) {
            const uint8 priority = config->interrupts->priorities[i];
            if ((priority > OS_MAX_ISR_PRIORITY) || ((i < 2u) && (priority != 0u)) ||
                (((config->category1_isrs & ((uint32)1u << i)) != 0u) && (priority == 0u)) ||
                ((config->input_event != 0u) && (i == OS_INPUT_INTERRUPT) && (priority == 0u))) {
                return E_OS_VALUE;
            }
            if ((config->interrupts->entries != NULL) && (config->interrupts->entries[i] != NULL) &&
                ((i < 2u) || (priority == 0u) ||
                 ((config->category1_isrs & ((uint32)1u << i)) != 0u) ||
                 ((config->input_event != 0u) && (i == OS_INPUT_INTERRUPT)))) {
                return E_OS_VALUE;
            }
        }
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
        const StatusType status = validate_resource(config, i, highest_task);
        if (status != E_OK) {
            return status;
        }
    }
    {
        StatusType time_status = Os_TimeValidate(config);
        if (time_status != E_OK) {
            return time_status;
        }
    }
    Os_Config = config;
    Arti_Init();
    return E_OK;
}

OS_MEMMAP void Os_Implementation_StartOS(AppModeType Mode)
{
    (void)InterlockedExchange(&start_called, 1);
    Os_StackCheck();
    if (Os_InterruptDisabled() != 0) {
        if (Os_BackendStarted() == 0) {
            /* Preserve the fixed first-StartOS non-returning rejection contract;
             * no native scheduler or automotive startup is performed. */
            Os_BackendShutdown(E_OS_DISABLEDINT);
        }
    } else {
        if (Os_HookContext() == OS_HOOK_NONE) {
            if (Os_BackendStarted() == 0) {
                Os_BackendStart(Mode);
            }
        }
    }
}

OS_MEMMAP boolean Os_Implementation_isOsStarted(void)
{
    const StatusType status = Os_ServiceAccessStatus(OSServiceId_isOsStarted);
    Os_StackCheck();
    Os_BackendGuardService();
    if (status != E_OK) {
        return FALSE;
    }
    return (InterlockedCompareExchange(&start_called, 0, 0) != 0) ? TRUE : FALSE;
}

OS_MEMMAP CoreIdType Os_Implementation_GetCoreID(void)
{
    /* Available in every context and before StartOS (Table 7.3, SWS_Os_00625);
     * the answer has no side effect to ignore while interrupts are disabled. */
    Os_StackCheck();
    return OS_CORE_ID_0;
}

OS_MEMMAP StatusType Os_Implementation_ControlIdle(CoreIdType CoreID, IdleModeType IdleMode)
{
    const Os_ErrorParametersType arguments = {.service_ControlIdle = {CoreID, IdleMode}};
    StatusType status = Os_ServiceAccessStatus(OSServiceId_ControlIdle);
    Os_StackCheck();
    Os_BackendGuardService();
    if (status == E_OK) {
        if (Os_BackendServiceContext() == 0) {
            status = E_OS_CALLEVEL;
        } else if (IdleMode != IDLE_NO_HALT) {
            status = E_OS_ID;
        } else {
            /* The existing Windows virtual-core idle loop does not halt.
             * This target has one supported mode, already effective. CoreID
             * is intentionally unchecked as required for single-core OS. */
        }
    }
    return (Os_HookContext() == OS_HOOK_ERROR)
               ? status
               : Os_ErrorResult(OSServiceId_ControlIdle, status, &arguments);
}

OS_MEMMAP void Os_Implementation_ShutdownOS(StatusType Error)
{
    const StatusType status = Os_ServiceAccessStatus(OSServiceId_ShutdownOS);
    Os_StackCheck();
    if (status != E_OK) {
        return;
    }
    Os_BackendShutdown(Error);
}

OS_MEMMAP static StatusType implementation_GetTaskID(TaskRefType TaskID)
{
    Os_StackCheck();
    Os_BackendGuardService();
    if (TaskID == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendTaskId(TaskID);
}

OS_MEMMAP AppModeType Os_Implementation_GetActiveApplicationMode(void)
{
    const StatusType status = Os_ServiceAccessStatus(OSServiceId_GetActiveApplicationMode);
    Os_StackCheck();
    Os_BackendGuardService();
    if (status != E_OK) {
        return 0u;
    }
    return Os_BackendApplicationMode();
}

OS_MEMMAP ISRType Os_Implementation_GetISRID(void)
{
    const StatusType status = Os_ServiceAccessStatus(OSServiceId_GetISRID);
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 interrupt = Os_BackendCurrentInterrupt();
    Os_StackCheck();
    Os_BackendGuardService();
    if (status != E_OK) {
        return INVALID_ISR;
    }
    return ((stack != NULL) && (stack->role == 'S') && (interrupt < 32u) &&
            (interrupt != OS_KERNEL_YIELD_INTERRUPT) &&
            (interrupt != OS_CONTROLLED_TICK_INTERRUPT) && (Os_Config != NULL) &&
            ((Os_Config->category1_isrs & ((uint32)1u << interrupt)) == 0u))
               ? (ISRType)interrupt
               : INVALID_ISR;
}

OS_MEMMAP static StatusType implementation_GetTaskState(TaskType TaskID, TaskStateRefType State)
{
    Os_StackCheck();
    Os_BackendGuardService();
    if (State == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendState(TaskID, State);
}

OS_MEMMAP static StatusType implementation_ActivateTask(TaskType TaskID)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendActivate(TaskID);
}

OS_MEMMAP static StatusType implementation_TerminateTask(void)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendFinish();
}

OS_MEMMAP static StatusType implementation_ChainTask(TaskType TaskID)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendChain(TaskID);
}

OS_MEMMAP static StatusType implementation_GetResource(ResourceType ResID)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 1);
}

OS_MEMMAP static StatusType implementation_ReleaseResource(ResourceType ResID)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendResource(ResID, 0);
}

OS_MEMMAP static StatusType implementation_Schedule(void)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendSchedule();
}

OS_MEMMAP static StatusType implementation_WaitEvent(EventMaskType Mask)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendWait(Mask);
}

OS_MEMMAP static StatusType implementation_ClearEvent(EventMaskType Mask)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendClear(Mask);
}

OS_MEMMAP static StatusType implementation_SetEvent(TaskType TaskID, EventMaskType Mask)
{
    Os_StackCheck();
    Os_BackendGuardService();
    return Os_BackendEvent(TaskID, Mask, NULL);
}

OS_MEMMAP static StatusType implementation_GetEvent(TaskType TaskID, EventMaskRefType Event)
{
    Os_StackCheck();
    Os_BackendGuardService();
    if (Event == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendEvent(TaskID, 0u, Event);
}

OS_MEMMAP StatusType Os_TargetInspectActivation(TaskType id, Os_ActivationInfoType *info)
{
    Os_StackCheck();
    Os_BackendGuardService();
    if (info == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendInspect(id, info);
}

OS_MEMMAP StatusType Os_Implementation_GetTaskID(TaskRefType TaskID)
{
    const Os_ErrorParametersType arguments = {.service_GetTaskID = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetTaskID);
    if (status == E_OK) {
        status = implementation_GetTaskID(TaskID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetTaskID, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_GetTaskState(TaskType TaskID, TaskStateRefType State)
{
    const Os_ErrorParametersType arguments = {.service_GetTaskState = {TaskID, State}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetTaskState);
    if (status == E_OK) {
        status = implementation_GetTaskState(TaskID, State);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetTaskState, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_ActivateTask(TaskType TaskID)
{
    const Os_ErrorParametersType arguments = {.service_ActivateTask = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_ActivateTask);
    if (status == E_OK) {
        status = implementation_ActivateTask(TaskID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ActivateTask, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_TerminateTask(void)
{
    const Os_ErrorParametersType arguments = {.service_TerminateTask = {0u}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_TerminateTask);
    if (status == E_OK) {
        status = implementation_TerminateTask();
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_TerminateTask, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_ChainTask(TaskType TaskID)
{
    const Os_ErrorParametersType arguments = {.service_ChainTask = {TaskID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_ChainTask);
    if (status == E_OK) {
        status = implementation_ChainTask(TaskID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ChainTask, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_GetResource(ResourceType ResID)
{
    const Os_ErrorParametersType arguments = {.service_GetResource = {ResID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetResource);
    if (status == E_OK) {
        status = implementation_GetResource(ResID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetResource, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_ReleaseResource(ResourceType ResID)
{
    const Os_ErrorParametersType arguments = {.service_ReleaseResource = {ResID}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_ReleaseResource);
    if (status == E_OK) {
        status = implementation_ReleaseResource(ResID);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ReleaseResource, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_Schedule(void)
{
    const Os_ErrorParametersType arguments = {.service_Schedule = {0u}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_Schedule);
    if (status == E_OK) {
        status = implementation_Schedule();
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_Schedule, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_WaitEvent(EventMaskType Mask)
{
    const Os_ErrorParametersType arguments = {.service_WaitEvent = {Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_WaitEvent);
    if (status == E_OK) {
        status = implementation_WaitEvent(Mask);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_WaitEvent, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_ClearEvent(EventMaskType Mask)
{
    const Os_ErrorParametersType arguments = {.service_ClearEvent = {Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_ClearEvent);
    if (status == E_OK) {
        status = implementation_ClearEvent(Mask);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_ClearEvent, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_SetEvent(TaskType TaskID, EventMaskType Mask)
{
    const Os_ErrorParametersType arguments = {.service_SetEvent = {TaskID, Mask}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_SetEvent);
    if (status == E_OK) {
        status = implementation_SetEvent(TaskID, Mask);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_SetEvent, status, &arguments);
}

OS_MEMMAP StatusType Os_Implementation_GetEvent(TaskType TaskID, EventMaskRefType Event)
{
    const Os_ErrorParametersType arguments = {.service_GetEvent = {TaskID, Event}};
    StatusType status;
    Os_StackCheck();
    Os_BackendGuardService();
    status = Os_ServiceAccessStatus(OSServiceId_GetEvent);
    if (status == E_OK) {
        status = implementation_GetEvent(TaskID, Event);
    }
    if (Os_HookContext() == OS_HOOK_ERROR) {
        return status;
    }
    return Os_ErrorResult(OSServiceId_GetEvent, status, &arguments);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
