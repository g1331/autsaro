/** @file One physical UDS connection with synchronous reads and caller-owned configuration. */
#ifndef DCM_H
#define DCM_H
#include "Std_Types.h"
#include "ComStack_Types.h"
#include "Rte_Dcm_Type.h"
/** SWS_Dcm_00989: one diagnostic payload byte. */
typedef uint8 Dcm_MsgItemType;
/** SWS_Dcm_00990: mutable diagnostic payload. */
typedef Dcm_MsgItemType *Dcm_MsgType;
/** SWS_Dcm_00991: payload length in bytes, excluding the service identifier. */
typedef uint32 Dcm_MsgLenType;
/** SWS_Dcm_00993: request/confirmation context identity within one protocol. */
typedef uint8 Dcm_IdContextType;
/** SWS_Dcm_00992: request attributes; the selected connection is always physical. */
typedef struct {
    uint8 reqType : 1;             /**< 0 physical, 1 functional, at LSB+0. */
    uint8 suppressPosResponse : 1; /**< 1 suppresses a positive response, at LSB+1. */
} Dcm_MsgAddInfoType;
/** SWS_Dcm_00994: call-local DSL/DSD/DSP message; payloads exclude the SID.
 * Dcm retains its own request/response arrays until this execution returns and
 * copies only a current request's completed response into the transport buffer.
 */
typedef struct {
    Dcm_MsgType reqData;           /**< First request byte after SID. */
    Dcm_MsgLenType reqDataLen;     /**< Request payload bytes. */
    Dcm_MsgType resData;           /**< First response byte after SID. */
    Dcm_MsgLenType resDataLen;     /**< Completed response payload bytes. */
    Dcm_MsgAddInfoType msgAddInfo; /**< Addressing and suppression attributes. */
    Dcm_MsgLenType resMaxDataLen;  /**< Response capacity after SID. */
    Dcm_IdContextType idContext;   /**< 0 for the selected single protocol context. */
    PduIdType dcmRxPduId;          /**< Receiving Dcm PDU ID. */
} Dcm_MsgContextType;
/** Adapter for the selected synchronous DataServices ReadData (SWS_Dcm_00793).
 * The four-byte output is call-local, and the function must finish synchronously.
 * The application supplies valid data (replacement values on failure); the adapter
 * may report an RTE infrastructure failure. Dcm then discards its partial output.
 * Dcm serializes these calls even if a callback restarts the module.
 */
typedef Std_ReturnType (*Dcm_ReadDidType)(uint8 *data);
/** Generated connection-specific SchM publisher; no authentication service is selected. */
typedef void (*Ecu_DcmAuthenticationModeType)(uint8 state);
/** Implementation-defined configuration (SWS_Dcm_00982), emitted as immutable dcm_config.
 * One physical UDS-on-CAN connection, sessions 1/3, services 0x10/0x22/0x3E and no
 * asynchronous/paged processing. The optional application DID contains four opaque
 * bytes. F186 reads the active session; OBD availability DIDs report the configured
 * data DID without calling the application. Timers count Dcm_MainFunction calls;
 * generated profiles run it every millisecond. P2* is advertised for the selected
 * sessions; synchronous ReadData never returns pending. No response is retried after
 * a lower-layer refusal or error confirmation (SWS_Dcm_00118).
 */
typedef struct {
    PduIdType receive;         /**< Dcm Rx ID (DcmDslProtocolRxPduId). */
    PduIdType transmit;        /**< Dcm confirmation ID (DcmDslTxConfirmationPduId). */
    NetworkHandleType channel; /**< ComM channel (DcmDslProtocolComMChannelRef). */
    uint16 did;           /**< Data DID; excludes F186/OBD availability (DcmDspDidIdentifier). */
    Dcm_ReadDidType read; /**< Synchronous read adapter; NULL selects only internal F186. */
    uint16 p2_ticks;      /**< Positive FullCom wait, in main periods (P2ServerMax). */
    uint16 p2_ms;         /**< Advertised P2, in ms (DcmDspSessionP2ServerMax). */
    uint32 p2_star_ms;    /**< Advertised P2*, in ms (DcmDspSessionP2StarServerMax). */
    uint32 s3_ticks;      /**< Positive idle session timeout, in main periods (S3Server). */
    PduLengthType buffer_length; /**< Rx/Tx capacity, 7..256 bytes (DcmDslBufferSize). */
    PduIdType router_transmit;   /**< PduR transmit ID, distinct from Dcm's confirmation ID. */
    Ecu_DcmAuthenticationModeType authentication_mode; /**< Non-NULL mode publisher; state 0. */
    uint16 max_did_to_read; /**< DcmDspMaxDidToRead; 0 means absent, with no count limit. */
} Dcm_ConfigType;
/** Initialize the connection, default session and deauthenticated mode (SWS_Dcm_00037).
 * @param[in] ConfigPtr Immutable selected configuration; NULL or an invalid configuration
 * leaves the module unchanged. Buffers must hold 7..256 bytes, timers must be positive,
 * did must differ from F186, and authentication_mode must not be NULL.
 * @par Return
 * None.
 * Preconditions: the old transport lifetime is quiesced before restarting an accepted
 * transmission; stop/reinitialize CanTp so it cannot later confirm an old response.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant. A restart may overtake an application call; its result is
 * discarded and the next application call waits for the old one to return.
 * Ownership: ConfigPtr and its callbacks are retained; keep old configurations valid
 * until any calls already using them have returned.
 */
void Dcm_Init(const Dcm_ConfigType *ConfigPtr);
/** Read the locked security level; SecurityAccess is not selected (SWS_Dcm_00338).
 * @param[out] SecLevel Receives DCM_SEC_LEV_LOCKED when initialized and non-NULL.
 * @return E_OK, including before Init or for NULL (DevErrorDetect is false).
 * Preconditions: None; invalid calls preserve output.
 * Context: task or callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: output is written before return; no pointer is retained.
 */
Std_ReturnType Dcm_GetSecurityLevel(Dcm_SecLevelType *SecLevel);
/** Read the active diagnostic session (SWS_Dcm_00339).
 * @param[out] SesCtrlType Receives session 1 or 3 when initialized and non-NULL.
 * @return E_OK, including before Init or for NULL (DevErrorDetect is false).
 * Preconditions: None; invalid calls preserve output.
 * Context: task or callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: output is written before return; no pointer is retained.
 */
Std_ReturnType Dcm_GetSesCtrlType(Dcm_SesCtrlType *SesCtrlType);
/** Reset the session and veto an older request's queued session change (SWS_Dcm_00520).
 * @return E_OK.
 * Preconditions: None; before Init no action is taken.
 * Context: task or callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; a reentrant mode publication is drained by the active publisher.
 * Ownership: None.
 */
Std_ReturnType Dcm_ResetToDefaultSession(void);
/** Enable or disable future ComM active-diagnostic notifications (SWS_Dcm_01068).
 * @param[in] active FALSE disables ActiveDiagnostic, TRUE enables it; inactive
 * notifications and the FullCom transmit gate remain effective in either state.
 * @return E_OK.
 * Preconditions: None; before Init no action is taken.
 * Context: task or callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
Std_ReturnType Dcm_SetActiveDiagnostic(boolean active);
/** Reserve the connection's receive buffer (SWS_Dcm_00094).
 * @param[in] id The configured Dcm Rx ID.
 * @param[in] info Optional first payload information; NULL is allowed. Metadata must be
 * NULL, length cannot exceed TpSduLength, and nonempty payload requires a data pointer.
 * The payload bytes are supplied by CopyRxData.
 * @param[in] TpSduLength Positive total size. A busy connection declines every further
 * request without a response (SWS_Dcm_00241, SWS_Dcm_01678).
 * @param[out] bufferSizePtr Non-NULL; receives the reserved capacity on success.
 * @return BUFREQ_OK when reserved, BUFREQ_E_OVFL when idle capacity is insufficient,
 * BUFREQ_E_NOT_OK for invalid input, NoCom, an unknown ID or an occupied reservation.
 * Preconditions: initialized; otherwise BUFREQ_E_NOT_OK, with output unchanged.
 * Context: TP callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; at most one reservation succeeds.
 * Ownership: info is not retained; output changes only on success.
 */
BufReq_ReturnType Dcm_StartOfReception(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType TpSduLength,
                                       PduLengthType *bufferSizePtr);
/** Copy the next segment into the reserved Rx buffer (SWS_Dcm_00556).
 * @param[in] id The configured Dcm Rx ID.
 * @param[in] info Non-NULL; SduLength 0 queries capacity and allows NULL SduDataPtr.
 * Nonzero length requires readable bytes and cannot exceed the reservation.
 * @param[out] bufferSizePtr Non-NULL; receives the remaining capacity after the copy.
 * @return BUFREQ_OK on success; BUFREQ_E_NOT_OK for invalid input, NoCom, wrong ID or
 * no matching reservation. Failure preserves the receive buffer and output size.
 * Preconditions: successful StartOfReception; otherwise BUFREQ_E_NOT_OK.
 * Context: TP callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; the TP orders segments belonging to the same reception.
 * Ownership: source bytes are copied before return; no pointer is retained.
 */
BufReq_ReturnType Dcm_CopyRxData(PduIdType id,
                                 const PduInfoType *info,
                                 PduLengthType *bufferSizePtr);
/** Complete the reserved Rx message (SWS_Dcm_00093).
 * @param[in] id The configured Dcm Rx ID; other IDs are ignored.
 * @param[in] result E_OK publishes a complete request; any other value discards it.
 * @par Return
 * None.
 * Preconditions: initialized and receiving; otherwise no action.
 * Context: TP callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; application ReadData runs only from Dcm_MainFunction.
 * Ownership: the request stays Dcm-owned until completion; no pointer is retained.
 */
void Dcm_TpRxIndication(PduIdType id, Std_ReturnType result);
/** Copy/retry the immutable response owned by an accepted TP transmission (SWS_Dcm_00092).
 * @param[in] id The configured Dcm confirmation ID.
 * @param[in] info Non-NULL destination description; SduLength 0 allows NULL SduDataPtr.
 * @param[in] retry NULL acknowledges copied bytes immediately; TP_CONFPENDING retains
 * them, TP_DATACONF confirms earlier copies, and TP_DATARETRY rewinds unconfirmed bytes.
 * @param[out] availableDataPtr Non-NULL; receives bytes remaining after this copy.
 * @return BUFREQ_OK on success; BUFREQ_E_BUSY if too many bytes were requested;
 * BUFREQ_E_NOT_OK for invalid input/state/ID or a retry outside retained data.
 * Preconditions: the current response is transmitting; otherwise BUFREQ_E_NOT_OK.
 * Context: TP callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; the TP orders copies for the same response.
 * Ownership: no pointer is retained; failed copies preserve data and size outputs.
 */
BufReq_ReturnType Dcm_CopyTxData(PduIdType id,
                                 const PduInfoType *info,
                                 const RetryInfoType *retry,
                                 PduLengthType *availableDataPtr);
/** Complete the outstanding TP transmission exactly once (SWS_Dcm_00351).
 * @param[in] id The reserved Dcm confirmation ID; other IDs are ignored.
 * @param[in] result E_OK commits an uncancelled session change; other values discard it.
 * @par Return
 * None.
 * Preconditions: outstanding TP request; otherwise no action.
 * Context: TP callback, including interrupt; not inside an exclusive area.
 * Reentrancy: reentrant; stale admission results cannot change a successor request.
 * Ownership: releases the response buffer; no pointer is retained.
 */
void Dcm_TpTxConfirmation(PduIdType id, Std_ReturnType result);
#endif
