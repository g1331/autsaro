#ifndef AUTOSAR_EPIC4_OS_TARGET_H
#define AUTOSAR_EPIC4_OS_TARGET_H
#include "Os.h"
#include <stddef.h>
#define OS_MAX_TASKS 16u
#define OS_MAX_PRIORITY 30u
typedef struct {
    TaskType id;
    const char *name;
    void (*entry)(void);
    uint8_t priority;
    uint8_t autostart_modes;
} Os_TaskConfig;
typedef struct {
    const Os_TaskConfig *tasks;
    size_t task_count;
    uint32_t host_stack_reserve;
} Os_TargetConfig;
/** Prepare one process-local target before StartOS.
 * @param config Static configuration retained for the lifetime of the process.
 * @return E_OK or a standard validation error; no threads created on rejection.
 */
StatusType Os_TargetPrepare(const Os_TargetConfig *config);
/** Bounded memory trace; does not perform host I/O.
 * @param marker Nonzero character to append.
 */
void Os_TargetTrace(char marker);
/** Whether target services may accept input/tick (Ready and not closing).
 * @return Nonzero only after successful StartupHook.
 */
int Os_TargetReady(void);
#endif
