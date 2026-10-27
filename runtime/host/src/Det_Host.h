/** @file Private controlled-host halt and per-thread recursion boundary. */
#ifndef DET_HOST_H
#define DET_HOST_H
#include "Std_Types.h"
#define DET_START_SEC_CODE
#include "Det_MemMap.h"
/** Enter this thread's nonreturning development report.
 * @return TRUE on first entry; FALSE on recursive entry.
 * Preconditions: Outside every exclusive area.
 * Context: Controlled host thread; no MCU interrupt semantics.
 * Reentrancy: Independent across threads; recursive entry returns FALSE.
 * Ownership: The thread-local marker lasts until the thread/process exits.
 */
DET_CODE boolean Det_HostEnterDevelopmentReport(void);
/** Terminate the controlled host ECU with status 134 after development reporting.
 * @par Return
 * None.
 * This function never returns.
 * Preconditions: All reporting callouts and exclusive areas have finished.
 * Context: Controlled host thread.
 * Reentrancy: Concurrent calls terminate the same process.
 * Ownership: No parameters or retained storage.
 */
DET_CODE void Det_HostHalt(void);
#define DET_STOP_SEC_CODE
#include "Det_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Det_Host.h -> Det_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Det_Host.h -> Det_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
