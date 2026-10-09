/** Selected Dcm scheduler and actual diagnostic-session mode producer. */
#ifndef SCHM_DCM_H
#define SCHM_DCM_H
#include "Rte_Dcm_Type.h"
void SchM_Enter_Dcm_DCM_STATE(void);
void SchM_Exit_Dcm_DCM_STATE(void);
void Dcm_MainFunction(void);
Std_ReturnType
SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode);
Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void);
#endif
