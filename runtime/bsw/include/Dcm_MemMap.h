/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 * MR12 DIR 4.10 VIOLATION: Process each required pair; guard fixed attributes only.
 */
#include "MemMap_Version.h"
#if defined(DCM_START_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DCM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(DCM_MEMMAP_VAR_ACTIVE) || defined(DCM_MEMMAP_CODE_ACTIVE)
#error "Nested Dcm memory section"
#endif
#define DCM_MEMMAP_VAR_ACTIVE
#define DCM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef DCM_MEMMAP_VAR_ACTIVE
#error "Unmatched Dcm cleared-variable section"
#endif
#undef DCM_MEMMAP_VAR_ACTIVE
#undef DCM_VAR_CLEARED
#elif defined(DCM_START_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DCM_START_SEC_CODE
#if defined(DCM_MEMMAP_VAR_ACTIVE) || defined(DCM_MEMMAP_CODE_ACTIVE)
#error "Nested Dcm memory section"
#endif
#define DCM_MEMMAP_CODE_ACTIVE
#define DCM_CODE __attribute__((section(".text")))
#elif defined(DCM_STOP_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DCM_STOP_SEC_CODE
#ifndef DCM_MEMMAP_CODE_ACTIVE
#error "Unmatched Dcm code section"
#endif
#undef DCM_MEMMAP_CODE_ACTIVE
#undef DCM_CODE
#else
#error "Dcm_MemMap requires exactly one supported marker"
#endif

#if defined(DCM_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                              \
    defined(DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(DCM_START_SEC_CODE) ||                \
    defined(DCM_STOP_SEC_CODE)
#error "Dcm_MemMap requires exactly one supported marker"
#endif
