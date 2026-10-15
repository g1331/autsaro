/** @file One physical, full-duplex, padded Classical CAN connection channel.
 * The receiver asks for the whole N-SDU with BS 0 and STmin 0; while the upper buffer is
 * short it waits up to N_Br and then sends at most wait_frame_max FC(WAIT).
 * Timer values are counts of the configured main-function period, never host time.
 */
#ifndef CANTP_H
#define CANTP_H
#include "Std_Types.h"
#include "ComStack_Types.h"
/** Implementation-defined configuration of the one channel (SWS_CanTp_00340), generated
 * as the read-only tp_config. Timer members count CanTp_MainFunction calls, not
 * milliseconds; each must be at least 2, since a timer may expire at the call right after
 * it starts, or CanTp_Init ignores the configuration.
 */
typedef struct {
    /** Rx N-SDU ID accepted by CanTp_CancelReceive (CanTpRxNSduId, ECUC_CanTp_00301). */
    PduIdType receive;
    /** Tx N-SDU ID accepted by CanTp_Transmit (CanTpTxNSduId, ECUC_CanTp_00268). */
    PduIdType transmit;
    /** Rx N-PDU ID of data and FC frames (CanTpRxNPduId, ECUC_CanTp_00258). */
    PduIdType lower_receive;
    /** Tx N-PDU ID passed to LSduR and confirmed (CanTpTxNPduConfirmationPduId,
     * ECUC_CanTp_00286). */
    PduIdType lower_transmit;
    /** Largest N-SDU in bytes, 8 to 256; the Dcm buffer size (DcmDslBufferSize). */
    uint16 maximum_length;
    /** N_As in main-function periods (CanTpNas, ECUC_CanTp_00263). */
    uint32 n_as;
    /** N_Ar in main-function periods (CanTpNar, ECUC_CanTp_00277). */
    uint32 n_ar;
    /** N_Br in main-function periods (CanTpNbr, ECUC_CanTp_00245). */
    uint32 n_br;
    /** N_Bs in main-function periods (CanTpNbs, ECUC_CanTp_00264). */
    uint32 n_bs;
    /** N_Cr in main-function periods (CanTpNcr, ECUC_CanTp_00279). */
    uint32 n_cr;
    /** N_Cs in main-function periods (CanTpNcs, ECUC_CanTp_00265). */
    uint32 n_cs;
    /** Main-function period in ms, used to convert a received STmin into periods
     * (CanTpMainFunctionPeriod, ECUC_CanTp_00240). */
    uint16 main_period_ms;
    /** FC(WAIT) frames sent after N_Br expiries before the reception is aborted; 0 aborts
     * at the first expiry (CanTpRxWftMax, ECUC_CanTp_00251). */
    uint8 wait_frame_max;
    /** Byte that fills unused frame bytes (CanTpPaddingByte, ECUC_CanTp_00298). */
    uint8 padding;
    /** PduR callback ID of the Rx N-SDU, from its canonical Pdu reference; a namespace
     * separate from receive. */
    PduIdType upper_receive;
    /** PduR callback ID of the Tx N-SDU, from its canonical Pdu reference; a namespace
     * separate from transmit. */
    PduIdType upper_transmit;
} CanTp_ConfigType;
/** Start the module with a generated configuration and drop all connections without
 * notification (SWS_CanTp_00208, 00273).
 * @param[in] CfgPtr Generated configuration with 8 to 256 bytes per N-SDU, timers of at
 * least 2 periods and a nonzero period; any other pointer, including NULL, is ignored and
 * the state is unchanged.
 * @return None.
 * Preconditions: None; a repeated call restarts the module.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: the configuration is kept until CanTp_Shutdown or the next CanTp_Init.
 */
void CanTp_Init(const CanTp_ConfigType *CfgPtr);
/** Stop the module and drop all connections without notification (SWS_CanTp_00200).
 * @return None.
 * Preconditions: None; later calls behave as before CanTp_Init.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: the configuration is released.
 */
void CanTp_Shutdown(void);
/** Request transmission of an N-SDU whose data is copied later via PduR (SWS_CanTp_00212).
 * @param[in] TxPduId The configured Tx N-SDU.
 * @param[in] PduInfoPtr Caller-owned; only SduLength (1 to the configured maximum) is read and
 * MetaDataPtr must be NULL.
 * @return E_OK when accepted; the final result is reported by PduR_CanTpTxConfirmation.
 * E_NOT_OK for an unknown ID, invalid length or while this N-SDU is still transmitting.
 * Preconditions: CanTp initialized; otherwise E_NOT_OK.
 * Context: task or upper-layer callback, including the final confirmation of the previous
 * N-SDU; not inside an exclusive area.
 * Reentrancy: reentrant; of concurrent requests for the N-SDU at most one is accepted.
 * Ownership: PduInfoPtr is not kept; the first frame may be sent before return.
 */
Std_ReturnType CanTp_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
/** CanTpTc is false, so no transmission can be cancelled (SWS_CanTp_00242, 00246).
 * @param[in] TxPduId Any ID; the configured Tx N-SDU also reports CANTP_E_OPER_NOT_SUPPORTED
 * as a runtime error.
 * @return E_NOT_OK.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
Std_ReturnType CanTp_CancelTransmit(PduIdType TxPduId);
/** Abort the reception of an N-SDU and notify PduR with E_NOT_OK (SWS_CanTp_00257, 00263).
 * @param[in] RxPduId The configured Rx N-SDU.
 * @return E_OK when the reception was aborted. E_NOT_OK for an unknown ID, while an SF or
 * FF is being admitted, or while the last CF is awaited (SWS_CanTp_00262); E_NOT_OK with
 * CANTP_E_OPER_NOT_SUPPORTED when no cancellable reception exists (none, refused with
 * FC(OVFLW), or ended with its notification still owed).
 * Preconditions: CanTp initialized; otherwise E_NOT_OK.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant; at most one concurrent call aborts a given reception.
 * Ownership: None; an FC still in flight keeps the Tx N-PDU until its confirmation.
 */
Std_ReturnType CanTp_CancelReceive(PduIdType RxPduId);
/** Indicate reception of an N-PDU (SWS_CanTp_00214).
 * @param[in] RxPduId The configured Rx N-PDU; other IDs are ignored.
 * @param[in] PduInfoPtr Lower-layer-owned frame of 1 to 8 bytes without metadata; others
 * are ignored.
 * @return None.
 * Preconditions: CanTp initialized; otherwise the frame is ignored.
 * Context: lower-layer callback, also in interrupt context (SWS_CanTp_00235); not inside
 * an exclusive area.
 * Reentrancy: reentrant; frames are processed in arrival order, and a frame beyond four
 * still unprocessed is dropped with CANTP_E_COM.
 * Ownership: the frame is copied before return and not kept.
 */
void CanTp_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
/** Confirm transmission of the N-PDU last passed to LSduR_CanTpTransmit (SWS_CanTp_00215).
 * @param[in] TxPduId The configured Tx N-PDU; other IDs are ignored.
 * @param[in] result E_OK when sent; otherwise the connection that sent it is aborted.
 * @return None.
 * Preconditions: None; a confirmation without a frame in flight is ignored.
 * Context: lower-layer callback, also in interrupt context (SWS_CanTp_00236); not inside
 * an exclusive area. The next frame on the N-PDU waits for CanTp_MainFunction.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void CanTp_TxConfirmation(PduIdType TxPduId, Std_ReturnType result);
#endif
