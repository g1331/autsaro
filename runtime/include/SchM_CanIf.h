/** @file Recursive callback admission and consumer state area. Acquire before the driver
 * lock; keep it through callback dispatch and lifecycle mutation. */
#ifndef SCHM_CANIF_H
#define SCHM_CANIF_H
void SchM_Enter_CanIf_CANIF_STATE(void);
void SchM_Exit_CanIf_CANIF_STATE(void);
#endif
