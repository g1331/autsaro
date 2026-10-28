/** @file Single CDD channel ComM for NM variant NONE, without PNC, inhibition or NvM.
 * Configuration remains immutable for the initialized lifetime. All users map to
 * this channel; users belong to host BSW, without SW-C mode ports.
 */
#ifndef COMM_H
#define COMM_H
#include "ComM_Version.h"
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "ComM.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
#include "Rte_ComM_Type.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "ComM.h -> Rte_ComM_Type.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.h -> Rte_ComM_Type.h: incompatible AUTOSAR release"
#endif
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
/* MR12 RULE 2.5 VIOLATION: Required public declaration remains available even when unused here. */
#define COMM_MODULE_ID 12u
/** SWS_ComM_91027: Std_ReturnType extension of ComM. */
/* MR12 RULE 2.5 VIOLATION: standard return code (SWS_ComM_91027) without mode limitation in
 * the selected profile; see the ComM deviation record. */
#define COMM_E_MODE_LIMITATION 2u /**< Request refused by mode inhibition. */
/* MR12 RULE 2.5 VIOLATION: standard return code (SWS_ComM_91027) without PNCs in the
 * selected profile; see the ComM deviation record. */
#define COMM_E_MULTIPLE_PNC_ASSIGNED 3u /**< User belongs to more than one PNC. */
#define COMM_E_NO_PNC_ASSIGNED       4u /**< User belongs to no PNC. */
/* MR12 RULE 2.5 VIOLATION: standard return code (SWS_ComM_91027) without PNC learning in the
 * selected profile; see the ComM deviation record. */
#define COMM_E_LEARNING_ACTIVE          5u /**< PNC learning phase is active. */
#define COMM_NO_COM_NO_PENDING_REQUEST  0u /**< NO without a pending FULL request. */
#define COMM_NO_COM_REQUEST_PENDING     1u /**< NO, waiting for CommunicationAllowed. */
#define COMM_FULL_COM_NETWORK_REQUESTED 2u /**< FULL, requested or minimum duration running. */
#define COMM_FULL_COM_READY_SLEEP       3u /**< FULL without demand; NM NONE stays here. */
/* MR12 RULE 2.5 VIOLATION: standard state (SWS_ComM_00674) that NM NONE never enters; see
 * the ComM deviation record. */
#define COMM_SILENT_COM 4u /**< SILENT; unused by NM NONE. */
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
/** SWS_ComM_00668: initialization status reported by ComM_GetStatus. */
typedef enum {
    COMM_UNINIT = 0, /**< Not initialized, or deinitialized. */
    COMM_INIT = 1    /**< Initialized and usable. */
} ComM_InitStatusType;
/** SWS_ComM_00674: channel state and sub-state. NM NONE never enters COMM_SILENT_COM. */
typedef uint8 ComM_StateType;
/** <Bus>SM_RequestComMode of the configured CDD bus state manager (SWS_ComM_00828). */
typedef Std_ReturnType (*ComM_BusRequestType)(NetworkHandleType Channel, ComM_ModeType ComMode);
/** <Bus>SM_GetCurrentComMode of the configured CDD bus state manager (SWS_ComM_00828). */
typedef Std_ReturnType (*ComM_BusGetType)(NetworkHandleType Channel, ComM_ModeType *ComMode);
/** One Dcm_ComM_<Mode>ComModeEntered notification (SWS_ComM_00828). */
typedef void (*ComM_DcmModeType)(uint8 NetworkId);
/** Implementation-defined configuration (SWS_ComM_00162), emitted as immutable comm_config.
 * One channel with ComMNmVariant NONE and bus type CDD. The Dcm notifications are
 * all NULL when no Dcm connection references the channel, otherwise all set.
 */
typedef struct {
    NetworkHandleType channel;        /**< Channel handle (ComMChannelId). */
    const ComM_UserHandleType *users; /**< Distinct user handles (ComMUserIdentifier). */
    uint16 user_count;                /**< 1..32 users, all mapped to the channel. */
    uint32 minimum_full_ticks;        /**< ComMTMinFullComModeDuration, in main periods. */
    ComM_BusRequestType request;      /**< CDD bus mode request (ComMCDDBusPrefix). */
    ComM_BusGetType current;          /**< CDD actual bus mode query (ComMCDDBusPrefix). */
    ComM_DcmModeType dcm_no;          /**< Dcm_ComM_NoComModeEntered or NULL. */
    ComM_DcmModeType dcm_silent;      /**< Dcm_ComM_SilentComModeEntered or NULL. */
    ComM_DcmModeType dcm_full;        /**< Dcm_ComM_FullComModeEntered or NULL. */
} ComM_ConfigType;
/** Start or restart the channel in COMM_NO_COM_NO_PENDING_REQUEST (SWS_ComM_00146).
 * Clears requests, Allowed and diagnostic demand, then requests NO from the bus
 * state manager without notifying the default mode (SWS_ComM_00313).
 * @param[in] ConfigPtr Valid configuration; an invalid one leaves ComM unchanged.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant. A restart may overtake a call in flight; that call's
 * lower request or publication then completes with the new lifetime's state.
 * Ownership: ConfigPtr is retained until the next Init or a successful DeInit.
 */
COMM_CODE void ComM_Init(const ComM_ConfigType *ConfigPtr);
/** Deinitialize while the channel is in COMM_NO_COM_NO_PENDING_REQUEST and the bus
 * state manager reports actual NO (SWS_ComM_00794); otherwise no action.
 * @par Return
 * None.
 * Preconditions: None; before Init no action is taken.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant.
 * Ownership: None.
 */
COMM_CODE void ComM_DeInit(void);
/** Report COMM_INIT or COMM_UNINIT (SWS_ComM_00242).
 * @param[out] Status Written on E_OK.
 * @return E_OK, or E_NOT_OK for a NULL pointer.
 * Preconditions: None.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetStatus(ComM_InitStatusType *Status);
/** Report the channel inhibition status; both inhibitions are disabled, so it is 0
 * (SWS_ComM_00619).
 * @param[in] Channel Configured channel.
 * @param[out] Status Written on E_OK.
 * @return E_OK, or E_NOT_OK before Init, for another channel or a NULL pointer.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetInhibitionStatus(NetworkHandleType Channel,
                                                  ComM_InhibitionStatusType *Status);
/** Accept a new ECU group classification (SWS_ComM_00552). With wake-up inhibition and
 * mode limitation disabled, no channel behaviour depends on it.
 * @param[in] Status Any classification mask.
 * @return E_OK, or E_NOT_OK before Init.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
COMM_CODE Std_ReturnType ComM_SetECUGroupClassification(ComM_InhibitionStatusType Status);
/** Record a user's NO or FULL request and evaluate the channel (SWS_ComM_00110).
 * @param[in] User Configured user handle.
 * @param[in] ComMode COMM_NO_COMMUNICATION or COMM_FULL_COMMUNICATION.
 * @return E_OK when recorded; E_NOT_OK before Init, for an unknown user or another mode.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
COMM_CODE Std_ReturnType ComM_RequestComMode(ComM_UserHandleType User, ComM_ModeType ComMode);
/** Report the user's last recorded request (SWS_ComM_00079).
 * @param[in] User Configured user handle.
 * @param[out] ComMode Written on E_OK.
 * @return E_OK, or E_NOT_OK before Init, for an unknown user or a NULL pointer.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetRequestedComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
/** Report the actual mode from the bus state manager (SWS_ComM_00083, SWS_ComM_00084).
 * @param[in] User Configured user handle.
 * @param[out] ComMode Written by the bus state manager on E_OK.
 * @return The bus state manager's result; E_NOT_OK before Init, for an unknown user or
 * a NULL pointer.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetCurrentComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
/** PNC support is disabled, so no user has a PNC (SWS_ComM_91002).
 * @param[in] User Configured user handle.
 * @param[out] ComMode COMM_E_NO_PNC_ASSIGNED for a configured user, otherwise not written.
 * @return COMM_E_NO_PNC_ASSIGNED for a configured user; E_NOT_OK before Init, for an
 * unknown user or a NULL pointer.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetCurrentPNCComMode(ComM_UserHandleType User,
                                                   ComM_ModeType *ComMode);
/** Report FULL, as no inhibition is selected (SWS_ComM_00085).
 * @param[in] User Configured user handle.
 * @param[out] ComMode Written on E_OK.
 * @return E_OK, or E_NOT_OK before Init, for an unknown user or a NULL pointer.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: no pointer is retained.
 */
COMM_CODE Std_ReturnType ComM_GetMaxComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
/** Propagate the actual bus mode to BswM and Dcm (SWS_ComM_00675). The channel state
 * does not follow it: NM NONE has no bus-driven transition (SWS_ComM Figure 7.7).
 * @param[in] Channel Configured channel.
 * @param[in] ComMode NO, SILENT or FULL; other values are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: bus state manager callback; not inside an exclusive area.
 * Reentrancy: reentrant. One caller at a time publishes; a reentrant or concurrent
 * indication is published after it, and only the latest of several is published.
 * Ownership: None.
 */
COMM_CODE void ComM_BusSM_ModeIndication(NetworkHandleType Channel, ComM_ModeType ComMode);
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM.h -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.h -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
