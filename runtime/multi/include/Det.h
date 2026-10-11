/** @file R24-11 Default Error Tracer public interface. */
#ifndef DET_H
#define DET_H
#include "Std_Types.h"

/** Configured hooks receive the original report, in configuration order.
 * Their return values cannot suppress later hooks or change the report result.
 * Hooks must support the caller's context and concurrent/reentrant reporting.
 */
typedef Std_ReturnType (*Det_ErrorHookType)(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                                            uint8_t ErrorId);
/** Vendor-defined immutable configuration. Arrays and their targets must remain
 * alive throughout reporting. Initialization is serialized with all reports.
 * A NULL ConfigPtr selects no hooks; no persistent storage or DLT is selected.
 */
typedef struct {
    const Det_ErrorHookType *error_hooks;
    uint16_t error_hook_count;
    const Det_ErrorHookType *runtime_callouts;
    uint16_t runtime_callout_count;
} Det_ConfigType;

/** Initialize reporting; a malformed hook list halts the controlled ECU. */
void Det_Init(const Det_ConfigType *ConfigPtr);
/** No startup-dependent storage is selected (SWS_Det_00025). */
void Det_Start(void);
/** Report to every runtime callout and return E_OK. Before Init, no action. */
Std_ReturnType Det_ReportRuntimeError(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                                      uint8_t ErrorId);
/** Report to configured development hooks and halt. Before Init, halt without
 * hooks. This function never returns; its return type is required by R24-11.
 */
Std_ReturnType Det_ReportError(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                               uint8_t ErrorId);
#endif
