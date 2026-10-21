/** @file Shared RTE service types for the selected OS (SWS_BSW_00147). */
#ifndef RTE_TYPE_H
#define RTE_TYPE_H
#include "Rte_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Rte_Type.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Rte_Type.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/** SWS_Os_00786 Counter handle; full uint32 domain, independent of table capacity. */
typedef uint32 CounterType;
/** SWS_Os_00794 OS Counter service value carrier; values are ticks (SWS_Os_00560). */
typedef uint64 TimeInMicrosecondsType;
#endif
