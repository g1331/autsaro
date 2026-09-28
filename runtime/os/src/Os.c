#include "Os_Target.h"
#include "Os_Backend.h"
const Os_TargetConfig *Os_Config;
StatusType Os_TargetPrepare(const Os_TargetConfig *config) {
    size_t i, j;
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
        if (task->id >= OS_MAX_TASKS) {
            return E_OS_ID;
        }
        if ((task->name == NULL) || (task->entry == NULL) || (task->priority == 0u) ||
            (task->priority > OS_MAX_PRIORITY) || (task->autostart_modes > 3u) ||
            (task->kind > OS_EXTENDED_TASK) || (task->activation_limit == 0u) ||
            (task->activation_limit > OS_MAX_ACTIVATIONS) ||
            ((task->kind == OS_EXTENDED_TASK) && (task->activation_limit != 1u))) {
            return E_OS_VALUE;
        }
        for (j = 0u; j < i; ++j) {
            if (task->id == config->tasks[j].id) {
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
StatusType Os_TargetInspectActivation(TaskType id, Os_ActivationInfo *info) {
    Os_StackCheck();
    Os_BackendGuardService();
    if (info == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    return Os_BackendInspect(id, info);
}
