/** @file Public AUTOSAR OS scalar types and constants for the single-core target. */
#ifndef AUTOSAR_OS_TYPES_H
#define AUTOSAR_OS_TYPES_H
#include "Os_Version.h"
#include <stddef.h>
#include <stdint.h>
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Os_Types.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Types.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#include "Rte_Os_Type.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "Os_Types.h -> Rte_Os_Type.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Types.h -> Rte_Os_Type.h: incompatible AUTOSAR release"
#endif
/** OSEK OS 2.2.3 13.1 service result; E_OK or a named E_OS_* status. */
typedef uint8 StatusType;
/** OSEK OS 2.2.3 13.2.1 Task identifier; selected target uses IDs below OS_MAX_TASKS. */
typedef uint8 TaskType;
/** OSEK OS 2.2.3 13.2.1 pointer to caller-owned Task identifier output. */
typedef TaskType *TaskRefType;
/** OSEK OS 2.2.3 13.2.1 Task state: RUNNING, WAITING, READY or SUSPENDED. */
typedef uint8 TaskStateType;
/** OSEK OS 2.2.3 13.2.1 pointer to caller-owned Task-state output. */
typedef TaskStateType *TaskStateRefType;
/** OSEK OS 2.2.3 13.7.2 application mode; this target uses mode bits 1 and 2. */
typedef uint8 AppModeType;
/** OSEK OS 2.2.3 13.7.3 default application mode, always valid for StartOS;
 * the generated configuration requires the OsAppMode of this name (ECUC_Os_00022). */
#define OSDEFAULTAPPMODE                                                                           \
    ((AppModeType)1u) /**< OSEK OS 2.2.3 13.7.3 and ECUC_Os_00022: valid default startup mode. */
/** OSEK OS 2.2.3 13.4.1 external-resource identifier below OS_MAX_RESOURCES. */
typedef uint8 ResourceType;
/** OSEK OS 2.2.3 13.5.1 bit set of Task events; zero contains no events. */
typedef uint32 EventMaskType;
/** OSEK OS 2.2.3 13.5.1 pointer to a caller-owned event-mask result. */
typedef EventMaskType *EventMaskRefType;
/** OSEK OS 2.2.3 13.6.1 Alarm identifier below OS_MAX_ALARMS. */
typedef uint8 AlarmType;
/** SWS_Os_00782 logical ISR identifier; INVALID_ISR means no active Category 2 ISR. */
typedef uint8 ISRType;
#define INVALID_ISR ((ISRType)UINT8_MAX) /**< SWS_Os_00782: no active Category 2 ISR. */
#define INVALID_TASK                                                                               \
    ((TaskType)UINT8_MAX) /**< OSEK OS 2.2.3 13.2.3.5: no currently running Task. */
/** SWS_Os_00789 unsigned physical conversion result; the conversion macro determines its unit. */
typedef uint64 PhysicalTimeType;
/** SWS_Os_00790 core identifier; this target exposes only OS_CORE_ID_0. */
typedef uint16 CoreIdType;
#define OS_CORE_ID_0 0u /**< SWS_Os_00790: identity of this target only logical core. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define OS_CORE_ID_MASTER                                                                          \
    OS_CORE_ID_0 /**< SWS_Os_00790: master core alias; this target uses core zero. */
/** SWS_Os_00793 idle mode; only IDLE_NO_HALT is supported by the controlled host. */
typedef uint8 IdleModeType;
#define IDLE_NO_HALT 0u /**< SWS_Os_00793: keep the controlled host core running during idle. */
/** OSEK OS 2.2.3 13.8.1 identity of a failed service exposed through ErrorHook accessors. */
typedef uint8 OSServiceIdType;
/** SWS_Os_00783 configured ScheduleTable identifier below OS_MAX_SCHEDULE_TABLES. */
typedef uint8 ScheduleTableType;
/** SWS_Os_00784 ScheduleTable state represented by SCHEDULETABLE_* constants. */
typedef uint8 ScheduleTableStatusType;
/** SWS_Os_00785 pointer to caller-owned ScheduleTable-state output. */
typedef ScheduleTableStatusType *ScheduleTableStatusRefType;
#define SCHEDULETABLE_STOPPED 0u /**< SWS_Os_00784: inactive table. */
#define SCHEDULETABLE_NEXT    1u /**< SWS_Os_00784: reserved successor table. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define SCHEDULETABLE_WAITING 2u /**< SWS_Os_00784: waiting for synchronization, not selected. */
#define SCHEDULETABLE_RUNNING 3u /**< SWS_Os_00784: running unsynchronized table. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define SCHEDULETABLE_RUNNING_AND_SYNCHRONOUS                                                      \
    4u /**< SWS_Os_00784: running synchronized table, not selected. */
/* Automotive Counter values are independent of FreeRTOS's 32-bit tick ABI. */
/** OSEK OS 2.2.3 13.6.1 Counter ticks; uint64 avoids coupling to the FreeRTOS 32-bit tick ABI. */
typedef uint64 TickType;
/** OSEK OS 2.2.3 13.6.1 pointer to caller-owned Counter tick storage. */
typedef TickType *TickRefType;
/** OSEK OS 2.2.3 13.6.1 immutable Alarm Counter limits and time-base properties. */
typedef struct {
    TickType maxallowedvalue; /**< Largest Counter tick value (OSEK OS 2.2.3 13.6.1). */
    TickType ticksperbase;    /**< Counter ticks per time base (OSEK OS 2.2.3 13.6.1). */
    TickType mincycle; /**< Minimum cyclic Alarm period in Counter ticks (OSEK OS 2.2.3 13.6.1). */
} AlarmBaseType;
/** OSEK OS 2.2.3 13.6.1 pointer to caller-owned Alarm Counter property output. */
typedef AlarmBaseType *AlarmBaseRefType;
#define RES_SCHEDULER                                                                              \
    7u /**< OSEK OS 2.2.3 13.4.4: scheduler resource, reserved when configured.                    \
        */
#define E_OK          0x00u /**< OSEK OS 2.2.3 13.1: successful service result. */
#define E_OS_ACCESS   1u    /**< OSEK OS 2.2.3 13.1: object/service access not permitted. */
#define E_OS_CALLEVEL 2u    /**< OSEK OS 2.2.3 13.1: service called from an invalid context. */
#define E_OS_ID       3u    /**< OSEK OS 2.2.3 13.1: invalid configured-object identifier. */
#define E_OS_LIMIT    4u    /**< OSEK OS 2.2.3 13.1: configured capacity exhausted. */
#define E_OS_NOFUNC   5u /**< OSEK OS 2.2.3 13.1: no active function/object state to operate on. */
#define E_OS_RESOURCE 6u /**< OSEK OS 2.2.3 13.1: caller still holds an external resource. */
#define E_OS_STATE    7u /**< OSEK OS 2.2.3 13.1: incompatible object/system state. */
#define E_OS_VALUE    8u /**< OSEK OS 2.2.3 13.1: argument outside the accepted range. */
#define E_OS_DISABLEDINT                                                                           \
    9u /**< AUTOSAR OS 7.15: caller has disabled service access with interrupt masks. */
#define E_OS_ILLEGAL_ADDRESS                                                                       \
    10u /**< SWS_Os_00051: invalid output storage; this host checks NULL pointers. */
#define E_OS_MISSINGEND                                                                            \
    11u /**< AUTOSAR OS 7.15: Task returned without TerminateTask/ChainTask.                       \
         */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_PROTECTION_MEMORY                                                                     \
    12u /**< AUTOSAR OS 7.15: memory protection failure; protection not selected. */
#define E_OS_STACKFAULT                                                                            \
    13u /**< AUTOSAR OS 7.15: native actor stack monitoring detected a fault.                      \
         */
/* R24-11 section 7.15 assigns numeric values to the implementation. These
 * public names do not select protection, spinlocks or multicore services. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_CORE                                                                                  \
    14u /**< AUTOSAR OS 7.15: invalid core access; multicore services not selected.                \
         */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_NESTING_DEADLOCK                                                                      \
    15u /**< AUTOSAR OS 7.15: spinlock nesting deadlock; spinlocks not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_PROTECTION_LOCKED                                                                     \
    16u /**< AUTOSAR OS 7.15: protection lock-time failure; protection not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_SPINLOCK 17u /**< AUTOSAR OS 7.15: invalid spinlock use; spinlocks not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_SERVICEID 18u /**< AUTOSAR OS 7.15: unsupported service identity. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_PROTECTION_EXCEPTION                                                                  \
    19u /**< AUTOSAR OS 7.15: protection exception; protection not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_INTERFERENCE_DEADLOCK                                                                 \
    20u /**< AUTOSAR OS 7.15: interference deadlock; spinlocks not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_PROTECTION_TIME                                                                       \
    21u /**< AUTOSAR OS 7.15: execution-time protection failure; protection not selected. */
/* MR12 RULE 2.5 VIOLATION: published OS status and state constants retain their preprocessing
 * representation. */
#define E_OS_PROTECTION_ARRIVAL                                                                    \
    22u /**< AUTOSAR OS 7.15: arrival-rate protection failure; protection not selected. */
#define RUNNING   0u /**< OSEK OS 2.2.3 13.2.1: Task currently executing. */
#define WAITING   1u /**< OSEK OS 2.2.3 13.2.1: Extended Task waiting for an event. */
#define READY     2u /**< OSEK OS 2.2.3 13.2.1: Task ready for execution. */
#define SUSPENDED 3u /**< OSEK OS 2.2.3 13.2.1: Task has no admitted activation. */
#endif
