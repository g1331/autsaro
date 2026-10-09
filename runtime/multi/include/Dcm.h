/** @file Selected physical UDS connection with configured buffers up to 256 bytes.
 * Configuration and callouts remain valid until the next Init. Only the
 * configured channel/PDU identities are accepted; failed copies preserve outputs.
 */
#ifndef DCM_H
#define DCM_H
#include "ComStack_Types.h"
typedef Std_ReturnType (*Dcm_ReadDidType)(uint8 *data);
typedef struct {
    PduIdType receive;
    PduIdType transmit;
    NetworkHandleType channel;
    uint16 did;
    Dcm_ReadDidType read;
    uint16 p2_ticks;
    uint16 p2_ms;
    uint16 p2_star_ms;
    uint32 s3_ticks;
    PduLengthType buffer_length;
} Dcm_ConfigType;
void Dcm_Init(const Dcm_ConfigType *ConfigPtr);
Std_ReturnType Dcm_SetActiveDiagnostic(boolean active);
BufReq_ReturnType Dcm_StartOfReception(PduIdType id, const PduInfoType *info,
                                       PduLengthType TpSduLength, PduLengthType *bufferSizePtr);
BufReq_ReturnType Dcm_CopyRxData(PduIdType id, const PduInfoType *info,
                                 PduLengthType *bufferSizePtr);
void Dcm_TpRxIndication(PduIdType id, Std_ReturnType result);
BufReq_ReturnType Dcm_CopyTxData(PduIdType id, const PduInfoType *info, const RetryInfoType *retry,
                                 PduLengthType *availableDataPtr);
void Dcm_TpTxConfirmation(PduIdType id, Std_ReturnType result);
#endif
