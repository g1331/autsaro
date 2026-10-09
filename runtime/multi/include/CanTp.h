/** @file One physical, half-duplex, padded Classical CAN connection.
 * Full upper buffers are reserved at admission; no partial-buffer WAIT is selected.
 * Timer values are counts of the configured main-function period, never host time.
 */
#ifndef CANTP_H
#define CANTP_H
#include "ComStack_Types.h"
typedef struct {
    PduIdType receive;
    PduIdType transmit;
    PduIdType lower_receive;
    PduIdType lower_transmit;
    uint16 maximum_length;
    uint32 n_as;
    uint32 n_ar;
    uint32 n_bs;
    uint32 n_cr;
    uint32 n_cs;
    uint16 main_period_ms;
    uint8 wait_frame_max;
    uint8 padding;
} CanTp_ConfigType;
void CanTp_Init(const CanTp_ConfigType *CfgPtr);
void CanTp_Shutdown(void);
Std_ReturnType CanTp_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
/** Cancel an accepted segmented reception, excluding final-CF N_Cr waiting.
 * An outstanding flow-control frame remains quarantined until its confirmation.
 */
Std_ReturnType CanTp_CancelReceive(PduIdType RxPduId);
void CanTp_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
void CanTp_TxConfirmation(PduIdType TxPduId, Std_ReturnType result);
#endif
