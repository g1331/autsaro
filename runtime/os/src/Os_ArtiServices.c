/** @file Traced public OS service wrappers around the implementation entry points. */
#include "Os_Backend.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP boolean isOsStarted(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_isOsStarted, (uint32)0u, 0);
    }
    const boolean result = Os_Implementation_isOsStarted();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)Os_ArtiAccessStatus());
        const uint32 started = result ? 1u : 0u;
        Os_ArtiService(
            OSServiceId_isOsStarted, (Os_ArtiAccessValid() != 0) ? started : UINT32_MAX, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

OS_MEMMAP StatusType TerminateTask(void)
{
    const sint32 trace = Os_ArtiServiceEnter();
    const Os_NativeStackType *stack = Os_StackCurrent();
    const uint32 current =
        ((stack != NULL) && (stack->role == 'T')) ? (uint32)Os_ArtiRunningTask : UINT32_MAX;
    if (trace != 0) {
        Os_ArtiService(OSServiceId_TerminateTask, (uint32)current, 0);
    }
    const StatusType result = Os_Implementation_TerminateTask();
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_TerminateTask, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}

OS_MEMMAP StatusType NextScheduleTable(ScheduleTableType ScheduleTableID_From,
                                       ScheduleTableType ScheduleTableID_To)
{
    const sint32 trace = Os_ArtiServiceEnter();
    if (trace != 0) {
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32)ScheduleTableID_To, 0);
    }
    const StatusType result =
        Os_Implementation_NextScheduleTable(ScheduleTableID_From, ScheduleTableID_To);
    if (trace != 0) {
        Arti_CaptureServiceStatus((uint32)result);
        Os_ArtiService(OSServiceId_NextScheduleTable, (uint32)result, 1);
    }
    Os_ArtiServiceLeave();
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_ArtiServices.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_ArtiServices.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
