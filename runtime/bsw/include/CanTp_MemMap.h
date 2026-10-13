/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#if defined(CANTP_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANTP_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(CANTP_MEMMAP_VAR_ACTIVE) || defined(CANTP_MEMMAP_CODE_ACTIVE)
#error "Nested CanTp memory section"
#endif
#define CANTP_MEMMAP_VAR_ACTIVE
#define CANTP_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(CANTP_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANTP_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef CANTP_MEMMAP_VAR_ACTIVE
#error "Unmatched CanTp cleared-variable section"
#endif
#undef CANTP_MEMMAP_VAR_ACTIVE
#undef CANTP_VAR_CLEARED
#elif defined(CANTP_START_SEC_CODE)
#undef CANTP_START_SEC_CODE
#if defined(CANTP_MEMMAP_VAR_ACTIVE) || defined(CANTP_MEMMAP_CODE_ACTIVE)
#error "Nested CanTp memory section"
#endif
#define CANTP_MEMMAP_CODE_ACTIVE
#define CANTP_CODE __attribute__((section(".text")))
#elif defined(CANTP_STOP_SEC_CODE)
#undef CANTP_STOP_SEC_CODE
#ifndef CANTP_MEMMAP_CODE_ACTIVE
#error "Unmatched CanTp code section"
#endif
#undef CANTP_MEMMAP_CODE_ACTIVE
#undef CANTP_CODE
#else
#ifndef CANTP_MEMMAP_HEADER_CHECK
#define CANTP_MEMMAP_HEADER_CHECK
/** Allows the normal standalone public-header compiler check. */
typedef unsigned char CanTp_MemMap_HeaderCheck;
#endif
#endif
