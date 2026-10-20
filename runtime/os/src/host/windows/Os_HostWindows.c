/** @file Windows-only adapter calling the Win32 SetEvent without the OS macro. */
#include "Os_Host.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event) { return SetEvent(event); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
