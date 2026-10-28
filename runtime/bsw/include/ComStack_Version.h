/** @file Communication stack type release identity shared by its headers. */
#ifndef COMSTACK_VERSION_H
#define COMSTACK_VERSION_H
#include "Autsaro_Version.h"
/** Published information (SWS_BSW_00059): fixed R24-11 interface release, 4.10.0. */
#define COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION    4U
#define COMSTACKTYPE_AR_RELEASE_MINOR_VERSION    10U
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COMSTACKTYPE_AR_RELEASE_REVISION_VERSION 0U
/** SWS_BSW_00059/00256: project implementation software release. */
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COMSTACKTYPE_SW_MAJOR_VERSION AUTSARO_SW_MAJOR_VERSION
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COMSTACKTYPE_SW_MINOR_VERSION AUTSARO_SW_MINOR_VERSION
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COMSTACKTYPE_SW_PATCH_VERSION AUTSARO_SW_PATCH_VERSION
#endif
