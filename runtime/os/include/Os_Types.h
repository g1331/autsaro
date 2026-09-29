#ifndef AUTOSAR_EPIC4_OS_TYPES_H
#define AUTOSAR_EPIC4_OS_TYPES_H
#include <stddef.h>
#include <stdint.h>
#include "Std_Types.h"
typedef uint8_t StatusType;
typedef uint8_t TaskType;
typedef TaskType *TaskRefType;
typedef uint8_t TaskStateType;
typedef TaskStateType *TaskStateRefType;
typedef uint8_t AppModeType;
typedef uint8_t ResourceType;
typedef uint32_t EventMaskType;
typedef EventMaskType *EventMaskRefType;
typedef uint8_t CounterType;
typedef uint8_t AlarmType;
/* R24-11 public type definitions remain available independently of whether
 * their conditional runtime services are selected in this SC1 configuration. */
typedef uint32_t ApplicationType;
#define INVALID_OSAPPLICATION UINT32_MAX
typedef uint8_t ApplicationStateType;
typedef ApplicationStateType *ApplicationStateRefType;
#define APPLICATION_ACCESSIBLE 0u
#define APPLICATION_TERMINATED 1u
typedef uint32_t TrustedFunctionIndexType;
typedef void *TrustedFunctionParameterRefType;
typedef uint32_t AccessType;
#define OSMEMORY_IS_READABLE(access) (((access) & UINT32_C(1)) != 0u)
#define OSMEMORY_IS_WRITEABLE(access) (((access) & UINT32_C(2)) != 0u)
#define OSMEMORY_IS_EXECUTABLE(access) (((access) & UINT32_C(4)) != 0u)
#define OSMEMORY_IS_STACKSPACE(access) (((access) & UINT32_C(8)) != 0u)
typedef uint8_t ObjectAccessType;
#define ACCESS 1u
#define NO_ACCESS 0u
typedef uint8_t ObjectTypeType;
#define OBJECT_TASK 0u
#define OBJECT_ISR 1u
#define OBJECT_ALARM 2u
#define OBJECT_RESOURCE 3u
#define OBJECT_COUNTER 4u
#define OBJECT_SCHEDULETABLE 5u
typedef void *MemoryStartAddressType;
typedef size_t MemorySizeType;
typedef uint8_t ISRType;
#define INVALID_ISR UINT8_MAX
#define INVALID_TASK UINT8_MAX
typedef uint8_t ProtectionReturnType;
#define PRO_IGNORE 0u
#define PRO_TERMINATETASKISR 1u
#define PRO_TERMINATEAPPL 2u
#define PRO_SHUTDOWN 3u
#define PRO_PREVENT_ARRIVAL_RATE 4u
typedef uint8_t RestartType;
#define OS_OSAPPLICATION_RESTART 1u
typedef uint64_t PhysicalTimeType;
typedef uint16_t CoreIdType;
#define OS_CORE_ID_0 0u
#define OS_CORE_ID_MASTER OS_CORE_ID_0
typedef uint16_t SpinlockIdType;
#define INVALID_SPINLOCK 0u
typedef enum { TRYTOGETSPINLOCK_NOSUCCESS = 0, TRYTOGETSPINLOCK_SUCCESS = 1 } TryToGetSpinlockType;
typedef uint8_t IdleModeType;
#define IDLE_NO_HALT 0u
typedef uint16_t AreaIdType;
typedef uint8_t OSServiceIdType;
typedef uint8_t ScheduleTableType;
typedef uint8_t ScheduleTableStatusType;
typedef ScheduleTableStatusType *ScheduleTableStatusRefType;
#define SCHEDULETABLE_STOPPED 0u
#define SCHEDULETABLE_NEXT 1u
#define SCHEDULETABLE_WAITING 2u
#define SCHEDULETABLE_RUNNING 3u
#define SCHEDULETABLE_RUNNING_AND_SYNCHRONOUS 4u
/* Automotive Counter values are independent of FreeRTOS's 32-bit tick ABI. */
typedef uint64_t TickType;
typedef TickType *TickRefType;
typedef struct {
    TickType maxallowedvalue;
    TickType ticksperbase;
    TickType mincycle;
} AlarmBaseType;
typedef AlarmBaseType *AlarmBaseRefType;
#define RES_SCHEDULER 7u
#define E_OK 0x00u
#define E_OS_ACCESS 1u
#define E_OS_CALLEVEL 2u
#define E_OS_ID 3u
#define E_OS_LIMIT 4u
#define E_OS_NOFUNC 5u
#define E_OS_RESOURCE 6u
#define E_OS_STATE 7u
#define E_OS_VALUE 8u
#define E_OS_DISABLEDINT 9u
#define E_OS_ILLEGAL_ADDRESS 10u
#define E_OS_MISSINGEND 11u
#define E_OS_STACKFAULT 13u
#define RUNNING 0u
#define WAITING 1u
#define READY 2u
#define SUSPENDED 3u
#endif
