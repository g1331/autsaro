/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 * MR12 DIR 4.10 VIOLATION: Process each required pair; guard fixed attributes only.
 */
#include "MemMap_Version.h"
#if defined(LSDUR_START_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef LSDUR_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(LSDUR_MEMMAP_VAR_ACTIVE) || defined(LSDUR_MEMMAP_CODE_ACTIVE)
#error "Nested LSduR memory section"
#endif
#define LSDUR_MEMMAP_VAR_ACTIVE
#define LSDUR_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(LSDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef LSDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef LSDUR_MEMMAP_VAR_ACTIVE
#error "Unmatched LSduR cleared-variable section"
#endif
#undef LSDUR_MEMMAP_VAR_ACTIVE
#undef LSDUR_VAR_CLEARED
#elif defined(LSDUR_START_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef LSDUR_START_SEC_CODE
#if defined(LSDUR_MEMMAP_VAR_ACTIVE) || defined(LSDUR_MEMMAP_CODE_ACTIVE)
#error "Nested LSduR memory section"
#endif
#define LSDUR_MEMMAP_CODE_ACTIVE
#define LSDUR_CODE __attribute__((section(".text")))
#elif defined(LSDUR_STOP_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef LSDUR_STOP_SEC_CODE
#ifndef LSDUR_MEMMAP_CODE_ACTIVE
#error "Unmatched LSduR code section"
#endif
#undef LSDUR_MEMMAP_CODE_ACTIVE
#undef LSDUR_CODE
#else
#error "LSduR_MemMap requires exactly one supported marker"
#endif

#if defined(LSDUR_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                            \
    defined(LSDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(LSDUR_START_SEC_CODE) ||            \
    defined(LSDUR_STOP_SEC_CODE)
#error "LSduR_MemMap requires exactly one supported marker"
#endif
