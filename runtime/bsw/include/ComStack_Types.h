/** @file R24-11 common communication types; widths come from ECU configuration. */
#ifndef COMSTACK_TYPES_H
#define COMSTACK_TYPES_H
#include "Std_Types.h"
#include "ComStack_Cfg.h"

typedef struct {
    uint8 *SduDataPtr;
    uint8 *MetaDataPtr;
    PduLengthType SduLength;
} PduInfoType;
typedef enum {
    BUFREQ_OK = 0,
    BUFREQ_E_NOT_OK = 1,
    BUFREQ_E_BUSY = 2,
    BUFREQ_E_OVFL = 3
} BufReq_ReturnType;
typedef enum { TP_DATACONF = 0, TP_DATARETRY = 1, TP_CONFPENDING = 2 } TpDataStateType;
typedef struct {
    TpDataStateType TpDataState;
    PduLengthType TxTpDataCnt;
} RetryInfoType;
typedef uint8 NetworkHandleType;
/** R24-11 SWS_COMTYPE_91001 draft notification handle. */
typedef uint16 CbkHandleIdType;
#endif
