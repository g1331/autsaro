/** Standard selected Dcm service types and diagnostic session mode mapping. */
#ifndef RTE_DCM_TYPE_H
#define RTE_DCM_TYPE_H
#include "Std_Types.h"
typedef uint8 Dcm_SecLevelType;
typedef uint8 Dcm_SesCtrlType;
#define DCM_SEC_LEV_LOCKED 0u
#define DCM_DEFAULT_SESSION 1u
#define DCM_PROGRAMMING_SESSION 2u
#define DCM_EXTENDED_DIAGNOSTIC_SESSION 3u
#define DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 4u
typedef uint8 Rte_ModeType_DcmDiagnosticSessionControl;
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION 0u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_PROGRAMMING_SESSION 1u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION 2u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 3u
#define RTE_TRANSITION_DcmDiagnosticSessionControl 255u
#endif
