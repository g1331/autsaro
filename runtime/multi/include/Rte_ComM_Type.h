/** @file R24-11 ComM service types (SWS_ComM_00670/00672).
 * Final RTE producer ownership must be selected by the generated profile.
 */
#ifndef RTE_COMM_TYPE_H
#define RTE_COMM_TYPE_H
#include "Std_Types.h"
typedef uint8 ComM_ModeType;
typedef uint16 ComM_UserHandleType;
#define COMM_NO_COMMUNICATION 0u
#define COMM_SILENT_COMMUNICATION 1u
#define COMM_FULL_COMMUNICATION 2u
#define COMM_FULL_COMMUNICATION_WITH_WAKEUP_REQUEST 3u
#define COMM_NOT_USED_USER_ID 65535u
#endif
