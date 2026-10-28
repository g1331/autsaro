/** @file Native ARTI publication cells types. */
#ifndef AUTOSAR_ARTI_TYPES_H
#define AUTOSAR_ARTI_TYPES_H
#include "Std_Types.h"
#include <stddef.h>
#ifdef _WIN32
/** Native four-byte aligned cell type required by the controlled host Interlocked ABI. */
typedef long Arti_Atomic32Type;
#else
/** Native four-byte aligned cell type required by the controlled host Interlocked ABI. */
typedef int32_t Arti_Atomic32Type;
#endif
enum {
    ARTI_ATOMIC32_WIDTH =
        sizeof(char[(sizeof(Arti_Atomic32Type) == 4u) ? 1 : -1]), /**< Reject incompatible
                                                             publication cells. */
};

#endif
