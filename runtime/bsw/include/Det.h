/** @file R24-11 Default Error Tracer; no DLT, persistent log or version API. */
#ifndef DET_H
#define DET_H
#include "Std_Types.h"

/** SWS_Det_00180/00181/00184: synchronous, reentrant report notification.
 * Callouts receive the original identity and should return E_OK. Det ignores
 * their return values, so a misbehaving callout cannot suppress later ones.
 */
typedef Std_ReturnType (*Det_ErrorHookType)(uint16 ModuleId,
                                            uint8 InstanceId,
                                            uint8 ApiId,
                                            uint8 ErrorId);
/** SWS_Det_00210: immutable configuration; concurrent Init calls are not allowed.
 * Arrays and their targets remain alive until all reports using them finish.
 */
typedef struct {
    const Det_ErrorHookType *error_hooks; /**< Ordered development hooks; NULL if count is 0. */
    uint16 error_hook_count;              /**< Number of development hooks, 0..65535. */
    const Det_ErrorHookType *runtime_callouts; /**< Ordered runtime callouts; NULL if count is 0. */
    uint16 runtime_callout_count;              /**< Number of runtime callouts, 0..65535. */
} Det_ConfigType;

/** Reset and select the reporting configuration (SWS_Det_00008/00020).
 * @param[in] ConfigPtr Caller-owned immutable configuration; NULL selects no hooks.
 * @par Return
 * None. An inconsistent hook list halts the controlled ECU.
 * Preconditions: Other Init calls are serialized; prior configurations outlive their reports.
 * Context: Lifecycle task, outside every exclusive area.
 * Reentrancy: Non-reentrant.
 * Ownership: Keeps ConfigPtr and its referenced arrays; no copies or allocation.
 */
void Det_Init(const Det_ConfigType *ConfigPtr);
/** Start reporting; no startup-dependent storage is selected (SWS_Det_00025).
 * @par Return
 * None.
 * Preconditions: Init completed; this selection performs no additional action.
 * Context: Lifecycle task, outside every exclusive area.
 * Reentrancy: Non-reentrant.
 * Ownership: No parameters or retained storage.
 */
void Det_Start(void);
/** Notify every runtime callout in order (SWS_Det_01001/00503).
 * @param[in] ModuleId Calling module identifier, 0..65535.
 * @param[in] InstanceId Calling instance, 0..255; 0 for single-instance modules.
 * @param[in] ApiId Calling API identifier, 0..255.
 * @param[in] ErrorId Runtime error identifier, 0..255.
 * @return Always E_OK; before Init, returns without notifying any callout.
 * Preconditions: Callouts are reentrant; retained configurations remain immutable.
 * Context: Caller task or callback, outside every exclusive area; no MCU ISR claim.
 * Reentrancy: Concurrent and nested reports are supported; callouts run outside DET_STATE.
 * Ownership: No parameter is retained; the active configuration outlives the report.
 */
Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId,
                                      uint8 InstanceId,
                                      uint8 ApiId,
                                      uint8 ErrorId);
/** Notify development hooks, then halt execution (SWS_Det_00009/00026/00208).
 * @param[in] ModuleId Calling module identifier, 0..65535.
 * @param[in] InstanceId Calling instance, 0..255; 0 for single-instance modules.
 * @param[in] ApiId Calling API identifier, 0..255.
 * @param[in] ErrorId Development error identifier, 0..255.
 * @return Never returns; Std_ReturnType is required by the standard signature.
 * Preconditions: Before Init, halts without hooks; retained configurations stay immutable.
 * Context: Caller task or callback, outside every exclusive area; no MCU ISR claim.
 * Reentrancy: Concurrent reports are independent; recursive development reports halt
 * without invoking the hooks again on that thread.
 * Ownership: No parameter is retained; hooks run outside DET_STATE before host halt.
 */
Std_ReturnType Det_ReportError(uint16 ModuleId, uint8 InstanceId, uint8 ApiId, uint8 ErrorId);
#endif
