/** @file R24-11 ComM service types (SWS_ComM_00669/00670/00672).
 * Final RTE producer ownership must be selected by the generated profile.
 */
#ifndef RTE_COMM_TYPE_H
#define RTE_COMM_TYPE_H
#include "Std_Types.h"
/** SWS_ComM_00672: Selected channel communication mode. */
typedef uint8 ComM_ModeType;
/** SWS_ComM_00669: Bit mask of active mode inhibitions. */
typedef uint8 ComM_InhibitionStatusType;
/** SWS_ComM_00670: Generated user ID, 0..65534; 65535 is reserved. */
typedef uint16 ComM_UserHandleType;
#define COMM_NO_COMMUNICATION     0u /**< Neither receive nor transmit is allowed. */
#define COMM_SILENT_COMMUNICATION 1u /**< Receive only; transmission is disabled. */
#define COMM_FULL_COMMUNICATION   2u /**< Receive and transmit are enabled. */
/* MR12 RULE 2.5 VIOLATION: standard mode (SWS_ComM_00672) that the selected
 * profile never requests; see the ComM deviation record. */
#define COMM_FULL_COMMUNICATION_WITH_WAKEUP_REQUEST                                                \
    3u /**< Internal full-communication wakeup mode; not selected here. */
#define COMM_NOT_USED_USER_ID 65535u /**< Reserved value; never a generated user handle. */
/* ComM_InhibitionStatusType bit fields (SWS_ComM_00669), named per
 * SWS_Rte_03995-03997. */
#ifndef WakeupInhibitionActive_BflMask
#define WakeupInhibitionActive_BflMask 1U /**< Inhibition bit mask. */
#endif                                    /* WakeupInhibitionActive_BflMask */
#ifndef WakeupInhibitionActive_BflPn
#define WakeupInhibitionActive_BflPn 0U /**< Zero-based inhibition bit position. */
#endif                                  /* WakeupInhibitionActive_BflPn */
#ifndef WakeupInhibitionActive_BflLn
#define WakeupInhibitionActive_BflLn                                                               \
    1U /**< Inhibition field width in bits.                                                        \
        */
#endif /* WakeupInhibitionActive_BflLn */
#ifndef LimitedToNoCom_BflMask
#define LimitedToNoCom_BflMask 2U /**< Inhibition bit mask. */
#endif                            /* LimitedToNoCom_BflMask */
#ifndef LimitedToNoCom_BflPn
#define LimitedToNoCom_BflPn 1U /**< Zero-based inhibition bit position. */
#endif                          /* LimitedToNoCom_BflPn */
#ifndef LimitedToNoCom_BflLn
#define LimitedToNoCom_BflLn 1U /**< Inhibition field width in bits. */
#endif                          /* LimitedToNoCom_BflLn */
#endif
