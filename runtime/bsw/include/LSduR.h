/** @file R24-11 LSduR: immutable one-to-one, no-metadata intra-partition routes. */
#ifndef LSDUR_H
#define LSDUR_H
#include "ComStack_Types.h"
typedef uint16 LSduR_PBConfigIdType;
typedef enum { LSDUR_UNINIT = 0, LSDUR_ONLINE = 1 } LSduR_StateType;
typedef enum { LSDUR_UP_PDUR = 0, LSDUR_UP_CANTP = 1 } LSduR_UpperType;
typedef void (*LSduR_RxType)(PduIdType id, const PduInfoType *info);
typedef void (*LSduR_ConfirmationType)(PduIdType id, Std_ReturnType result);
typedef Std_ReturnType (*LSduR_TriggerType)(PduIdType id, PduInfoType *info);
typedef struct {
    PduIdType lower;
    PduIdType upper;
    LSduR_RxType indication;
} LSduR_RxRouteType;
typedef struct {
    LSduR_UpperType module;
    PduIdType upper;
    PduIdType lower;
    LSduR_ConfirmationType confirmation;
    LSduR_TriggerType trigger;
} LSduR_TxRouteType;
typedef struct {
    LSduR_PBConfigIdType id;
    const LSduR_RxRouteType *receive;
    uint16 receive_count;
    const LSduR_TxRouteType *transmit;
    uint16 transmit_count;
} LSduR_PBConfigType;
void LSduR_Init(const LSduR_PBConfigType *ConfigPtr);
LSduR_PBConfigIdType LSduR_GetConfigurationId(void);
#endif
