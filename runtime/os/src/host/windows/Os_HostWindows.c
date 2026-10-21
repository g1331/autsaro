/** @file Windows-only adapter calling the Win32 SetEvent without the OS macro. */
#include "Os_Host.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event) { return SetEvent(event); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostWindows.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostWindows.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
