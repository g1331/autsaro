/** @file Standard BSW statuses for the controlled multi-component stack. */
#ifndef STD_TYPES_H
#define STD_TYPES_H
#include "Autsaro_Version.h"
/** Published information (SWS_BSW_00059): fixed R24-11 interface release, 4.10.0. */
#define STD_TYPES_AR_RELEASE_MAJOR_VERSION    4U
#define STD_TYPES_AR_RELEASE_MINOR_VERSION    10U
#define STD_TYPES_AR_RELEASE_REVISION_VERSION 0U
/** SWS_BSW_00059/00256: project implementation software release. */
#define STD_TYPES_SW_MAJOR_VERSION AUTSARO_SW_MAJOR_VERSION
#define STD_TYPES_SW_MINOR_VERSION AUTSARO_SW_MINOR_VERSION
#define STD_TYPES_SW_PATCH_VERSION AUTSARO_SW_PATCH_VERSION
#include "Platform_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(PLATFORM_AR_RELEASE_MAJOR_VERSION) || !defined(PLATFORM_AR_RELEASE_MINOR_VERSION)
#error "Std_Types.h -> Platform_Types.h: missing AUTOSAR release"
#elif (PLATFORM_AR_RELEASE_MAJOR_VERSION != 4U) || (PLATFORM_AR_RELEASE_MINOR_VERSION != 10U)
#error "Std_Types.h -> Platform_Types.h: incompatible AUTOSAR release"
#endif

/** SWS_Std_00005: API return: E_OK/E_NOT_OK; 0x02..0x3f for module errors. */
typedef uint8 Std_ReturnType;
/** SWS_Std_00015: vendor/module and software version returned by GetVersionInfo. */
typedef struct {
    uint16 vendorID;        /**< Registered vendor identifier. */
    uint16 moduleID;        /**< AUTOSAR BSW module identifier. */
    uint8 sw_major_version; /**< Software major version. */
    uint8 sw_minor_version; /**< Software minor version. */
    uint8 sw_patch_version; /**< Software patch version. */
} Std_VersionInfoType;
#define NULL_PTR   ((void *)0) /**< SWS_Std_00031: null void pointer. */
#define E_OK       0x00u       /**< SWS_Std_00006: operation succeeded. */
#define E_NOT_OK   0x01u       /**< SWS_Std_00006: operation failed. */
#define STD_HIGH   0x01u       /**< SWS_Std_00007: physical high level. */
#define STD_LOW    0x00u       /**< SWS_Std_00007: physical low level. */
#define STD_ACTIVE 0x01u       /**< SWS_Std_00013: logical active state. */
#define STD_IDLE   0x00u       /**< SWS_Std_00013: logical idle state. */
#define STD_ON     0x01u       /**< SWS_Std_00010: enabled configuration value. */
#define STD_OFF    0x00u       /**< SWS_Std_00010: disabled configuration value. */
#endif
