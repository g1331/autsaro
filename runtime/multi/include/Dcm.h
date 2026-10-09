/** @file Selected physical UDS connection with configured buffers up to 256 bytes.
 * Configuration and callouts remain valid until the next Init. Only the
 * configured channel/PDU identities are accepted; failed copies preserve outputs.
 */
#ifndef DCM_H
#define DCM_H
#include "ComStack_Types.h"
#include "Rte_Dcm_Type.h"
typedef Std_ReturnType (*Dcm_ReadDidType)(uint8 *data);
/** Generated connection-specific SchM publisher; no authentication service is selected. */
typedef void (*Ecu_DcmAuthenticationModeType)(uint8 state);
typedef struct {
    PduIdType receive;
    PduIdType transmit;
    NetworkHandleType channel;
    uint16 did;
    /** NULL selects no application DID; internal session DID F186 remains available. */
    Dcm_ReadDidType read;
    uint16 p2_ticks;
    uint16 p2_ms;
    uint32 p2_star_ms;
    uint32 s3_ticks;
    PduLengthType buffer_length;
    /** Router transmit request ID, distinct from Dcm's own confirmation ID. */
    PduIdType router_transmit;
    Ecu_DcmAuthenticationModeType authentication_mode;
} Dcm_ConfigType;
void Dcm_Init(const Dcm_ConfigType *ConfigPtr);
/** Selected DevErrorDetect=false: all three APIs always return E_OK.
 * Uninitialized or NULL getters preserve output; reset before Init has no action.
 */
Std_ReturnType Dcm_GetSecurityLevel(Dcm_SecLevelType *SecLevel);
Std_ReturnType Dcm_GetSesCtrlType(Dcm_SesCtrlType *SesCtrlType);
Std_ReturnType Dcm_ResetToDefaultSession(void);
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
