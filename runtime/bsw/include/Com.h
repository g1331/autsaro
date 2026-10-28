/** @file Selected R24-11 immediate UINT32 COM interface and immutable configuration. */
#ifndef COM_H
#define COM_H
#include "Com_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Com.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "Com.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COM_MODULE_ID             50u
#define COM_SERVICE_NOT_AVAILABLE 0x80u /**< Service unavailable (SWS_Com_00865). */
/* MR12 RULE 2.5 VIOLATION: standard return code (SWS_Com_00865) that is never returned,
 * because the selected profile has no large or TP I-PDUs; see the Com deviation record. */
#define COM_BUSY 0x81u /**< Large/TP buffer busy (SWS_Com_00865); not returned here. */

#define COM_START_SEC_CODE
#include "Com_MemMap.h"
/** COM initialization state (SWS_Com_00819). */
typedef enum {
    COM_UNINIT = 0, /**< No valid configuration; reset state. */
    COM_INIT = 1    /**< Configuration is valid and services are usable. */
} Com_StatusType;
/** Zero-based generated signal handle (SWS_Com_00820). */
typedef uint16 Com_SignalIdType;
/** Zero-based I-PDU group handle (SWS_Com_00822); one Rx group is selected. */
typedef uint16 Com_IpduGroupIdType;
/** Generated signal/PDU binding. Counts are Rx-main-function periods and PDU data is four-byte
 * little endian; derived fields are not independent ECUC parameters. One four-byte little-endian
 * signal per PDU, no update bits or metadata.
 *  * Timer counts are configured Rx main-function periods, not host timestamps.
 */
typedef struct {
    PduIdType pdu;              /**< Generated ComIPduHandleId from the referenced route. */
    Com_SignalIdType signal_id; /**< Generated zero-based Com signal handle. */
    boolean receive;            /**< TRUE for ComIPduDirection RECEIVE, FALSE for SEND. */
    uint32 initial_value;       /**< Native UINT32 ComSignalInitValue; selected value is zero. */
    uint32 timeout_ticks;       /**< ComTimeout in Rx-main periods; 0 disables the deadline. */
    CbkHandleIdType callback_handle; /**< ComUserCbkHandleId for Rx; unused for Tx. */
} Com_PduConfigType;
/** Generated RTE receive/timeout callback; handle is ComUserCbkHandleId, called outside COM_STATE.
 */
typedef void (*Com_NotificationType)(CbkHandleIdType handle);
/** Generated PduR transmit binding; borrows the four-byte COM snapshot only until return. */
typedef Std_ReturnType (*Com_TransmitType)(PduIdType id, const PduInfoType *info);
/** Implementation configuration (SWS_Com_00825); counts and callback bindings are derived from the
 * validated COM/PduR plan, not independent ECUC parameters. */
typedef struct {
    const Com_PduConfigType *pdus; /**< Immutable PDU table; must be non-NULL. */
    uint16 pdu_count;              /**< PDU count, 1..32. */
    Com_IpduGroupIdType
        receive_group; /**< The configured ComIPduGroup handle; contains Rx PDUs only. */
    Com_NotificationType
        receive_notification; /**< Required when any Rx PDU exists; called outside COM_STATE. */
    Com_NotificationType
        timeout_notification;  /**< Required when any Rx PDU exists; called outside COM_STATE. */
    Com_TransmitType transmit; /**< Required when any Tx PDU exists; called outside COM_STATE. */
} Com_ConfigType;

/** Reset COM state and install the selected UINT32 configuration (SWS_Com_00432).
 * @param[in] config Immutable configuration, 1 to 32 PDUs and one Rx group; invalid or NULL leaves
 * COM_UNINIT.
 * @par Return
 * None.
 * Preconditions: None; reinitialization replaces the previous configuration and restarts reception
 * state. Context: startup task; not inside an exclusive area. Reentrancy: not reentrant. Ownership:
 * The configuration, PDU table and callbacks remain valid until DeInit or the next Init; signal
 * data is copied into COM storage.
 */
COM_CODE void Com_Init(const Com_ConfigType *config);
/** Reset signal and monitoring state and release the configuration (SWS_Com_00130).
 * @par Return
 * None.
 * Preconditions: None; repeated deinitialization is harmless.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: The configuration pointer is released; a notification already in flight completes
 * outside COM_STATE.
 */
COM_CODE void Com_DeInit(void);
/** Read whether COM has a valid configuration (SWS_Com_00194).
 * @return COM_INIT when configured, COM_UNINIT otherwise.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
COM_CODE Com_StatusType Com_GetStatus(void);
/** Copy one UINT32 Tx signal into COM storage (SWS_Com_00197).
 * @param[in] SignalId A configured Tx signal handle.
 * @param[in] SignalDataPtr Non-NULL caller-owned storage containing four bytes of native UINT32
 * signal data.
 * @return E_OK on update; COM_SERVICE_NOT_AVAILABLE when uninitialized; E_NOT_OK for an unknown/Rx
 * signal or NULL data. Preconditions: Initialized; otherwise COM_SERVICE_NOT_AVAILABLE.
 * Transmission occurs separately in the Tx main function. Context: task or upper-layer callback;
 * not inside an exclusive area. Reentrancy: reentrant. Ownership: Four bytes are copied before
 * return; the pointer is never retained.
 */
COM_CODE uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr);
/** Copy the current UINT32 Rx signal to the caller (SWS_Com_00198).
 * @param[in] SignalId A configured Rx signal handle.
 * @param[out] SignalDataPtr Non-NULL caller-owned four-byte output; receives current/initial data
 * even when the Rx group is stopped.
 * @return E_OK for a started Rx group; COM_SERVICE_NOT_AVAILABLE when stopped or uninitialized;
 * E_NOT_OK for invalid arguments. Preconditions: A valid configuration is required to write output;
 * otherwise output is preserved. Context: task or upper-layer callback; not inside an exclusive
 * area. Reentrancy: reentrant. Ownership: The output is written before return when the configured
 * Rx signal exists, including stopped-group status; it is not retained.
 */
COM_CODE uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr);
/** No invalid values are configured, so no signal can be invalidated (SWS_Com_00203/00643).
 * @param[in] SignalId Any signal handle; none has a configured invalid value.
 * @return COM_SERVICE_NOT_AVAILABLE without changing signal data.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
COM_CODE uint8 Com_InvalidateSignal(Com_SignalIdType SignalId);
/** Start the single Rx I-PDU group (SWS_Com_91001). Starting an already started group has no
 * effect; first timeout is zero.
 * @param[in] IpduGroupId The configured Rx group ID; unknown groups are ignored.
 * @param[in] Initialize TRUE reloads initial signal values on the stopped-to-started transition;
 * FALSE preserves current values.
 * @par Return
 * None.
 * Preconditions: Initialized and the configured group; otherwise no action.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different I-PDU groups; not for the same group.
 * Ownership: None.
 */
COM_CODE void Com_IpduGroupStart(Com_IpduGroupIdType IpduGroupId, boolean Initialize);
/** Stop the single Rx I-PDU group (SWS_Com_91002). Retain the current signal values and reset
 * receive deadlines.
 * @param[in] IpduGroupId The configured Rx group ID; unknown groups are ignored.
 * @par Return
 * None.
 * Preconditions: Initialized and the configured group; otherwise no action.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different I-PDU groups; not for the same group.
 * Ownership: None.
 */
COM_CODE void Com_IpduGroupStop(Com_IpduGroupIdType IpduGroupId);
/** Enable reception deadline monitoring (SWS_Com_91004). No first timeout runs until a reception;
 * enabling an enabled monitor has no effect.
 * @param[in] IpduGroupId The configured Rx group ID; unknown groups are ignored.
 * @par Return
 * None.
 * Preconditions: Initialized and the configured group; otherwise no action.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different I-PDU groups; not for the same group.
 * Ownership: None.
 */
COM_CODE void Com_EnableReceptionDM(Com_IpduGroupIdType IpduGroupId);
/** Disable reception deadline monitoring (SWS_Com_91003). The group start/stop state and current
 * signal values are unchanged.
 * @param[in] IpduGroupId The configured Rx group ID; unknown groups are ignored.
 * @par Return
 * None.
 * Preconditions: Initialized and the configured group; otherwise no action.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different I-PDU groups; not for the same group.
 * Ownership: None.
 */
COM_CODE void Com_DisableReceptionDM(Com_IpduGroupIdType IpduGroupId);
/** Update reception timing and unpack a complete four-byte signal (SWS_Com_00123).
 * @param[in] RxPduId A configured Rx I-PDU handle.
 * @param[in] PduInfoPtr Borrowed descriptor and bytes; a short PDU resets deadline monitoring but
 * does not overwrite signal data.
 * @par Return
 * None.
 * Preconditions: Initialized and the Rx group started; invalid inputs and unknown PDUs are ignored.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: Data is copied before return and not retained; reception notifications run outside
 * COM_STATE in recorded order.
 */
COM_CODE void Com_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr);
/** Immediately send current data, without storing a deferred trigger (SWS_Com_00348/00840).
 * @param[in] PduId A configured Tx I-PDU handle; selected Tx PDUs have no group and are started by
 * Init.
 * @return E_OK for an initialized Tx PDU; E_NOT_OK otherwise. The lower refusal is not reported
 * here because retry is disabled. Preconditions: Initialized; otherwise E_NOT_OK. Context: task or
 * upper-layer callback; not inside an exclusive area. Reentrancy: not reentrant. Ownership: The
 * four-byte snapshot is borrowed by the lower transmit callback only until it returns, outside
 * COM_STATE.
 */
COM_CODE Std_ReturnType Com_TriggerIPDUSend(PduIdType PduId);
/** Select PERIODIC for TRUE or the absent FALSE mode NONE (SWS_Com_00784).
 * @param[in] PduId A configured Tx I-PDU handle.
 * @param[in] Mode TRUE enables periodic transmission on the next Tx main call; FALSE disables it,
 * but explicit triggering still sends.
 * @par Return
 * None.
 * Preconditions: Initialized and a Tx PDU; otherwise no action.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: None.
 */
COM_CODE void Com_SwitchIpduTxMode(PduIdType PduId, boolean Mode);
/** Copy the current four-byte Tx PDU into a lower buffer (SWS_Com_00001).
 * @param[in] TxPduId A configured Tx I-PDU handle.
 * @param[in,out] PduInfoPtr Non-NULL caller-owned descriptor and destination; input capacity must
 * be at least four bytes. Success writes four bytes and SduLength=4.
 * @return E_OK after copying; E_NOT_OK preserves the descriptor and payload.
 * Preconditions: Initialized, a configured Tx PDU and adequate non-NULL output; otherwise E_NOT_OK.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: No pointer is retained; all successful output writes occur before return.
 */
COM_CODE Std_ReturnType Com_TriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr);
/** Consume the result without Tx ACK/NACK callbacks or Tx deadline monitoring (SWS_Com_00124).
 * @param[in] TxPduId Any Tx I-PDU handle; selected confirmation processing has no state.
 * @param[in] result E_OK or E_NOT_OK; neither suppresses the next periodic request.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: lower-layer callback, including interrupt context; not inside an exclusive area.
 * Reentrancy: reentrant for different PDU IDs; not reentrant for the same PDU ID.
 * Ownership: None.
 */
COM_CODE void Com_TxConfirmation(PduIdType TxPduId, Std_ReturnType result);
#define COM_STOP_SEC_CODE
#include "Com_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Com.h -> Com_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com.h -> Com_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
