/** @file Selected unbuffered CanTp-to-Dcm transport routes. */
#ifndef PDUR_CANTP_H
#define PDUR_CANTP_H
#include "PduR_Version.h"
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "PduR_CanTp.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR_CanTp.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
/** Route reception admission directly to Dcm (SWS_PduR_00507, SWS_Dcm_00642).
 * @param[in] id A configured CanTp Rx N-SDU handle.
 * @param[in] info Borrowed first-frame descriptor, or NULL when no initial data is available;
 * no metadata. Initial bytes are not consumed by this admission call.
 * @param[in] length Positive total N-SDU length in bytes; Dcm rejects zero.
 * @param[out] available Caller-owned free receive-buffer capacity, written on acceptance.
 * @return The Dcm admission result; BUFREQ_E_NOT_OK without a route. Refusal preserves available.
 * Preconditions: Initialized and a configured route; otherwise BUFREQ_E_NOT_OK.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: No pointer is retained. Submit the initial bytes through CopyRxData after admission
 * (SWS_Dcm_00556); StartOfReception only reserves the reception state.
 */
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType length,
                                             PduLengthType *available);
/** Route the next receive segment to Dcm (SWS_PduR_00512).
 * @param[in] id A configured CanTp Rx N-SDU handle.
 * @param[in] info Borrowed source segment; zero length queries capacity and permits NULL
 * SduDataPtr.
 * @param[out] available Caller-owned remaining receive capacity in bytes, set by Dcm on success.
 * @return The Dcm copy result; BUFREQ_E_NOT_OK without a route.
 * Preconditions: Initialized and an admitted Dcm reception; the upper callback rejects other
 * states. Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: PduR retains no pointer; segment data is consumed synchronously.
 */
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available);
/** Forward the terminal Rx result to Dcm (SWS_PduR_00375).
 * @param[in] id A configured CanTp Rx N-SDU handle.
 * @param[in] result E_OK for completion, E_NOT_OK for failure.
 * @par Return
 * None.
 * Preconditions: Initialized and a configured route; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result);
/** Route a transmit-buffer request to Dcm (SWS_PduR_00518).
 * @param[in] id A configured CanTp Tx N-SDU handle.
 * @param[in] info Borrowed descriptor naming the writable destination and requested byte count;
 * zero length queries availability.
 * @param[in] retry NULL for the selected no-retry path; unsupported retry states are rejected by
 * Dcm.
 * @param[out] available Caller-owned count of bytes remaining after a successful copy.
 * @return The Dcm copy result; BUFREQ_E_NOT_OK without a route.
 * Preconditions: Initialized and an active Dcm transmission; the upper callback rejects other
 * states. Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: PduR retains no pointer. Per SWS the descriptor is an input even though its referenced
 * payload is filled on success.
 */
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available);
/** Forward the terminal Tx result to Dcm (SWS_PduR_00381).
 * @param[in] id A configured CanTp Tx N-SDU handle.
 * @param[in] result E_OK for completion, E_NOT_OK for failure.
 * @par Return
 * None.
 * Preconditions: Initialized and a configured route; otherwise no action.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result);
#endif
