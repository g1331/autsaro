/** @file Windows-only adapter calling the Win32 SetEvent without the OS macro. */
#include "Os_Host.h"
#include "Os_HostIo.h"
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

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP int Os_HostReadByte(char *byte)
{
    DWORD count = 0u;
    int result = -1;
    if (ReadFile(GetStdHandle(STD_INPUT_HANDLE), byte, 1u, &count, NULL) != 0) {
        result = (int)count;
    } else if (GetLastError() == ERROR_BROKEN_PIPE) {
        result = 0;
    } else {
        /* Invalid handles and other read failures are not EOF. */
    }
    return result;
}

OS_MEMMAP int Os_HostWriteBytes(int error_stream, const char *data, size_t length)
{
    size_t offset = 0u;
    int result = 0;
    if ((error_stream == 0) || (error_stream == 1)) {
        HANDLE handle = GetStdHandle(error_stream == 0 ? STD_OUTPUT_HANDLE : STD_ERROR_HANDLE);
        result = 1;
        while ((offset < length) && (result != 0)) {
            const size_t remaining = length - offset;
            const DWORD requested = remaining > UINT32_MAX ? UINT32_MAX : (DWORD)remaining;
            DWORD written = 0u;
            if ((WriteFile(handle, &data[offset], requested, &written, NULL) == 0) ||
                (written == 0u) || (written > requested)) {
                result = 0;
            } else {
                offset += written;
            }
        }
    }
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
