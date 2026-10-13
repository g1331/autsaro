/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#if defined(DCM_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef DCM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(DCM_MEMMAP_VAR_ACTIVE) || defined(DCM_MEMMAP_CODE_ACTIVE)
#error "Nested Dcm memory section"
#endif
#define DCM_MEMMAP_VAR_ACTIVE
#define DCM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef DCM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef DCM_MEMMAP_VAR_ACTIVE
#error "Unmatched Dcm cleared-variable section"
#endif
#undef DCM_MEMMAP_VAR_ACTIVE
#undef DCM_VAR_CLEARED
#elif defined(DCM_START_SEC_CODE)
#undef DCM_START_SEC_CODE
#if defined(DCM_MEMMAP_VAR_ACTIVE) || defined(DCM_MEMMAP_CODE_ACTIVE)
#error "Nested Dcm memory section"
#endif
#define DCM_MEMMAP_CODE_ACTIVE
#define DCM_CODE __attribute__((section(".text")))
#elif defined(DCM_STOP_SEC_CODE)
#undef DCM_STOP_SEC_CODE
#ifndef DCM_MEMMAP_CODE_ACTIVE
#error "Unmatched Dcm code section"
#endif
#undef DCM_MEMMAP_CODE_ACTIVE
#undef DCM_CODE
#else
#ifndef DCM_MEMMAP_HEADER_CHECK
#define DCM_MEMMAP_HEADER_CHECK
/** Allows the normal standalone public-header compiler check. */
typedef unsigned char Dcm_MemMap_HeaderCheck;
#endif
#endif
