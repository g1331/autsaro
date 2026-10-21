/** @file Flat controlled x64 mapping: code .text, cleared state .bss.
 * Repeated inclusion consumes balanced module markers. Native host TLS remains
 * in the toolchain's TLS sections. This mapping makes no MCU placement claim.
 */
#include "MemMap_Version.h"
#if defined(CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#if defined(CANIF_MEMMAP_VAR_ACTIVE) || defined(CANIF_MEMMAP_CODE_ACTIVE)
#error "Nested CanIf memory section"
#endif
#define CANIF_MEMMAP_VAR_ACTIVE
#define CANIF_VAR_CLEARED __attribute__((section(".bss")))
#elif defined(CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#ifndef CANIF_MEMMAP_VAR_ACTIVE
#error "Unmatched CanIf cleared-variable section"
#endif
#undef CANIF_MEMMAP_VAR_ACTIVE
#undef CANIF_VAR_CLEARED
#elif defined(CANIF_START_SEC_CODE)
#undef CANIF_START_SEC_CODE
#if defined(CANIF_MEMMAP_VAR_ACTIVE) || defined(CANIF_MEMMAP_CODE_ACTIVE)
#error "Nested CanIf memory section"
#endif
#define CANIF_MEMMAP_CODE_ACTIVE
#define CANIF_CODE __attribute__((section(".text")))
#elif defined(CANIF_STOP_SEC_CODE)
#undef CANIF_STOP_SEC_CODE
#ifndef CANIF_MEMMAP_CODE_ACTIVE
#error "Unmatched CanIf code section"
#endif
#undef CANIF_MEMMAP_CODE_ACTIVE
#undef CANIF_CODE
#else
#ifndef CANIF_MEMMAP_HEADER_CHECK
#define CANIF_MEMMAP_HEADER_CHECK
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "CanIf_MemMap.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf_MemMap.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/** No-marker header smoke-test token; not an ECU interface type. */
typedef uint8 CanIf_MemMap_HeaderCheck;
#endif
#endif
