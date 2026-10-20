/** @file Shared RTE service types for the selected OS (SWS_BSW_00147). */
#ifndef AUTOSAR_RTE_TYPE_H
#define AUTOSAR_RTE_TYPE_H
#include "Std_Types.h"
/** SWS_Os_00786 Counter handle; full uint32 domain, independent of table capacity. */
typedef uint32 CounterType;
/** SWS_Os_00794 OS Counter service value carrier; values are ticks (SWS_Os_00560). */
typedef uint64 TimeInMicrosecondsType;
#endif
