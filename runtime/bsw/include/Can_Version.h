/** @file Can AUTOSAR release identity shared by its delivered headers. */
#ifndef CAN_VERSION_H
#define CAN_VERSION_H
#include "Autsaro_Version.h"
/* R24-11 MOD DOC-REVISION; SWS_BSW_00059/00256 release identity.
 * Formal vendor identity remains unassigned. */
#define CAN_AR_RELEASE_MAJOR_VERSION    4U
#define CAN_AR_RELEASE_MINOR_VERSION    10U
#define CAN_AR_RELEASE_REVISION_VERSION 0U
/** SWS_BSW_00059/00256: project implementation software release. */
#define CAN_SW_MAJOR_VERSION AUTSARO_SW_MAJOR_VERSION
#define CAN_SW_MINOR_VERSION AUTSARO_SW_MINOR_VERSION
#define CAN_SW_PATCH_VERSION AUTSARO_SW_PATCH_VERSION
#endif
