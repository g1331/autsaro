/** @file Repeated-include memory mapping for the fixed Win64 OS code target.
 * OS_START_SEC_CODE and OS_STOP_SEC_CODE surround OS entry declarations.
 * OS_CODE binds their definitions to a readable, executable PE code section.
 * The header intentionally has no include guard.
 */
#ifndef OS_MEMMAP_ATTRIBUTES_DEFINED
#ifdef OS_CODE
#error OS_CODE override is unsupported
#endif
#define OS_MEMMAP_ATTRIBUTES_DEFINED
#define OS_CODE __attribute__((section(".os_code")))
#endif
#if defined(OS_START_SEC_CODE) && defined(OS_STOP_SEC_CODE)
#error Conflicting OS memory mapping markers
#elif defined(OS_START_SEC_CODE)
#undef OS_START_SEC_CODE
#ifdef OS_MEMMAP_CODE_ACTIVE
#error Nested OS code section
#endif
#define OS_MEMMAP_CODE_ACTIVE
#elif defined(OS_STOP_SEC_CODE)
#undef OS_STOP_SEC_CODE
#ifndef OS_MEMMAP_CODE_ACTIVE
#error OS code section stopped without a matching start
#endif
#undef OS_MEMMAP_CODE_ACTIVE
#else
#error Unsupported or missing OS memory mapping marker
#endif
