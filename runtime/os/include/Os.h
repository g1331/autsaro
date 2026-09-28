#ifndef AUTOSAR_EPIC4_OS_H
#define AUTOSAR_EPIC4_OS_H
#include <stdint.h>
typedef uint8_t StatusType;
typedef uint8_t TaskType;
typedef uint8_t TaskStateType;
typedef TaskStateType *TaskStateRefType;
typedef uint8_t AppModeType;
#define E_OK 0u
#define E_OS_ACCESS 1u
#define E_OS_CALLEVEL 2u
#define E_OS_ID 3u
#define E_OS_LIMIT 4u
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
 * @return E_OS_CALLEVEL or E_OS_STATE on rejection; success enters the next task.
 */
StatusType TerminateTask(void);
void StartupHook(void);
void ShutdownHook(StatusType Error);
#endif
