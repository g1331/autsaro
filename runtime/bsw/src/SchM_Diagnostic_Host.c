/** Diagnostic buffers share the recursive CanIf admission resource with callbacks. */
#include "ComStack_Cfg.h"
#include "SchM_Dcm.h"
#include "SchM_CanTp.h"
#include "SchM_CanIf.h"
void SchM_Enter_Dcm_DCM_STATE(void) { SchM_Enter_CanIf_CANIF_STATE(); }
void SchM_Exit_Dcm_DCM_STATE(void) { SchM_Exit_CanIf_CANIF_STATE(); }
void SchM_Enter_CanTp_CANTP_STATE(void) { SchM_Enter_CanIf_CANIF_STATE(); }
void SchM_Exit_CanTp_CANTP_STATE(void) { SchM_Exit_CanIf_CANIF_STATE(); }

