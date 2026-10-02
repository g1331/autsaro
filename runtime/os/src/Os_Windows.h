#ifndef AUTOSAR_OS_WINDOWS_H
#define AUTOSAR_OS_WINDOWS_H
/* The standard OS service and Win32 API share the spelling SetEvent. Import
 * Windows declarations without the owned public OS macro, then restore it.
 * Native users call the separately compiled adapter; apps keep the OS macro. */
#ifdef SetEvent
#undef SetEvent
#define OS_RESTORE_EVENT_SERVICE 1
#endif
/* Windows COM declares an unrelated ApplicationType enum. Keep it inside the
 * host adapter namespace while preserving the standard Os.h public name. */
#define ApplicationType Os_Win32ApplicationType
#define boolean Os_Win32Boolean
#include <windows.h>
#undef boolean
#undef ApplicationType
BOOL Os_HostSetEvent(HANDLE event);
#ifdef OS_RESTORE_EVENT_SERVICE
#define SetEvent Os_SetEvent
#undef OS_RESTORE_EVENT_SERVICE
#endif
#endif
