#ifndef AUTOSAR_EPIC4_OS_BACKEND_H
#define AUTOSAR_EPIC4_OS_BACKEND_H
#include "Os_Target.h"
#include "FreeRTOS.h"
#include "task.h"
#include <windows.h>
#include "Os_Stack.h"
void Os_BackendStart(AppModeType mode);
void Os_BackendShutdown(StatusType error);
void Os_BackendRequestShutdown(StatusType error);
void Os_BackendGuardService(void);
StatusType Os_BackendState(TaskType id, TaskStateRefType state);
StatusType Os_BackendActivate(TaskType id);
StatusType Os_BackendFinish(void);
StatusType Os_BackendInspect(TaskType id, Os_ActivationInfo *info);
void Os_BackendObserveSwitch(void);
int Os_BackendTakeIsrReschedule(void);
void Os_BackendAssert(const char *file, int line);
HANDLE Os_PortEvent(LPSECURITY_ATTRIBUTES attributes, BOOL manual, BOOL initial, LPCSTR name);
HANDLE Os_PortMutex(LPSECURITY_ATTRIBUTES attributes, BOOL owner, LPCSTR name);
HANDLE Os_PortThread(LPSECURITY_ATTRIBUTES attributes, SIZE_T stack, LPTHREAD_START_ROUTINE start,
                     LPVOID argument, DWORD flags, LPDWORD id);
HANDLE Os_PortTaskThread(TaskFunction_t code, void *argument, const StackType_t *buffer_top);
void Os_PortResumeThread(HANDLE thread);
DWORD Os_PortSuspendThread(HANDLE thread);
BOOL Os_PortGetThreadContext(HANDLE thread, CONTEXT *context);
extern const Os_TargetConfig *Os_Config;
extern volatile LONG Os_Closing;
#endif
