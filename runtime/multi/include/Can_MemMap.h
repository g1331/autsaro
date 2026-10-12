/** @file Repeated section protocol for controlled GCC PE/ELF targets.
 * Explicit .text/.bss mapping; no MCU linker placement is implied.
 */
#if (defined(CAN_START_SEC_CODE) + defined(CAN_STOP_SEC_CODE) +                                    \
     defined(CAN_START_SEC_VAR_CLEARED_UNSPECIFIED) +                                              \
     defined(CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED)) != 1
#error "Can_MemMap requires exactly one supported marker"
#else
#if defined(CAN_START_SEC_CODE)
#if defined(CAN_MEMMAP_OPEN)
#error "Nested CAN section"
#endif
#define CAN_MEMMAP_OPEN
#define CAN_MEMMAP_CODE_OPEN
#define CAN_CODE __attribute__((section(".text")))
#undef CAN_START_SEC_CODE
#elif defined(CAN_STOP_SEC_CODE)
#if !defined(CAN_MEMMAP_CODE_OPEN)
#error "CAN CODE stop without matching start"
#endif
#undef CAN_MEMMAP_CODE_OPEN
#undef CAN_MEMMAP_OPEN
#undef CAN_CODE
#undef CAN_STOP_SEC_CODE
#elif defined(CAN_START_SEC_VAR_CLEARED_UNSPECIFIED)
#if defined(CAN_MEMMAP_OPEN)
#error "Nested CAN section"
#endif
#define CAN_MEMMAP_OPEN
#define CAN_MEMMAP_VAR_OPEN
#define CAN_VAR_CLEARED __attribute__((section(".bss")))
#undef CAN_START_SEC_VAR_CLEARED_UNSPECIFIED
#elif defined(CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#if !defined(CAN_MEMMAP_VAR_OPEN)
#error "CAN VAR stop without matching start"
#endif
#undef CAN_MEMMAP_VAR_OPEN
#undef CAN_MEMMAP_OPEN
#undef CAN_VAR_CLEARED
#undef CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#endif
#endif
