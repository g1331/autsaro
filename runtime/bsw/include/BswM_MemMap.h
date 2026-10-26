/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#include "MemMap_Version.h"
#if defined(BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(BSWM_MEMMAP_VAR_ACTIVE) || defined(BSWM_MEMMAP_CODE_ACTIVE)
#error "Nested BswM memory section"
#endif
#define BSWM_MEMMAP_VAR_ACTIVE
#define BSWM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef BSWM_MEMMAP_VAR_ACTIVE
#error "Unmatched BswM cleared-variable section"
#endif
#undef BSWM_MEMMAP_VAR_ACTIVE
#undef BSWM_VAR_CLEARED
#elif defined(BSWM_START_SEC_CODE)
#undef BSWM_START_SEC_CODE
#if defined(BSWM_MEMMAP_VAR_ACTIVE) || defined(BSWM_MEMMAP_CODE_ACTIVE)
#error "Nested BswM memory section"
#endif
#define BSWM_MEMMAP_CODE_ACTIVE
#define BSWM_CODE __attribute__((section(".text")))
#elif defined(BSWM_STOP_SEC_CODE)
#undef BSWM_STOP_SEC_CODE
#ifndef BSWM_MEMMAP_CODE_ACTIVE
#error "Unmatched BswM code section"
#endif
#undef BSWM_MEMMAP_CODE_ACTIVE
#undef BSWM_CODE
#else
#error "BswM_MemMap requires exactly one supported marker"
#endif

#if defined(BSWM_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                             \
    defined(BSWM_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(BSWM_START_SEC_CODE) ||              \
    defined(BSWM_STOP_SEC_CODE)
#error "BswM_MemMap requires exactly one supported marker"
#endif
