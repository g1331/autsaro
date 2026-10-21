/** @file Compile-time OS status and error-access switches, validated here. */
#ifndef AUTOSAR_OS_CFG_H
#define AUTOSAR_OS_CFG_H
#include "Os_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Os_Cfg.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Cfg.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/* Both status modes retain defensive host checks and nonzero ErrorHook
 * reporting. The reference ECU selects Extended Status; generated headers
 * select the validated ECUC value. Standard warnings remain mandatory. */
/** ECUC_Os_00046: default EXTENDED status; generator/compiler may override. */
#ifndef OS_STATUS_EXTENDED
#define OS_STATUS_EXTENDED STD_ON
#endif
#if (OS_STATUS_EXTENDED != STD_OFF) && (OS_STATUS_EXTENDED != STD_ON)
#error Invalid OS status configuration
#endif
/* The fixed reference ECU selects both standard error-access switches.
 * Native configuration tests compile each independent off/on combination. */
/** ECUC_Os_00047: default service-ID access; generator/compiler may override. */
#ifndef OS_USE_GET_SERVICE_ID
#define OS_USE_GET_SERVICE_ID STD_ON
#endif
/** ECUC_Os_00048: default argument access; generator/compiler may override. */
#ifndef OS_USE_PARAMETER_ACCESS
#define OS_USE_PARAMETER_ACCESS STD_ON
#endif
#if ((OS_USE_GET_SERVICE_ID != STD_OFF) && (OS_USE_GET_SERVICE_ID != STD_ON)) ||                   \
    ((OS_USE_PARAMETER_ACCESS != STD_OFF) && (OS_USE_PARAMETER_ACCESS != STD_ON))
#error Invalid OS error-access configuration
#endif
#endif
