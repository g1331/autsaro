/** @file Caller-owned bounded record construction for native protocol and fault output. */
#include "Os_HostIo.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_HostTextByte(Os_HostTextType *text, char value)
{
    if ((text->failed == 0) && (text->length < text->capacity)) {
        text->data[text->length] = value;
        ++text->length;
    } else {
        text->failed = 1;
    }
}

OS_MEMMAP void Os_HostTextAppend(Os_HostTextType *text, const char *value)
{
    size_t index = 0u;
    while ((text->failed == 0) && (value[index] != '\0')) {
        Os_HostTextByte(text, value[index]);
        ++index;
    }
}

OS_MEMMAP void Os_HostTextField(Os_HostTextType *text, const char *name, uint64_t value)
{
    char digits[20];
    size_t count = 0u;
    uint64_t remaining = value;
    Os_HostTextAppend(text, name);
    do {
        digits[count] = "0123456789"[remaining % UINT64_C(10)];
        ++count;
        remaining /= UINT64_C(10);
    } while (remaining != 0u);
    while (count != 0u) {
        --count;
        Os_HostTextByte(text, digits[count]);
    }
}

OS_MEMMAP void Os_HostTextHex(Os_HostTextType *text, uint64_t value, size_t width, int uppercase)
{
    const char *digits = (uppercase == 0) ? "0123456789abcdef" : "0123456789ABCDEF";
    if ((width == 0u) || (width > 16u) || ((uppercase != 0) && (uppercase != 1))) {
        text->failed = 1;
    } else if ((width < 16u) && ((value >> (width * 4u)) != 0u)) {
        text->failed = 1;
    } else {
        size_t remaining = width;
        while (remaining != 0u) {
            --remaining;
            Os_HostTextByte(text, digits[(value >> (remaining * 4u)) & UINT64_C(15)]);
        }
    }
}

OS_MEMMAP int Os_HostTextWrite(const Os_HostTextType *text)
{
    int result = 0;
    if ((text->failed == 0) && (text->length <= text->capacity)) {
        result = Os_HostWriteBytes(0, text->data, text->length);
    }
    return result;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostIo.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostIo.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
