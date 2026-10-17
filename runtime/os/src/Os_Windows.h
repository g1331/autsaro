/** @file Win32 header import that keeps the OS SetEvent and ApplicationType names intact. */
#ifndef AUTOSAR_OS_WINDOWS_H
#define AUTOSAR_OS_WINDOWS_H
#include "Std_Types.h"
/* The standard OS service and Win32 API share the spelling SetEvent. Import
 * Windows declarations without the owned public OS macro, then restore it.
 * Native users call the separately compiled adapter; apps keep the OS macro. */
#ifdef SetEvent
#undef SetEvent
#define OS_RESTORE_EVENT_SERVICE STD_ON
#endif
/* Windows COM declares an unrelated ApplicationType enum. Keep it inside the
 * host adapter namespace while preserving the standard Os.h public name. */
#define ApplicationType Os_Win32ApplicationType
#define boolean         Os_Win32Boolean
#include <windows.h>
#undef boolean
#undef ApplicationType
BOOL Os_HostSetEvent(HANDLE event);
#if defined(OS_RESTORE_EVENT_SERVICE) && (OS_RESTORE_EVENT_SERVICE == STD_ON)
#define SetEvent Os_SetEvent
#undef OS_RESTORE_EVENT_SERVICE
#endif
#endif
