/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#if defined(DET_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef DET_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_VAR_ACTIVE
#define DET_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef DET_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef DET_MEMMAP_VAR_ACTIVE
#error "Unmatched Det cleared-variable section"
#endif
#undef DET_MEMMAP_VAR_ACTIVE
#undef DET_VAR_CLEARED
#elif defined(DET_START_SEC_CODE)
#undef DET_START_SEC_CODE
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_CODE_ACTIVE
#define DET_CODE __attribute__((section(".text")))
#elif defined(DET_STOP_SEC_CODE)
#undef DET_STOP_SEC_CODE
#ifndef DET_MEMMAP_CODE_ACTIVE
#error "Unmatched Det code section"
#endif
#undef DET_MEMMAP_CODE_ACTIVE
#undef DET_CODE
#elif defined(DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE)
#undef DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE
#if defined(DET_MEMMAP_VAR_ACTIVE) || defined(DET_MEMMAP_CODE_ACTIVE) ||                           \
    defined(DET_MEMMAP_CALLOUT_ACTIVE)
#error "Nested Det memory section"
#endif
#define DET_MEMMAP_CALLOUT_ACTIVE
#define DET_CALLOUT_CODE __attribute__((section(".text.det.Ecu_DetRuntimeCallout")))
#elif defined(DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE)
#undef DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE
#ifndef DET_MEMMAP_CALLOUT_ACTIVE
#error "Unmatched Det callout section"
#endif
#undef DET_MEMMAP_CALLOUT_ACTIVE
#undef DET_CALLOUT_CODE
#else
#ifndef DET_MEMMAP_HEADER_CHECK
#define DET_MEMMAP_HEADER_CHECK
#include "Std_Types.h"
/** No-marker header smoke-test token; not an ECU interface type. */
typedef uint8 Det_MemMap_HeaderCheck;
#endif
#endif
