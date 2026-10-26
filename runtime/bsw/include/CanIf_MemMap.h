/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#include "MemMap_Version.h"
#if defined(CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(CANIF_MEMMAP_VAR_ACTIVE) || defined(CANIF_MEMMAP_CODE_ACTIVE)
#error "Nested CanIf memory section"
#endif
#define CANIF_MEMMAP_VAR_ACTIVE
#define CANIF_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef CANIF_MEMMAP_VAR_ACTIVE
#error "Unmatched CanIf cleared-variable section"
#endif
#undef CANIF_MEMMAP_VAR_ACTIVE
#undef CANIF_VAR_CLEARED
#elif defined(CANIF_START_SEC_CODE)
#undef CANIF_START_SEC_CODE
#if defined(CANIF_MEMMAP_VAR_ACTIVE) || defined(CANIF_MEMMAP_CODE_ACTIVE)
#error "Nested CanIf memory section"
#endif
#define CANIF_MEMMAP_CODE_ACTIVE
#define CANIF_CODE __attribute__((section(".text")))
#elif defined(CANIF_STOP_SEC_CODE)
#undef CANIF_STOP_SEC_CODE
#ifndef CANIF_MEMMAP_CODE_ACTIVE
#error "Unmatched CanIf code section"
#endif
#undef CANIF_MEMMAP_CODE_ACTIVE
#undef CANIF_CODE
#else
#error "CanIf_MemMap requires exactly one supported marker"
#endif

#if defined(CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                            \
    defined(CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(CANIF_START_SEC_CODE) ||            \
    defined(CANIF_STOP_SEC_CODE)
#error "CanIf_MemMap requires exactly one supported marker"
#endif
