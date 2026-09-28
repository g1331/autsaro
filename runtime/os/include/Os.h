#ifndef AUTOSAR_EPIC4_OS_H
#define AUTOSAR_EPIC4_OS_H
#include <stdint.h>
typedef uint8_t StatusType;
typedef uint8_t TaskType;
typedef uint8_t TaskStateType;
typedef TaskStateType *TaskStateRefType;
typedef uint8_t AppModeType;
typedef uint8_t ResourceType;
typedef uint32_t EventMaskType;
typedef EventMaskType *EventMaskRefType;
#define RES_SCHEDULER 7u
#define E_OK 0u
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
void StartupHook(void);
void ShutdownHook(StatusType Error);
#endif
