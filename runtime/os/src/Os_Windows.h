/** @file Win32 header import that keeps the OS SetEvent and ApplicationType names intact. */
#ifndef AUTOSAR_OS_WINDOWS_H
#define AUTOSAR_OS_WINDOWS_H
#include "Os_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Os_Windows.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Windows.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
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
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP BOOL Os_HostSetEvent(HANDLE event);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Windows.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Windows.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#if defined(OS_RESTORE_EVENT_SERVICE) && (OS_RESTORE_EVENT_SERVICE == STD_ON)
#define SetEvent Os_SetEvent
#undef OS_RESTORE_EVENT_SERVICE
#endif
#endif
