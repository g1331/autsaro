/** @file Selected standard PDU routing, immutable one-to-one routes. */
#ifndef PDUR_H
#define PDUR_H
#include "Std_Types.h"
#include "ComStack_Types.h"
typedef uint16 PduR_PBConfigIdType;
typedef uint16 PduR_RoutingPathGroupIdType;
typedef enum { PDUR_UNINIT = 0, PDUR_ONLINE = 1 } PduR_StateType;
typedef enum { PDUR_UP_COM = 0, PDUR_UP_DCM = 1 } PduR_UpperType;
/* Lower module of a transmit route: each owns a separate handle namespace. */
typedef enum { PDUR_LO_LSDUR = 0, PDUR_LO_CANTP = 1 } PduR_LowerType;
typedef Std_ReturnType (*PduR_TransmitType)(PduIdType id, const PduInfoType *info);
typedef void (*PduR_ReceiveType)(PduIdType id, const PduInfoType *info);
typedef void (*PduR_ConfirmationType)(PduIdType id, Std_ReturnType result);
typedef Std_ReturnType (*PduR_TriggerType)(PduIdType id, PduInfoType *info);
typedef BufReq_ReturnType (*PduR_StartType)(PduIdType id,
                                            const PduInfoType *info,
                                            PduLengthType length,
                                            PduLengthType *available);
typedef BufReq_ReturnType (*PduR_CopyRxType)(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType *available);
typedef BufReq_ReturnType (*PduR_CopyTxType)(PduIdType id,
                                             const PduInfoType *info,
                                             const RetryInfoType *retry,
                                             PduLengthType *available);
typedef struct {
    PduIdType lower_rx;
    PduIdType upper_rx;
    PduIdType lower_tx;
    PduIdType upper_tx;
    PduR_StartType start;
    PduR_CopyRxType copy_rx;
    PduR_ConfirmationType receive;
    PduR_CopyTxType copy_tx;
    PduR_ConfirmationType confirmation;
} PduR_TpRouteType;
typedef struct {
    PduIdType lower;
    PduIdType upper;
    PduR_ReceiveType indication;
} PduR_RxRouteType;
typedef struct {
    PduR_UpperType module;
    PduIdType upper;
    PduR_LowerType lower_module;
    PduIdType lower;
    PduR_TransmitType transmit;
    PduR_ConfirmationType confirmation;
    PduR_TriggerType trigger;
} PduR_TxRouteType;
typedef struct {
    PduR_PBConfigIdType id;
    const PduR_RxRouteType *receive;
    uint16 receive_count;
    const PduR_TxRouteType *transmit;
    uint16 transmit_count;
    const PduR_TpRouteType *transport;
    uint16 transport_count;
} PduR_PBConfigType;
void PduR_Init(const PduR_PBConfigType *ConfigPtr);
PduR_PBConfigIdType PduR_GetConfigurationId(void);
void PduR_EnableRouting(PduR_RoutingPathGroupIdType id);
void PduR_DisableRouting(PduR_RoutingPathGroupIdType id, boolean initialize);
#endif
