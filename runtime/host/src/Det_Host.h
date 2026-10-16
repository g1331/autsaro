/** @file Private controlled-host halt and per-thread recursion boundary. */
#ifndef DET_HOST_H
#define DET_HOST_H
#include "Std_Types.h"
/** Enter this thread's nonreturning development report.
 * @return TRUE on first entry; FALSE on recursive entry.
 * Preconditions: Outside every exclusive area.
 * Context: Controlled host thread; no MCU interrupt semantics.
 * Reentrancy: Independent across threads; recursive entry returns FALSE.
 * Ownership: The thread-local marker lasts until the thread/process exits.
 */
boolean Det_HostEnterDevelopmentReport(void);
/** Abort the controlled host ECU after development reporting.
 * @return Never returns.
 * Preconditions: All reporting callouts and exclusive areas have finished.
 * Context: Controlled host thread.
 * Reentrancy: Concurrent calls terminate the same process.
 * Ownership: No parameters or retained storage.
 */
void Det_HostHalt(void);
#endif
