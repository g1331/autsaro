/** @file Explicit repeated-include OS mapping for the controlled GCC x64 targets.
 * SWS_MemMap_00003: a matching pair surrounds only declarations/definitions of its class.
 * OS_MEMMAP applies the active section attribute; it is unavailable outside a pair.
 * OS_CODE remains the published entry attribute used by the OSEK body macros.
 * CLEARED/INIT preserve C startup and actor TLS initialization, never an Init-time reset.
 * The native vector refinement retains the existing writable, eight-byte-aligned .os_vec ABI.
 * This header intentionally has no include guard.
 */
#ifndef OS_MEMMAP_ATTRIBUTES_DEFINED
#ifdef OS_CODE
#error OS_CODE override is unsupported
#endif
#ifdef OS_THREAD_LOCAL
#error OS_THREAD_LOCAL override is unsupported
#endif
#ifdef OS_MEMMAP
#error OS_MEMMAP override is unsupported
#endif
#define OS_MEMMAP_ATTRIBUTES_DEFINED
#define OS_CODE         __attribute__((section(".os_code")))
#define OS_THREAD_LOCAL __thread
#ifdef _WIN32
#define OS_MEMMAP_CONST_SECTION       ".rdata"
#define OS_MEMMAP_TLS_CLEARED_SECTION ".tls$OS"
#else
#define OS_MEMMAP_CONST_SECTION       ".data.rel.ro.os"
#define OS_MEMMAP_TLS_CLEARED_SECTION ".tbss.os"
#endif
#endif
#if (defined(OS_START_SEC_CODE) + defined(OS_STOP_SEC_CODE) +                                      \
     defined(OS_START_SEC_CONFIG_DATA_PREBUILD_64) +                                               \
     defined(OS_STOP_SEC_CONFIG_DATA_PREBUILD_64) + defined(OS_START_SEC_CONFIG_DATA_PREBUILD_8) + \
     defined(OS_STOP_SEC_CONFIG_DATA_PREBUILD_8) + defined(OS_START_SEC_CONST_64) +                \
     defined(OS_STOP_SEC_CONST_64) + defined(OS_START_SEC_CONST_8) +                               \
     defined(OS_STOP_SEC_CONST_8) + defined(OS_START_SEC_CONST_PTR) +                              \
     defined(OS_STOP_SEC_CONST_PTR) + defined(OS_START_SEC_VAR_CLEARED_32) +                       \
     defined(OS_STOP_SEC_VAR_CLEARED_32) + defined(OS_START_SEC_VAR_CLEARED_64) +                  \
     defined(OS_STOP_SEC_VAR_CLEARED_64) + defined(OS_START_SEC_VAR_CLEARED_8) +                   \
     defined(OS_STOP_SEC_VAR_CLEARED_8) + defined(OS_START_SEC_VAR_CLEARED_PTR) +                  \
     defined(OS_STOP_SEC_VAR_CLEARED_PTR) + defined(OS_START_SEC_VAR_CLEARED_TLS_32) +             \
     defined(OS_STOP_SEC_VAR_CLEARED_TLS_32) + defined(OS_START_SEC_VAR_CLEARED_TLS_64) +          \
     defined(OS_STOP_SEC_VAR_CLEARED_TLS_64) + defined(OS_START_SEC_VAR_CLEARED_TLS_8) +           \
     defined(OS_STOP_SEC_VAR_CLEARED_TLS_8) + defined(OS_START_SEC_VAR_CLEARED_TLS_PTR) +          \
     defined(OS_STOP_SEC_VAR_CLEARED_TLS_PTR) +                                                    \
     defined(OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED) +                                           \
     defined(OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED) +                                            \
     defined(OS_START_SEC_VAR_CLEARED_UNSPECIFIED) +                                               \
     defined(OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED) + defined(OS_START_SEC_VAR_INIT_32) +            \
     defined(OS_STOP_SEC_VAR_INIT_32) + defined(OS_START_SEC_VAR_INIT_8) +                         \
     defined(OS_STOP_SEC_VAR_INIT_8) + defined(OS_START_SEC_VAR_INIT_UNSPECIFIED) +                \
     defined(OS_STOP_SEC_VAR_INIT_UNSPECIFIED) + defined(OS_START_SEC_VAR_INIT_VECTOR_PTR) +       \
     defined(OS_STOP_SEC_VAR_INIT_VECTOR_PTR)) != 1
#error Conflicting OS memory mapping markers
#endif
#if defined(OS_START_SEC_CODE)
#undef OS_START_SEC_CODE
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS code section
#endif
#define OS_MEMMAP_ACTIVE 1
#define OS_MEMMAP        OS_CODE
#define OS_MEMMAP_CODE_ACTIVE
#elif defined(OS_STOP_SEC_CODE)
#undef OS_STOP_SEC_CODE
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 1)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#undef OS_MEMMAP_CODE_ACTIVE
#elif defined(OS_START_SEC_CONFIG_DATA_PREBUILD_64)
#undef OS_START_SEC_CONFIG_DATA_PREBUILD_64
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 2
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_CONST_SECTION)))
#elif defined(OS_STOP_SEC_CONFIG_DATA_PREBUILD_64)
#undef OS_STOP_SEC_CONFIG_DATA_PREBUILD_64
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 2)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_CONFIG_DATA_PREBUILD_8)
#undef OS_START_SEC_CONFIG_DATA_PREBUILD_8
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 3
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_CONST_SECTION)))
#elif defined(OS_STOP_SEC_CONFIG_DATA_PREBUILD_8)
#undef OS_STOP_SEC_CONFIG_DATA_PREBUILD_8
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 3)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_CONST_64)
#undef OS_START_SEC_CONST_64
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 4
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_CONST_SECTION)))
#elif defined(OS_STOP_SEC_CONST_64)
#undef OS_STOP_SEC_CONST_64
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 4)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_CONST_8)
#undef OS_START_SEC_CONST_8
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 5
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_CONST_SECTION)))
#elif defined(OS_STOP_SEC_CONST_8)
#undef OS_STOP_SEC_CONST_8
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 5)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_CONST_PTR)
#undef OS_START_SEC_CONST_PTR
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 6
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_CONST_SECTION)))
#elif defined(OS_STOP_SEC_CONST_PTR)
#undef OS_STOP_SEC_CONST_PTR
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 6)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_32)
#undef OS_START_SEC_VAR_CLEARED_32
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 7
#define OS_MEMMAP        __attribute__((section(".bss.os")))
#elif defined(OS_STOP_SEC_VAR_CLEARED_32)
#undef OS_STOP_SEC_VAR_CLEARED_32
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 7)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_64)
#undef OS_START_SEC_VAR_CLEARED_64
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 8
#define OS_MEMMAP        __attribute__((section(".bss.os")))
#elif defined(OS_STOP_SEC_VAR_CLEARED_64)
#undef OS_STOP_SEC_VAR_CLEARED_64
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 8)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_8)
#undef OS_START_SEC_VAR_CLEARED_8
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 9
#define OS_MEMMAP        __attribute__((section(".bss.os")))
#elif defined(OS_STOP_SEC_VAR_CLEARED_8)
#undef OS_STOP_SEC_VAR_CLEARED_8
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 9)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_PTR)
#undef OS_START_SEC_VAR_CLEARED_PTR
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 10
#define OS_MEMMAP        __attribute__((section(".bss.os")))
#elif defined(OS_STOP_SEC_VAR_CLEARED_PTR)
#undef OS_STOP_SEC_VAR_CLEARED_PTR
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 10)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_TLS_32)
#undef OS_START_SEC_VAR_CLEARED_TLS_32
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 11
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_TLS_CLEARED_SECTION)))
#elif defined(OS_STOP_SEC_VAR_CLEARED_TLS_32)
#undef OS_STOP_SEC_VAR_CLEARED_TLS_32
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 11)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_TLS_64)
#undef OS_START_SEC_VAR_CLEARED_TLS_64
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 12
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_TLS_CLEARED_SECTION)))
#elif defined(OS_STOP_SEC_VAR_CLEARED_TLS_64)
#undef OS_STOP_SEC_VAR_CLEARED_TLS_64
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 12)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_TLS_8)
#undef OS_START_SEC_VAR_CLEARED_TLS_8
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 13
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_TLS_CLEARED_SECTION)))
#elif defined(OS_STOP_SEC_VAR_CLEARED_TLS_8)
#undef OS_STOP_SEC_VAR_CLEARED_TLS_8
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 13)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_TLS_PTR)
#undef OS_START_SEC_VAR_CLEARED_TLS_PTR
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 14
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_TLS_CLEARED_SECTION)))
#elif defined(OS_STOP_SEC_VAR_CLEARED_TLS_PTR)
#undef OS_STOP_SEC_VAR_CLEARED_TLS_PTR
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 14)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED)
#undef OS_START_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 15
#define OS_MEMMAP        __attribute__((section(OS_MEMMAP_TLS_CLEARED_SECTION)))
#elif defined(OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED)
#undef OS_STOP_SEC_VAR_CLEARED_TLS_UNSPECIFIED
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 15)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_CLEARED_UNSPECIFIED)
#undef OS_START_SEC_VAR_CLEARED_UNSPECIFIED
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 16
#define OS_MEMMAP        __attribute__((section(".bss.os")))
#elif defined(OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED)
#undef OS_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 16)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_INIT_32)
#undef OS_START_SEC_VAR_INIT_32
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 17
#define OS_MEMMAP        __attribute__((section(".data.os")))
#elif defined(OS_STOP_SEC_VAR_INIT_32)
#undef OS_STOP_SEC_VAR_INIT_32
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 17)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_INIT_8)
#undef OS_START_SEC_VAR_INIT_8
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 18
#define OS_MEMMAP        __attribute__((section(".data.os")))
#elif defined(OS_STOP_SEC_VAR_INIT_8)
#undef OS_STOP_SEC_VAR_INIT_8
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 18)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_INIT_UNSPECIFIED)
#undef OS_START_SEC_VAR_INIT_UNSPECIFIED
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 19
#define OS_MEMMAP        __attribute__((section(".data.os")))
#elif defined(OS_STOP_SEC_VAR_INIT_UNSPECIFIED)
#undef OS_STOP_SEC_VAR_INIT_UNSPECIFIED
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 19)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#elif defined(OS_START_SEC_VAR_INIT_VECTOR_PTR)
#undef OS_START_SEC_VAR_INIT_VECTOR_PTR
#ifdef OS_MEMMAP_ACTIVE
#error Nested OS memory section
#endif
#define OS_MEMMAP_ACTIVE 20
#define OS_MEMMAP        __attribute__((section(".os_vec"), aligned(8)))
#elif defined(OS_STOP_SEC_VAR_INIT_VECTOR_PTR)
#undef OS_STOP_SEC_VAR_INIT_VECTOR_PTR
#if !defined(OS_MEMMAP_ACTIVE) || (OS_MEMMAP_ACTIVE != 20)
#error OS section stopped without a matching start
#endif
#undef OS_MEMMAP_ACTIVE
#undef OS_MEMMAP
#else
#error Unsupported or missing OS memory mapping marker
#endif
