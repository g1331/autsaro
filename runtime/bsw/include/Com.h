/** @file Selected R24-11 immediate UINT32 COM interface and immutable configuration. */
#ifndef COM_H
#define COM_H
#include "Std_Types.h"
#include "ComStack_Types.h"

typedef enum { COM_UNINIT = 0, COM_INIT = 1 } Com_StatusType;
typedef uint16 Com_SignalIdType;
typedef uint16 Com_IpduGroupIdType;
#define COM_SERVICE_NOT_AVAILABLE 0x80u
/* MR12 RULE 2.5 VIOLATION: standard return code (SWS_Com_00865) that is never returned,
 * because the selected profile has no large or TP I-PDUs; see the Com deviation record. */
#define COM_BUSY 0x81u

/** One four-byte little-endian signal per PDU, no update bits or metadata.
 * Timer counts are configured Rx main-function periods, not host timestamps.
 */
typedef struct {
    PduIdType pdu;
    Com_SignalIdType signal;
    boolean receive;
    uint32 initial_value;
    uint32 timeout_ticks;
    CbkHandleIdType callback_handle;
} Com_PduConfigType;
typedef void (*Com_NotificationType)(CbkHandleIdType handle);
typedef Std_ReturnType (*Com_TransmitType)(PduIdType id, const PduInfoType *info);
typedef struct {
    const Com_PduConfigType *pdus;
    uint16 pdu_count;
    Com_IpduGroupIdType receive_group;
    Com_NotificationType receive_notification;
    Com_NotificationType timeout_notification;
    Com_TransmitType transmit;
} Com_ConfigType;

/** Configuration and referenced storage remain valid until DeInit.
 * Selected profile: at most 32 PDUs, one Rx group, first timeout zero, NONE.
 */
void Com_Init(const Com_ConfigType *config);
void Com_DeInit(void);
Com_StatusType Com_GetStatus(void);
uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr);
uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr);
uint8 Com_InvalidateSignal(Com_SignalIdType SignalId);
void Com_IpduGroupStart(Com_IpduGroupIdType IpduGroupId, boolean Initialize);
void Com_IpduGroupStop(Com_IpduGroupIdType IpduGroupId);
void Com_EnableReceptionDM(Com_IpduGroupIdType IpduGroupId);
void Com_DisableReceptionDM(Com_IpduGroupIdType IpduGroupId);
void Com_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
/** Send the current data at once; no deferred trigger is stored. E_OK while initialized
 * for a Tx PDU: selected Tx PDUs have no group and are started by Init (00840), with zero
 * MDT and no retry, so a lower rejection is not reported here.
 */
Std_ReturnType Com_TriggerIPDUSend(PduIdType PduId);
/** Mode FALSE selects NONE, the default of the absent ComTxModeFalse; Com_TriggerIPDUSend
 * still sends. Only Tx PDUs have a transmission mode.
 */
void Com_SwitchIpduTxMode(PduIdType PduId, boolean Mode);
Std_ReturnType Com_TriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
void Com_TxConfirmation(PduIdType TxPduId, Std_ReturnType result);
#endif
