/** @file
 * @brief R24-11 OS service implementation data types (SWS_Os_00794/00786).
 * CounterType has one definition shared with the native OS public API.
 */
#ifndef AUTOSAR_RTE_OS_TYPE_H
#define AUTOSAR_RTE_OS_TYPE_H
#include "Os_Types.h"

/** OS service value carrier (SWS_Os_00786); Counter APIs carry ticks
 * (SWS_Os_00560). */
typedef uint64 TimeInMicrosecondsType;

#endif
