/** @file Os AUTOSAR release identity shared by its delivered headers. */
#ifndef OS_VERSION_H
#define OS_VERSION_H
#include "Autsaro_Version.h"
/* R24-11 MOD DOC-REVISION; SWS_BSW_00059/00256 release identity.
 * Formal vendor identity remains unassigned. */
#define OS_AR_RELEASE_MAJOR_VERSION    4U
#define OS_AR_RELEASE_MINOR_VERSION    10U
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define OS_AR_RELEASE_REVISION_VERSION 0U
/** SWS_BSW_00059/00256: project implementation software release. */
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define OS_SW_MAJOR_VERSION AUTSARO_SW_MAJOR_VERSION
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define OS_SW_MINOR_VERSION AUTSARO_SW_MINOR_VERSION
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define OS_SW_PATCH_VERSION AUTSARO_SW_PATCH_VERSION
#endif
