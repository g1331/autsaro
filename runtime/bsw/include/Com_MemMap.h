/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#include "MemMap_Version.h"
#if defined(COM_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef COM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(COM_MEMMAP_VAR_ACTIVE) || defined(COM_MEMMAP_CODE_ACTIVE)
#error "Nested Com memory section"
#endif
#define COM_MEMMAP_VAR_ACTIVE
#define COM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef COM_MEMMAP_VAR_ACTIVE
#error "Unmatched Com cleared-variable section"
#endif
#undef COM_MEMMAP_VAR_ACTIVE
#undef COM_VAR_CLEARED
#elif defined(COM_START_SEC_CODE)
#undef COM_START_SEC_CODE
#if defined(COM_MEMMAP_VAR_ACTIVE) || defined(COM_MEMMAP_CODE_ACTIVE)
#error "Nested Com memory section"
#endif
#define COM_MEMMAP_CODE_ACTIVE
#define COM_CODE __attribute__((section(".text")))
#elif defined(COM_STOP_SEC_CODE)
#undef COM_STOP_SEC_CODE
#ifndef COM_MEMMAP_CODE_ACTIVE
#error "Unmatched Com code section"
#endif
#undef COM_MEMMAP_CODE_ACTIVE
#undef COM_CODE
#else
#error "Com_MemMap requires exactly one supported marker"
#endif

#if defined(COM_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                              \
    defined(COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(COM_START_SEC_CODE) ||                \
    defined(COM_STOP_SEC_CODE)
#error "Com_MemMap requires exactly one supported marker"
#endif
