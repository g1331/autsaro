#ifndef AUTOSAR_EPIC4_OS_BACKEND_H
#define AUTOSAR_EPIC4_OS_BACKEND_H
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include <windows.h>
void Os_BackendStart(AppModeType mode);
void Os_BackendShutdown(StatusType error);
StatusType Os_BackendState(TaskType id, TaskStateRefType state);
void Os_BackendAssert(const char *file, int line);
HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name);
HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);
HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                     LPVOID argument, DWORD flags, LPDWORD id);
extern const Os_TargetConfig *Os_Config;
extern volatile LONG Os_Closing;
#endif
