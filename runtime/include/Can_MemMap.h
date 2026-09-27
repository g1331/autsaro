/** @file
 * @brief Host CAN memory section markers for the flat Windows address space.
 *
 * This header intentionally has no include guard because each section marker
 * must be consumed by a separate include. The host linker places code and
 * zero-initialized data in its default sections.
 */
#if defined(CAN_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#elif defined(CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#elif defined(CAN_START_SEC_CODE)
#undef CAN_START_SEC_CODE
#elif defined(CAN_STOP_SEC_CODE)
#undef CAN_STOP_SEC_CODE
#else
#ifndef CAN_MEMMAP_HEADER_CHECK
#define CAN_MEMMAP_HEADER_CHECK
/** @brief Allows standalone C99 header checks without an active section. */
typedef unsigned char Can_MemMap_HeaderCheck;
#endif
#endif
