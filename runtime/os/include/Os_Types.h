/** @file Public AUTOSAR OS scalar types and constants for the single-core target. */
#ifndef AUTOSAR_OS_TYPES_H
#define AUTOSAR_OS_TYPES_H
#include <stddef.h>
#include <stdint.h>
#include "Std_Types.h"
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
/** SWS_Os_91007 application mode; this target uses mode bits 1 and 2, not multicore inheritance. */
typedef uint8 AppModeType;
/** OSEK OS 2.2.3 13.7.3 default application mode, always valid for StartOS;
 * the generated configuration requires the OsAppMode of this name (ECUC_Os_00022). */
#define OSDEFAULTAPPMODE                                                                           \
    ((AppModeType)1u) /**< OSEK OS 2.2.3 13.7.3 and ECUC_Os_00022: valid default startup mode. */
/* Public R24-11 constants; this selected target has one logical core. Their
 * declarations do not enable multicore startup or mode inheritance. */
#define DONOTCARE                                                                                  \
    UINT8_MAX /**< SWS_Os_91007: unspecified AppMode; multicore inheritance is not selected. */
#define TotalNumberOfCores 1u /**< SWS_Os_91002: one controlled logical core. */
/** OSEK OS 2.2.3 13.4.1 external-resource identifier below OS_MAX_RESOURCES. */
typedef uint8 ResourceType;
/** OSEK OS 2.2.3 13.5.1 bit set of Task events; zero contains no events. */
typedef uint32 EventMaskType;
/** OSEK OS 2.2.3 13.5.1 pointer to a caller-owned event-mask result. */
typedef EventMaskType *EventMaskRefType;
/* Shared with Rte_Os_Type.h; never narrow an external Counter identifier. */
/** Counter identifier used by SWS_Os_00383; uint32 is shared with Rte_Os_Type.h. */
typedef uint32 CounterType;
/** OSEK OS 2.2.3 13.6.1 Alarm identifier below OS_MAX_ALARMS. */
typedef uint8 AlarmType;
/* R24-11 public type definitions remain available independently of whether
 * their conditional runtime services are selected in this SC1 configuration. */
/** SWS_Os_00772 OS-Application identifier; no OS-Application runtime services selected here. */
typedef uint32 ApplicationType;
#define INVALID_OSAPPLICATION UINT32_MAX /**< SWS_Os_00772: invalid OS-Application identifier. */
/** SWS_Os_00773 application state; declaration does not enable OS-Application support. */
typedef uint8 ApplicationStateType;
/** SWS_Os_00774 pointer to application-state output owned by the caller. */
typedef ApplicationStateType *ApplicationStateRefType;
#define APPLICATION_ACCESSIBLE                                                                     \
    0u /**< SWS_Os_00773: application accessible state; runtime services not selected. */
#define APPLICATION_TERMINATED                                                                     \
    1u /**< SWS_Os_00773: application terminated state; runtime services not selected. */
/** SWS_Os_00775 trusted-function index; trusted-function services are not selected. */
typedef uint32 TrustedFunctionIndexType;
/** SWS_Os_00776 opaque parameter storage for a trusted function; no ownership transfer. */
typedef void *TrustedFunctionParameterRefType;
/** SWS_Os_00777 integral access-rights mask tested by OSMEMORY_IS_* macros. */
typedef uint32 AccessType;
/** Test the read property of AccessType (SWS_Os_00777).
 * @param[in] access Integral access-rights mask, evaluated once.
 * @return Nonzero when the selected access bit is set; zero otherwise.
 * Preconditions: integral AccessType-compatible argument.
 * Context: any expression context, including existing critical/exclusive areas.
 * Reentrancy: reentrant; no state is read or written.
 * Ownership: copies the mask value; no pointer is dereferenced or retained.
 */
#define OSMEMORY_IS_READABLE(access) (((access) & UINT32_C(1)) != 0u)
/** Test the write property of AccessType (SWS_Os_00777).
 * @param[in] access Integral access-rights mask, evaluated once.
 * @return Nonzero when the selected access bit is set; zero otherwise.
 * Preconditions: integral AccessType-compatible argument.
 * Context: any expression context, including existing critical/exclusive areas.
 * Reentrancy: reentrant; no state is read or written.
 * Ownership: copies the mask value; no pointer is dereferenced or retained.
 */
#define OSMEMORY_IS_WRITEABLE(access) (((access) & UINT32_C(2)) != 0u)
/** Test the execute property of AccessType (SWS_Os_00777).
 * @param[in] access Integral access-rights mask, evaluated once.
 * @return Nonzero when the selected access bit is set; zero otherwise.
 * Preconditions: integral AccessType-compatible argument.
 * Context: any expression context, including existing critical/exclusive areas.
 * Reentrancy: reentrant; no state is read or written.
 * Ownership: copies the mask value; no pointer is dereferenced or retained.
 */
#define OSMEMORY_IS_EXECUTABLE(access) (((access) & UINT32_C(4)) != 0u)
/** Test the stack-space property of AccessType (SWS_Os_00777).
 * @param[in] access Integral access-rights mask, evaluated once.
 * @return Nonzero when the selected access bit is set; zero otherwise.
 * Preconditions: integral AccessType-compatible argument.
 * Context: any expression context, including existing critical/exclusive areas.
 * Reentrancy: reentrant; no state is read or written.
 * Ownership: copies the mask value; no pointer is dereferenced or retained.
 */
#define OSMEMORY_IS_STACKSPACE(access) (((access) & UINT32_C(8)) != 0u)
/** SWS_Os_00778 object-access decision, ACCESS or NO_ACCESS. */
typedef uint8 ObjectAccessType;
#define ACCESS    1u /**< SWS_Os_00778: access to the object is permitted. */
#define NO_ACCESS 0u /**< SWS_Os_00778: access to the object is denied. */
/** SWS_Os_00779 kind of an OS object, represented by OBJECT_* constants. */
typedef uint8 ObjectTypeType;
#define OBJECT_TASK          0u /**< SWS_Os_00779: task object kind. */
#define OBJECT_ISR           1u /**< SWS_Os_00779: isr object kind. */
#define OBJECT_ALARM         2u /**< SWS_Os_00779: alarm object kind. */
#define OBJECT_RESOURCE      3u /**< SWS_Os_00779: resource object kind. */
#define OBJECT_COUNTER       4u /**< SWS_Os_00779: counter object kind. */
#define OBJECT_SCHEDULETABLE 5u /**< SWS_Os_00779: ScheduleTable object kind. */
/** SWS_Os_00780 native address of a memory region; does not own that region. */
typedef void *MemoryStartAddressType;
/** SWS_Os_00781 size of a memory region in bytes; native size_t preserves host address width. */
typedef size_t MemorySizeType;
/** SWS_Os_00782 logical ISR identifier; INVALID_ISR means no active Category 2 ISR. */
typedef uint8 ISRType;
#define INVALID_ISR  UINT8_MAX /**< SWS_Os_00782: no active Category 2 ISR. */
#define INVALID_TASK UINT8_MAX /**< OSEK OS 2.2.3 13.2.3.5: no currently running Task. */
/** SWS_Os_00787 protection decision; protection services are not selected in SC1. */
typedef uint8 ProtectionReturnType;
#define PRO_IGNORE                                                                                 \
    0u /**< SWS_Os_00787: ignore a protection error; protection services not selected. */
#define PRO_TERMINATETASKISR                                                                       \
    1u /**< SWS_Os_00787: terminate offending Task/ISR; protection not selected. */
#define PRO_TERMINATEAPPL                                                                          \
    2u /**< SWS_Os_00787: terminate offending application; protection not selected. */
#define PRO_SHUTDOWN 3u /**< SWS_Os_00787: request OS shutdown after a protection error. */
#define PRO_PREVENT_ARRIVAL_RATE                                                                   \
    4u /**< SWS_Os_00787: reject excessive arrival; protection not selected. */
/** SWS_Os_00788 OS-Application restart choice; restart services are not selected. */
typedef uint8 RestartType;
#define OS_OSAPPLICATION_RESTART                                                                   \
    1u /**< SWS_Os_00788: restart selection; OS-Application services not selected. */
/** SWS_Os_00789 unsigned physical conversion result; the conversion macro determines its unit. */
typedef uint64 PhysicalTimeType;
/** SWS_Os_00790 core identifier; this target exposes only OS_CORE_ID_0. */
typedef uint16 CoreIdType;
#define OS_CORE_ID_0 0u /**< SWS_Os_00790: identity of this target only logical core. */
#define OS_CORE_ID_MASTER                                                                          \
    OS_CORE_ID_0 /**< SWS_Os_00790: master core alias; this target uses core zero. */
/** SWS_Os_00791 spinlock identifier; declaration does not select multicore spinlocks. */
typedef uint16 SpinlockIdType;
#define INVALID_SPINLOCK                                                                           \
    0u /**< Zero invalid spinlock identifier; spinlock services are not selected. */
/** SWS_Os_00792 success indication for a spinlock attempt; spinlocks are not selected. */
typedef enum {
    TRYTOGETSPINLOCK_NOSUCCESS = 0, /**< Spinlock not acquired. */
    TRYTOGETSPINLOCK_SUCCESS = 1    /**< Spinlock acquired. */
} TryToGetSpinlockType;
/** SWS_Os_00793 idle mode; only IDLE_NO_HALT is supported by the controlled host. */
typedef uint8 IdleModeType;
#define IDLE_NO_HALT 0u /**< SWS_Os_00793: keep the controlled host core running during idle. */
/** SWS_Os_91000 peripheral-area identifier, with standard range 0 through 65534. */
typedef uint16 AreaIdType;
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
#define SCHEDULETABLE_WAITING 2u /**< SWS_Os_00784: waiting for synchronization, not selected. */
#define SCHEDULETABLE_RUNNING 3u /**< SWS_Os_00784: running unsynchronized table. */
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
#define E_OS_PROTECTION_MEMORY                                                                     \
    12u /**< AUTOSAR OS 7.15: memory protection failure; protection not selected. */
#define E_OS_STACKFAULT                                                                            \
    13u /**< AUTOSAR OS 7.15: native actor stack monitoring detected a fault.                      \
         */
/* R24-11 section 7.15 assigns numeric values to the implementation. These
 * public names do not select protection, spinlocks or multicore services. */
#define E_OS_CORE                                                                                  \
    14u /**< AUTOSAR OS 7.15: invalid core access; multicore services not selected.                \
         */
#define E_OS_NESTING_DEADLOCK                                                                      \
    15u /**< AUTOSAR OS 7.15: spinlock nesting deadlock; spinlocks not selected. */
#define E_OS_PROTECTION_LOCKED                                                                     \
    16u /**< AUTOSAR OS 7.15: protection lock-time failure; protection not selected. */
#define E_OS_SPINLOCK  17u /**< AUTOSAR OS 7.15: invalid spinlock use; spinlocks not selected. */
#define E_OS_SERVICEID 18u /**< AUTOSAR OS 7.15: unsupported service identity. */
#define E_OS_PROTECTION_EXCEPTION                                                                  \
    19u /**< AUTOSAR OS 7.15: protection exception; protection not selected. */
#define E_OS_INTERFERENCE_DEADLOCK                                                                 \
    20u /**< AUTOSAR OS 7.15: interference deadlock; spinlocks not selected. */
#define E_OS_PROTECTION_TIME                                                                       \
    21u /**< AUTOSAR OS 7.15: execution-time protection failure; protection not selected. */
#define E_OS_PROTECTION_ARRIVAL                                                                    \
    22u /**< AUTOSAR OS 7.15: arrival-rate protection failure; protection not selected. */
#define RUNNING   0u /**< OSEK OS 2.2.3 13.2.1: Task currently executing. */
#define WAITING   1u /**< OSEK OS 2.2.3 13.2.1: Extended Task waiting for an event. */
#define READY     2u /**< OSEK OS 2.2.3 13.2.1: Task ready for execution. */
#define SUSPENDED 3u /**< OSEK OS 2.2.3 13.2.1: Task has no admitted activation. */
#endif
