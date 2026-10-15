/** @file R24-11 ComM service types (SWS_ComM_00669/00670/00672).
 * Final RTE producer ownership must be selected by the generated profile.
 */
#ifndef RTE_COMM_TYPE_H
#define RTE_COMM_TYPE_H
#include "Std_Types.h"
typedef uint8 ComM_ModeType;
typedef uint8 ComM_InhibitionStatusType;
typedef uint16 ComM_UserHandleType;
#define COMM_NO_COMMUNICATION                       0u
#define COMM_SILENT_COMMUNICATION                   1u
#define COMM_FULL_COMMUNICATION                     2u
#define COMM_FULL_COMMUNICATION_WITH_WAKEUP_REQUEST 3u
#define COMM_NOT_USED_USER_ID                       65535u
/* ComM_InhibitionStatusType bit fields (SWS_ComM_00669), named per SWS_Rte_03995-03997. */
#ifndef WakeupInhibitionActive_BflMask
#define WakeupInhibitionActive_BflMask 1U
#endif /* WakeupInhibitionActive_BflMask */
#ifndef WakeupInhibitionActive_BflPn
#define WakeupInhibitionActive_BflPn 0U
#endif /* WakeupInhibitionActive_BflPn */
#ifndef WakeupInhibitionActive_BflLn
#define WakeupInhibitionActive_BflLn 1U
#endif /* WakeupInhibitionActive_BflLn */
#ifndef LimitedToNoCom_BflMask
#define LimitedToNoCom_BflMask 2U
#endif /* LimitedToNoCom_BflMask */
#ifndef LimitedToNoCom_BflPn
#define LimitedToNoCom_BflPn 1U
#endif /* LimitedToNoCom_BflPn */
#ifndef LimitedToNoCom_BflLn
#define LimitedToNoCom_BflLn 1U
#endif /* LimitedToNoCom_BflLn */
#endif
