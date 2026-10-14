/** @file Windows-only adapter calling the Win32 SetEvent without the OS macro. */
#include "Os_Host.h"
BOOL Os_HostSetEvent(HANDLE event) { return SetEvent(event); }
