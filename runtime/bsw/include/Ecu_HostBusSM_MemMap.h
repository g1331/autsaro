/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#if defined(ECU_HOSTBUSSM_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef ECU_HOSTBUSSM_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(ECU_HOSTBUSSM_MEMMAP_VAR_ACTIVE) || defined(ECU_HOSTBUSSM_MEMMAP_CODE_ACTIVE)
#error "Nested Ecu_HostBusSM memory section"
#endif
#define ECU_HOSTBUSSM_MEMMAP_VAR_ACTIVE
#define ECU_HOSTBUSSM_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(ECU_HOSTBUSSM_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef ECU_HOSTBUSSM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef ECU_HOSTBUSSM_MEMMAP_VAR_ACTIVE
#error "Unmatched Ecu_HostBusSM cleared-variable section"
#endif
#undef ECU_HOSTBUSSM_MEMMAP_VAR_ACTIVE
#undef ECU_HOSTBUSSM_VAR_CLEARED
#elif defined(ECU_HOSTBUSSM_START_SEC_CODE)
#undef ECU_HOSTBUSSM_START_SEC_CODE
#if defined(ECU_HOSTBUSSM_MEMMAP_VAR_ACTIVE) || defined(ECU_HOSTBUSSM_MEMMAP_CODE_ACTIVE)
#error "Nested Ecu_HostBusSM memory section"
#endif
#define ECU_HOSTBUSSM_MEMMAP_CODE_ACTIVE
#define ECU_HOSTBUSSM_CODE __attribute__((section(".text")))
#elif defined(ECU_HOSTBUSSM_STOP_SEC_CODE)
#undef ECU_HOSTBUSSM_STOP_SEC_CODE
#ifndef ECU_HOSTBUSSM_MEMMAP_CODE_ACTIVE
#error "Unmatched Ecu_HostBusSM code section"
#endif
#undef ECU_HOSTBUSSM_MEMMAP_CODE_ACTIVE
#undef ECU_HOSTBUSSM_CODE
#else
#ifndef ECU_HOSTBUSSM_MEMMAP_HEADER_CHECK
#define ECU_HOSTBUSSM_MEMMAP_HEADER_CHECK
/** Allows the normal standalone public-header compiler check. */
typedef unsigned char Ecu_HostBusSM_MemMap_HeaderCheck;
#endif
#endif
