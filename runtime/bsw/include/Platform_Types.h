/** @file Controlled x64 platform types for the standard multi-component stack. */
#ifndef PLATFORM_TYPES_H
#define PLATFORM_TYPES_H
/** Published information (SWS_BSW_00059): fixed R24-11 interface release, 4.10.0. */
#define PLATFORM_AR_RELEASE_MAJOR_VERSION    4U
#define PLATFORM_AR_RELEASE_MINOR_VERSION    10U
#define PLATFORM_AR_RELEASE_REVISION_VERSION 0U
#include <stdbool.h>
#include <stdint.h>

#define CPU_TYPE_8      8u             /**< SWS_Platform_00064: 8-bit register width. */
#define CPU_TYPE_16     16u            /**< SWS_Platform_00064: 16-bit register width. */
#define CPU_TYPE_32     32u            /**< SWS_Platform_00064: 32-bit register width. */
#define CPU_TYPE_64     64u            /**< SWS_Platform_00064: 64-bit register width. */
#define CPU_TYPE        CPU_TYPE_64    /**< SWS_Platform_00064: controlled x64 CPU width. */
#define MSB_FIRST       0u             /**< SWS_Platform_00038: most significant bit first. */
#define LSB_FIRST       1u             /**< SWS_Platform_00038: least significant bit first. */
#define CPU_BIT_ORDER   LSB_FIRST      /**< SWS_Platform_00038: little-endian bits. */
#define HIGH_BYTE_FIRST 0u             /**< SWS_Platform_00039: high byte at lower address. */
#define LOW_BYTE_FIRST  1u             /**< SWS_Platform_00039: low byte at lower address. */
#define CPU_BYTE_ORDER  LOW_BYTE_FIRST /**< SWS_Platform_00039: little-endian bytes. */
#ifndef TRUE
#define TRUE true /**< SWS_Platform_00056: boolean true. */
#endif
#ifndef FALSE
#define FALSE false /**< SWS_Platform_00056: boolean false. */
#endif

#ifdef __cplusplus
/** SWS_Platform_00026: Boolean value; use TRUE and FALSE. */
typedef bool boolean;
#else
/** SWS_Platform_00026: Boolean value; use TRUE and FALSE. */
typedef _Bool boolean;
#endif
/** SWS_Platform_00016: Signed 8-bit value. */
typedef int8_t sint8;
/** SWS_Platform_00017: Signed 16-bit value. */
typedef int16_t sint16;
/** SWS_Platform_00018: Signed 32-bit value. */
typedef int32_t sint32;
/** SWS_Platform_00067: Signed 64-bit value. */
typedef int64_t sint64;
/** SWS_Platform_00013: Unsigned 8-bit value. */
typedef uint8_t uint8;
/** SWS_Platform_00014: Unsigned 16-bit value. */
typedef uint16_t uint16;
/** SWS_Platform_00015: Unsigned 32-bit value. */
typedef uint32_t uint32;
/** SWS_Platform_00066: Unsigned 64-bit value. */
typedef uint64_t uint64;
/** SWS_Platform_00023: Signed value of at least 8 bits. */
typedef int_least8_t sint8_least;
/** SWS_Platform_00024: Signed value of at least 16 bits. */
typedef int_least16_t sint16_least;
/** SWS_Platform_00025: Signed value of at least 32 bits. */
typedef int_least32_t sint32_least;
/** SWS_Platform_00020: Unsigned value of at least 8 bits. */
typedef uint_least8_t uint8_least;
/** SWS_Platform_00021: Unsigned value of at least 16 bits. */
typedef uint_least16_t uint16_least;
/** SWS_Platform_00022: Unsigned value of at least 32 bits. */
typedef uint_least32_t uint32_least;
/** SWS_Platform_00041: IEEE 754 binary32 value. */
typedef float float32;
/** SWS_Platform_00042: IEEE 754 binary64 value. */
typedef double float64;
/** SWS_Platform_91001: Pointer to caller result storage. */
typedef void *VoidPtr;
/** SWS_Platform_91002: Pointer for read-only input access. */
typedef const void *ConstVoidPtr;
#endif
