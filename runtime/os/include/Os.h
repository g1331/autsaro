#ifndef AUTOSAR_EPIC4_OS_H
#define AUTOSAR_EPIC4_OS_H
#include <stddef.h>
#include <stdint.h>
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
#define E_OS_ILLEGAL_ADDRESS 10u
#define E_OS_STACKFAULT 13u
/** Start a NONE-synchronized table at a positive relative delay.
 * @param ScheduleTableID Configured table identifier.
 * @param Offset Delay; Offset plus initial offset must fit counter maximum.
 * @return E_OK or E_OS_ID/VALUE/STATE/CALLEVEL; refusal preserves state.
 */
StatusType StartScheduleTableRel(ScheduleTableType ScheduleTableID, TickType Offset);
/** Start at the next occurrence of an absolute counter value.
 * @param ScheduleTableID Configured table identifier.
 * @param Start Value within counter maximum; equality waits a full wrap.
 * @return E_OK or E_OS_ID/VALUE/STATE/CALLEVEL; refusal preserves state.
 */
StatusType StartScheduleTableAbs(ScheduleTableType ScheduleTableID, TickType Start);
/** Stop a running or reserved NEXT table and detach its successor.
 * @param ScheduleTableID Configured table identifier.
 * @return E_OK or E_OS_ID/NOFUNC/CALLEVEL.
 */
StatusType StopScheduleTable(ScheduleTableType ScheduleTableID);
/** Reserve a stopped table to follow the running source's final delay.
 * @param ScheduleTableID_From Running source table.
 * @param ScheduleTableID_To Stopped destination on the same counter/strategy.
 * @return E_OK or E_OS_ID/NOFUNC/STATE/CALLEVEL; refusal preserves both.
 */
StatusType NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                             ScheduleTableType ScheduleTableID_To);
/** Copy the table's current state within the existing OS critical boundary.
 * @param ScheduleTableID Configured table identifier.
 * @param ScheduleStatus Nonnull caller-owned output, unchanged on refusal.
 * @return E_OK or E_OS_ID/ILLEGAL_ADDRESS/CALLEVEL.
 */
StatusType GetScheduleTableStatus(ScheduleTableType ScheduleTableID,
                                  ScheduleTableStatusRefType ScheduleStatus);
#define RUNNING 0u
#define WAITING 1u
#define READY 2u
#define SUSPENDED 3u
/** Start the configured OS in mode; successful startup never returns.
 * @param Mode Static application mode (1 or 2).
 */
void StartOS(AppModeType Mode);
/** Close irreversibly; invoke ShutdownHook on the established control stack,
 * or on the startup caller before control resources exist.
 * @param Error Standard shutdown reason.
 */
void ShutdownOS(StatusType Error);
/** Read the automotive task selected by the actual kernel.
 * @param TaskID Nonnull caller-owned output; INVALID_TASK if none is selected.
 * @return E_OK, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS; refusal preserves output.
 */
StatusType GetTaskID(TaskRefType TaskID);
/** Read the application mode selected by StartOS, including in global hooks.
 * @return Selected mode; zero before StartOS has selected a mode.
 */
AppModeType GetActiveApplicationMode(void);
/** Query actual backend task state.
 * @param TaskID Configured task identifier.
 * @param State Output state, must not be null.
 * @return E_OK, E_OS_ID, E_OS_CALLEVEL, E_OS_ILLEGAL_ADDRESS or E_OS_STATE.
 */
StatusType GetTaskState(TaskType TaskID, TaskStateRefType State);
/** Accept one activation request in Task or Category 2 ISR context.
 * @param TaskID Configured task identifier.
 * @return E_OK, E_OS_ID, E_OS_LIMIT or E_OS_CALLEVEL; rejection preserves requests.
 */
StatusType ActivateTask(TaskType TaskID);
/** Complete the current activation. Success never returns to the application.
 * @return E_OS_CALLEVEL, E_OS_RESOURCE or E_OS_STATE on rejection; success never returns.
 */
StatusType TerminateTask(void);
/** Complete this activation and accept a sequential successor atomically.
 * @param TaskID Configured successor; self-chain does not consume extra capacity.
 * @return E_OS_ID, E_OS_LIMIT, E_OS_RESOURCE or E_OS_CALLEVEL on rejection.
 * Successful calls never return to the completed application frame.
 */
StatusType ChainTask(TaskType TaskID);
/** Acquire a configured external resource using its static priority ceiling.
 * @param ResID Resource accessible to this task.
 * @return E_OK, E_OS_ID, E_OS_ACCESS or E_OS_CALLEVEL; no blocking acquisition.
 */
StatusType GetResource(ResourceType ResID);
/** Release the last external resource acquired by this task.
 * @param ResID Last owned resource, in LIFO order.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC or E_OS_CALLEVEL.
 */
StatusType ReleaseResource(ResourceType ResID);
/** Explicitly release/reacquire the current task's internal resource.
 * @return E_OK, E_OS_CALLEVEL or E_OS_RESOURCE; no effect for a FULL task without an internal
 * resource.
 */
StatusType Schedule(void);
/** Wait until any requested stored event is set; the current instance is retained.
 * @param Mask Event predicate, including zero for an indefinite wait.
 * @return E_OK, E_OS_ACCESS, E_OS_RESOURCE or E_OS_CALLEVEL.
 */
StatusType WaitEvent(EventMaskType Mask);
/** Clear stored bits owned by the current Extended task.
 * @param Mask Bits to clear.
 * @return E_OK, E_OS_ACCESS or E_OS_CALLEVEL.
 */
StatusType ClearEvent(EventMaskType Mask);
/** Set stored event bits of an active Extended task; repeated setting merges bits.
 * @param TaskID Extended task identifier.
 * @param Mask Bits to publish.
 * @return E_OK, E_OS_ID, E_OS_ACCESS, E_OS_STATE or E_OS_CALLEVEL.
 */
#define SetEvent Os_SetEvent
StatusType Os_SetEvent(TaskType TaskID, EventMaskType Mask);
/** Read the stored events of an active Extended task.
 * @param TaskID Extended task identifier.
 * @param Event Caller-owned non-null output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ACCESS, E_OS_STATE, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 */
StatusType GetEvent(TaskType TaskID, EventMaskRefType Event);
/** Increment a software Counter and execute due Alarm actions.
 * @param CounterID Configured software Counter.
 * @return E_OK, E_OS_ID or E_OS_CALLEVEL; action errors invoke configured ErrorHook.
 */
StatusType IncrementCounter(CounterType CounterID);
/** Read a Counter's current modular value.
 * @param CounterID Configured Counter.
 * @param Value Non-null output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 */
StatusType GetCounterValue(CounterType CounterID, TickRefType Value);
/** Read a modular elapsed value and update the prior sample.
 * @param CounterID Configured Counter.
 * @param Value Prior sample on input, current value on success.
 * @param ElapsedValue Difference within one modulus; multiple wraps are not detectable.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 */
StatusType GetElapsedValue(CounterType CounterID, TickRefType Value, TickRefType ElapsedValue);
/** Read immutable Alarm Counter properties.
 * @param AlarmID Configured Alarm.
 * @param Info Non-null output.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 */
StatusType GetAlarmBase(AlarmType AlarmID, AlarmBaseRefType Info);
/** Read ticks remaining until the active Alarm expires.
 * @param AlarmID Configured Alarm.
 * @param Tick Non-null output, unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC, E_OS_ILLEGAL_ADDRESS or E_OS_CALLEVEL.
 */
StatusType GetAlarm(AlarmType AlarmID, TickRefType Tick);
/** Start a relative Alarm.
 * @param AlarmID Configured Alarm.
 * @param Increment Positive distance within the Counter maximum.
 * @param Cycle Zero for one-shot, or a valid cycle.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_STATE or E_OS_CALLEVEL.
 */
StatusType SetRelAlarm(AlarmType AlarmID, TickType Increment, TickType Cycle);
/** Start an absolute Alarm; the current value expires only after a complete modulus.
 * @param AlarmID Configured Alarm.
 * @param Start Absolute value within the Counter maximum.
 * @param Cycle Zero for one-shot, or a valid cycle.
 * @return E_OK, E_OS_ID, E_OS_VALUE, E_OS_STATE or E_OS_CALLEVEL.
 */
StatusType SetAbsAlarm(AlarmType AlarmID, TickType Start, TickType Cycle);
/** Cancel an active Alarm.
 * @param AlarmID Configured Alarm.
 * @return E_OK, E_OS_ID, E_OS_NOFUNC or E_OS_CALLEVEL.
 */
StatusType CancelAlarm(AlarmType AlarmID);
void StartupHook(void);
void ShutdownHook(StatusType Error);
#endif
