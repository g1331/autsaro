/** @file
 * @brief Host CanIf memory section markers for the flat Windows address space.
 *
 * This header intentionally has no include guard because each section marker
 * must be consumed by a separate include. The host linker places code and
 * zero-initialized data in its default sections.
 */
#if defined(CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#elif defined(CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#elif defined(CANIF_START_SEC_CODE)
#undef CANIF_START_SEC_CODE
#elif defined(CANIF_STOP_SEC_CODE)
#undef CANIF_STOP_SEC_CODE
#else
#ifndef CANIF_MEMMAP_HEADER_CHECK
#define CANIF_MEMMAP_HEADER_CHECK
/** @brief Allows standalone C99 header checks without an active section. */
typedef unsigned char CanIf_MemMap_HeaderCheck;
#endif
#endif
