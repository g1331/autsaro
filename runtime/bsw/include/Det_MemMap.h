/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. VAR_CLEARED_8 maps the
 * selected thread-local reporting flag. This mapping makes no MCU placement claim.
 * MR12 DIR 4.10 VIOLATION: Process each required pair; guard fixed attributes only.
 */
#include "MemMap_Version.h"
#if defined(DET_START_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE) || defined(DET_MEMMAP_TLS_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_VAR_ACTIVE
#define DET_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef DET_MEMMAP_VAR_ACTIVE
#error "Unmatched Det cleared-variable section"
#endif
#undef DET_MEMMAP_VAR_ACTIVE
#undef DET_VAR_CLEARED
#elif defined(DET_START_SEC_VAR_CLEARED_8)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_START_SEC_VAR_CLEARED_8
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE) || defined(DET_MEMMAP_TLS_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_TLS_ACTIVE
#if defined(_WIN32)
#define DET_VAR_CLEARED_8 __attribute__((section(".tls$DET")))
#else
#define DET_VAR_CLEARED_8 __attribute__((section(".tbss.det")))
#endif
#elif defined(DET_STOP_SEC_VAR_CLEARED_8)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_STOP_SEC_VAR_CLEARED_8
#ifndef DET_MEMMAP_TLS_ACTIVE
#error "Unmatched Det thread-variable section"
#endif
#undef DET_MEMMAP_TLS_ACTIVE
#undef DET_VAR_CLEARED_8
#elif defined(DET_START_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_START_SEC_CODE
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE) || defined(DET_MEMMAP_TLS_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_CODE_ACTIVE
#define DET_CODE __attribute__((section(".text")))
#elif defined(DET_STOP_SEC_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_STOP_SEC_CODE
#ifndef DET_MEMMAP_CODE_ACTIVE
#error "Unmatched Det code section"
#endif
#undef DET_MEMMAP_CODE_ACTIVE
#undef DET_CODE
#elif defined(DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE) || defined(DET_MEMMAP_TLS_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_CALLOUT_ACTIVE
#define DET_CALLOUT_CODE __attribute__((section(".text.det.Ecu_DetRuntimeCallout")))
#elif defined(DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE)
/* MR12 RULE 20.5 VIOLATION: Clear the handled request; check each later mapping pair. */
#undef DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE
#ifndef DET_MEMMAP_CALLOUT_ACTIVE
#error "Unmatched Det callout section"
#endif
#undef DET_MEMMAP_CALLOUT_ACTIVE
#undef DET_CALLOUT_CODE
#else
#error "Det_MemMap requires exactly one supported marker"
#endif

#if defined(DET_START_SEC_VAR_CLEARED_8) || defined(DET_STOP_SEC_VAR_CLEARED_8) ||                 \
    defined(DET_START_SEC_VAR_CLEARED_UNSPECIFIED) ||                                              \
    defined(DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined(DET_START_SEC_CODE) ||                \
    defined(DET_STOP_SEC_CODE) || defined(DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE) ||             \
    defined(DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE)
#error "Det_MemMap requires exactly one supported marker"
#endif
