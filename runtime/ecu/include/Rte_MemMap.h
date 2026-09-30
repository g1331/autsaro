/** @file Repeated-include code mapping for generated Win64 RTE implementations.
 * RTE_START_SEC_CODE / RTE_STOP_SEC_CODE surround generated code.
 * RTE_CODE selects the readable, executable .rte_code PE section.
 */
#ifndef RTE_MEMMAP_ATTRIBUTES_DEFINED
#ifdef RTE_CODE
#error RTE_CODE override is unsupported
#endif
#define RTE_MEMMAP_ATTRIBUTES_DEFINED
#define RTE_CODE __attribute__((section(".rte_code")))
#endif
#if defined(RTE_START_SEC_CODE) && defined(RTE_STOP_SEC_CODE)
#error Conflicting RTE memory mapping markers
#elif defined(RTE_START_SEC_CODE)
#undef RTE_START_SEC_CODE
#ifdef RTE_MEMMAP_CODE_ACTIVE
#error Nested RTE code section
#endif
#define RTE_MEMMAP_CODE_ACTIVE
#elif defined(RTE_STOP_SEC_CODE)
#undef RTE_STOP_SEC_CODE
#ifndef RTE_MEMMAP_CODE_ACTIVE
#error RTE code section stopped without a matching start
#endif
#undef RTE_MEMMAP_CODE_ACTIVE
#else
#error Unsupported or missing RTE memory mapping marker
#endif
