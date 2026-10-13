/** @file Controlled x64 platform types for the standard multi-component stack. */
#ifndef PLATFORM_TYPES_H
#define PLATFORM_TYPES_H
#include <stdbool.h>
#include <stdint.h>

#define CPU_TYPE_8 8u
#define CPU_TYPE_16 16u
#define CPU_TYPE_32 32u
#define CPU_TYPE_64 64u
#define CPU_TYPE CPU_TYPE_64
#define MSB_FIRST 0u
#define LSB_FIRST 1u
#define CPU_BIT_ORDER LSB_FIRST
#define HIGH_BYTE_FIRST 0u
#define LOW_BYTE_FIRST 1u
#define CPU_BYTE_ORDER LOW_BYTE_FIRST
#ifndef TRUE
#define TRUE true
#endif
#ifndef FALSE
#define FALSE false
#endif

typedef _Bool boolean;
typedef int8_t sint8;
typedef int16_t sint16;
typedef int32_t sint32;
typedef int64_t sint64;
typedef uint8_t uint8;
typedef uint16_t uint16;
typedef uint32_t uint32;
typedef uint64_t uint64;
typedef int_least8_t sint8_least;
typedef int_least16_t sint16_least;
typedef int_least32_t sint32_least;
typedef uint_least8_t uint8_least;
typedef uint_least16_t uint16_least;
typedef uint_least32_t uint32_least;
typedef float float32;
typedef double float64;
typedef void *VoidPtr;
typedef const void *ConstVoidPtr;
#endif
