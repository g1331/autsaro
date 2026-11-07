/** @file ComM AUTOSAR release identity shared by its delivered headers. */
#ifndef COMM_VERSION_H
#define COMM_VERSION_H
#include "Autsaro_Version.h"
/* Module identity (SWS_BSW_00059/00256, Appendix A.2).
 * MR12 RULE 2.5 VIOLATION: SWS_BSW_00059/00256 requires these immutable published identity fields.
 */
#define COMM_MODULE_ID 12u
/* R24-11 MOD DOC-REVISION; SWS_BSW_00059/00256 release identity.
 * Formal vendor identity remains unassigned. */
#define COMM_AR_RELEASE_MAJOR_VERSION    4U
#define COMM_AR_RELEASE_MINOR_VERSION    10U
#define COMM_AR_RELEASE_REVISION_VERSION 0U
/** SWS_BSW_00059/00256: project implementation software release. */
#define COMM_SW_MAJOR_VERSION AUTSARO_SW_MAJOR_VERSION
#define COMM_SW_MINOR_VERSION AUTSARO_SW_MINOR_VERSION
#define COMM_SW_PATCH_VERSION AUTSARO_SW_PATCH_VERSION
#endif
