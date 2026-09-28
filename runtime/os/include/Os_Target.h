#ifndef AUTOSAR_EPIC4_OS_TARGET_H
#define AUTOSAR_EPIC4_OS_TARGET_H
#include "Os.h"
#include <stddef.h>
#define OS_MAX_TASKS 16u
#define OS_MAX_PRIORITY 30u
#define OS_MAX_ACTIVATIONS 32u
#define OS_BASIC_TASK 0u
#define OS_EXTENDED_TASK 1u
typedef struct {
    TaskType id;
    const char *name;
    void (*entry)(void);
    uint8_t priority;
    uint8_t autostart_modes;
    uint8_t kind;
    uint8_t activation_limit;
} Os_TaskConfig;
typedef struct {
    TaskStateType state;
    unsigned count;
    /* Live FIFO order stamps; atomic rebasing preserves order, not lifetime IDs. */
    uint64_t requests[OS_MAX_ACTIVATIONS];
    uint64_t kernel_sequence;
} Os_ActivationInfo;
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
/** Inspect a stable request/state snapshot in Task or Category 2 ISR context.
 * @param id Configured Task identifier.
 * @param info Caller-owned output; unchanged on rejection.
 * @return E_OK, E_OS_ID, E_OS_ILLEGAL_ADDRESS, E_OS_CALLEVEL or E_OS_STATE.
 */
StatusType Os_TargetInspectActivation(TaskType id, Os_ActivationInfo *info);
#ifdef OS_ACTIVATION_TESTS
void Os_TestObserve(void);
void Os_TargetTestSequence(uint64_t sequence);
void Os_TestBeforeActivationLock(void);
void Os_TestAtActivationAdmission(unsigned point);
void Os_TestBeforeClose(void);
unsigned Os_TargetTestPending(TaskType id);
#endif
#endif
