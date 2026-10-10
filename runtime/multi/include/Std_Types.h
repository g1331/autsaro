/** @file Standard BSW statuses for the controlled multi-component stack. */
#ifndef STD_TYPES_H
#define STD_TYPES_H
#include "Platform_Types.h"

typedef uint8 Std_ReturnType;
typedef struct {
    uint16 vendorID;
    uint16 moduleID;
    uint8 sw_major_version;
    uint8 sw_minor_version;
    uint8 sw_patch_version;
} Std_VersionInfoType;
#define NULL_PTR ((void *)0)
#define E_OK 0x00u
#define E_NOT_OK 0x01u
#define STD_HIGH 0x01u
#define STD_LOW 0x00u
#define STD_ACTIVE 0x01u
#define STD_IDLE 0x00u
#define STD_ON 0x01u
#define STD_OFF 0x00u
#endif
