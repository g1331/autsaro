/** @file R24-11 common communication types; widths come from ECU configuration. */
#ifndef COMSTACK_TYPES_H
#define COMSTACK_TYPES_H
/** Published information (SWS_BSW_00059): fixed R24-11 interface release, 4.10.0. */
#define COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION    4U
#define COMSTACKTYPE_AR_RELEASE_MINOR_VERSION    10U
#define COMSTACKTYPE_AR_RELEASE_REVISION_VERSION 0U
#include "Std_Types.h"
#include "ComStack_Cfg.h"

/** SWS_COMTYPE_00011: payload, configured metadata and payload byte length. */
typedef struct {
    uint8 *SduDataPtr;       /**< Borrowed payload pointer; lifetime follows the called API. */
    uint8 *MetaDataPtr;      /**< Configured metadata; multi-byte items use platform byte order. */
    PduLengthType SduLength; /**< SDU payload length in bytes. */
} PduInfoType;
/** SWS_COMTYPE_00012: result of a communication buffer request. */
typedef enum {
    BUFREQ_OK = 0,       /**< Buffer request succeeded. */
    BUFREQ_E_NOT_OK = 1, /**< Buffer cannot be accessed. */
    BUFREQ_E_BUSY = 2,   /**< Temporarily unavailable; caller may retry. */
    BUFREQ_E_OVFL = 3    /**< Requested buffer length cannot be provided. */
} BufReq_ReturnType;
/** SWS_COMTYPE_00027: confirmation/retry state for TP buffer data. */
typedef enum {
    TP_DATACONF = 0,   /**< Earlier copied data confirmed; excludes data copied by this call. */
    TP_DATARETRY = 1,  /**< Copy earlier data again from the supplied retry offset. */
    TP_CONFPENDING = 2 /**< Retain previously copied data pending confirmation. */
} TpDataStateType;
/** SWS_COMTYPE_00037: transport buffer state and retry byte offset. */
typedef struct {
    TpDataStateType TpDataState; /**< State selecting confirmation or retry handling. */
    PduLengthType TxTpDataCnt;   /**< Retry byte offset relative to current position. */
} RetryInfoType;
/** SWS_COMTYPE_00038: Communication channel identifier, zero-based 0..255. */
typedef uint8 NetworkHandleType;
/** R24-11 SWS_COMTYPE_91001 draft notification handle. */
typedef uint16 CbkHandleIdType;
#endif
