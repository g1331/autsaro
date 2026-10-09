/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#if defined(COMM_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef COMM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(COMM_MEMMAP_VAR_ACTIVE) || defined(COMM_MEMMAP_CODE_ACTIVE)
#error "Nested ComM memory section"
#endif
#define COMM_MEMMAP_VAR_ACTIVE
#define COMM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef COMM_MEMMAP_VAR_ACTIVE
#error "Unmatched ComM cleared-variable section"
#endif
#undef COMM_MEMMAP_VAR_ACTIVE
#undef COMM_VAR_CLEARED
#elif defined(COMM_START_SEC_CODE)
#undef COMM_START_SEC_CODE
#if defined(COMM_MEMMAP_VAR_ACTIVE) || defined(COMM_MEMMAP_CODE_ACTIVE)
#error "Nested ComM memory section"
#endif
#define COMM_MEMMAP_CODE_ACTIVE
#define COMM_CODE __attribute__((section(".text")))
#elif defined(COMM_STOP_SEC_CODE)
#undef COMM_STOP_SEC_CODE
#ifndef COMM_MEMMAP_CODE_ACTIVE
#error "Unmatched ComM code section"
#endif
#undef COMM_MEMMAP_CODE_ACTIVE
#undef COMM_CODE
#else
#ifndef COMM_MEMMAP_HEADER_CHECK
#define COMM_MEMMAP_HEADER_CHECK
/** Allows the normal standalone public-header compiler check. */
typedef unsigned char ComM_MemMap_HeaderCheck;
#endif
#endif
