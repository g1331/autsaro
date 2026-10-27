/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 * MR12 DIR 4.10 VIOLATION: Process each required pair; guard fixed attributes only.
 */
#include "MemMap_Version.h"
#if defined(COMM_START_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef COMM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(COMM_MEMMAP_VAR_ACTIVE) || defined(COMM_MEMMAP_CODE_ACTIVE)
#error "Nested ComM memory section"
#endif
#define COMM_MEMMAP_VAR_ACTIVE
#define COMM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef COMM_MEMMAP_VAR_ACTIVE
#error "Unmatched ComM cleared-variable section"
#endif
#undef COMM_MEMMAP_VAR_ACTIVE
#undef COMM_VAR_CLEARED
#elif defined(COMM_START_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef COMM_START_SEC_CODE
#if defined(COMM_MEMMAP_VAR_ACTIVE) || defined(COMM_MEMMAP_CODE_ACTIVE)
#error "Nested ComM memory section"
#endif
#define COMM_MEMMAP_CODE_ACTIVE
#define COMM_CODE __attribute__((section(".text")))
#elif defined(COMM_STOP_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef COMM_STOP_SEC_CODE
#ifndef COMM_MEMMAP_CODE_ACTIVE
#error "Unmatched ComM code section"
#endif
#undef COMM_MEMMAP_CODE_ACTIVE
#undef COMM_CODE
#else
#error "ComM_MemMap requires exactly one supported marker"
#endif

#if defined(COMM_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                             \
    defined(COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(COMM_START_SEC_CODE) ||              \
    defined(COMM_STOP_SEC_CODE)
#error "ComM_MemMap requires exactly one supported marker"
#endif
