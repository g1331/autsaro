#ifndef AUTOSAR_EPIC4_FREERTOS_CONFIG_H
#define AUTOSAR_EPIC4_FREERTOS_CONFIG_H
#define configUSE_PREEMPTION 1
#define configUSE_TIME_SLICING 0
#define configUSE_PORT_OPTIMISED_TASK_SELECTION 0
#define configUSE_IDLE_HOOK 1
#define configUSE_TICK_HOOK 0
#define configTICK_RATE_HZ 1000
#define configMAX_PRIORITIES 32
#define configMINIMAL_STACK_SIZE 512
#define configMAX_TASK_NAME_LEN 16
#define configTICK_TYPE_WIDTH_IN_BITS TICK_TYPE_WIDTH_32_BITS
#define configSUPPORT_STATIC_ALLOCATION 1
#define configSUPPORT_DYNAMIC_ALLOCATION 0
#define configUSE_TIMERS 0
#define configUSE_MUTEXES 0
#define configUSE_TASK_NOTIFICATIONS 1
#define configTASK_NOTIFICATION_ARRAY_ENTRIES 1
#define configUSE_TRACE_FACILITY 1
#define configCHECK_FOR_STACK_OVERFLOW 0
#define configUSE_CO_ROUTINES 0
#define INCLUDE_vTaskSuspend 1
#define INCLUDE_vTaskPrioritySet 1
#define INCLUDE_uxTaskPriorityGet 1
#define INCLUDE_vTaskDelay 0
#define INCLUDE_vTaskDelete 0
#define INCLUDE_xTaskGetCurrentTaskHandle 1
#define INCLUDE_eTaskGetState 1
#define INCLUDE_xTaskGetHandle 1
void Os_BackendAssert(const char *file, int line);
#define configASSERT(x)                                                                            \
    do {                                                                                           \
        if (!(x))                                                                                  \
            Os_BackendAssert(__FILE__, __LINE__);                                                  \
    } while (0)
#endif
