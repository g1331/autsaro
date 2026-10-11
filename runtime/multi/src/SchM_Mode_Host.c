/** @file Mode modules share the recursive CanIf admission resource.
 * Configured exclusive-area declarations/provenance belong to full generation.
 */
#include "SchM_ComM.h"
#include "SchM_BswM.h"
#include "SchM_CanIf.h"
void SchM_Enter_ComM_COMM_STATE(void) { SchM_Enter_CanIf_CANIF_STATE(); }
void SchM_Exit_ComM_COMM_STATE(void) { SchM_Exit_CanIf_CANIF_STATE(); }
void SchM_Enter_BswM_BSWM_STATE(void) { SchM_Enter_CanIf_CANIF_STATE(); }
void SchM_Exit_BswM_BSWM_STATE(void) { SchM_Exit_CanIf_CANIF_STATE(); }
