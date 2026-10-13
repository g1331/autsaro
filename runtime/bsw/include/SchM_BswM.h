/** @file BswM scheduler contract for the selected all-IMMEDIATE configuration.
 * Exclusive-area bodies use the actual common host CAN resource.
 */
#ifndef SCHM_BSWM_H
#define SCHM_BSWM_H
void BswM_MainFunction(void);
void SchM_Enter_BswM_BSWM_STATE(void);
void SchM_Exit_BswM_BSWM_STATE(void);
#endif
