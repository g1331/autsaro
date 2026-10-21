/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#include "MemMap_Version.h"
#if defined(PDUR_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef PDUR_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(PDUR_MEMMAP_VAR_ACTIVE) || defined(PDUR_MEMMAP_CODE_ACTIVE)
#error "Nested PduR memory section"
#endif
#define PDUR_MEMMAP_VAR_ACTIVE
#define PDUR_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(PDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef PDUR_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef PDUR_MEMMAP_VAR_ACTIVE
#error "Unmatched PduR cleared-variable section"
#endif
#undef PDUR_MEMMAP_VAR_ACTIVE
#undef PDUR_VAR_CLEARED
#elif defined(PDUR_START_SEC_CODE)
#undef PDUR_START_SEC_CODE
#if defined(PDUR_MEMMAP_VAR_ACTIVE) || defined(PDUR_MEMMAP_CODE_ACTIVE)
#error "Nested PduR memory section"
#endif
#define PDUR_MEMMAP_CODE_ACTIVE
#define PDUR_CODE __attribute__((section(".text")))
#elif defined(PDUR_STOP_SEC_CODE)
#undef PDUR_STOP_SEC_CODE
#ifndef PDUR_MEMMAP_CODE_ACTIVE
#error "Unmatched PduR code section"
#endif
#undef PDUR_MEMMAP_CODE_ACTIVE
#undef PDUR_CODE
#else
#ifndef PDUR_MEMMAP_HEADER_CHECK
#define PDUR_MEMMAP_HEADER_CHECK
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "PduR_MemMap.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR_MemMap.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/** No-marker header smoke-test token; not an ECU interface type. */
typedef uint8 PduR_MemMap_HeaderCheck;
#endif
#endif
