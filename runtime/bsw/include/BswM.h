/** @file One IMMEDIATE ComM input with mode-dependent lower admission rule.
 * Each selected action runs in caller context; no deferred rules are selected.
 */
#ifndef BSWM_H
#define BSWM_H
#include "BswM_Version.h"
#include "ComM.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMM_AR_RELEASE_MAJOR_VERSION) || !defined(COMM_AR_RELEASE_MINOR_VERSION)
#error "BswM.h -> ComM.h: missing AUTOSAR release"
#elif (COMM_AR_RELEASE_MAJOR_VERSION != 4U) || (COMM_AR_RELEASE_MINOR_VERSION != 10U)
#error "BswM.h -> ComM.h: incompatible AUTOSAR release"
#endif
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
#define BSWM_MODULE_ID 42u
/** SWS_BswM_00214: mode requested by a BswM user. */
/* MR12 RULE 2.3 VIOLATION: standard type (SWS_BswM_00214) without the generic request that
 * uses it; see the BswM deviation record. */
typedef uint16 BswM_ModeType;
/** SWS_BswM_00216: identity of a BswM user. */
/* MR12 RULE 2.3 VIOLATION: standard type (SWS_BswM_00216) without the generic request that
 * uses it; see the BswM deviation record. */
typedef uint16 BswM_UserType;
/** Generated action list of the three ComM rules: one user callout per indicated mode.
 * Returns E_NOT_OK for another channel or mode, otherwise the callout's result.
 */
typedef Std_ReturnType (*BswM_ModeActionType)(NetworkHandleType Network, ComM_ModeType Mode);
/** Implementation-defined configuration (SWS_BswM_00213), emitted as immutable bswm_config.
 * The ComM input starts as BswMModeInitValue COMM_NO_COMMUNICATION and every rule
 * as BSWM_FALSE; no rule is evaluated before the first indication.
 */
typedef struct {
    NetworkHandleType channel;  /**< Indicated channel (BswMComMChannelRef). */
    BswM_ModeActionType action; /**< Rule action lists (BswMRuleTrueActionList). */
} BswM_ConfigType;
/** Start arbitrating ComM indications (SWS_BswM_00002).
 * @param[in] ConfigPtr Configuration with an action; otherwise BswM is unchanged.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: startup task; not inside an exclusive area.
 * Reentrancy: not reentrant. A restart drops a postponed input; an action already
 * running completes.
 * Ownership: ConfigPtr is retained until the next Init or Deinit.
 */
void BswM_Init(const BswM_ConfigType *ConfigPtr);
/** Stop arbitration; later indications have no effect (SWS_BswM_00119, SWS_BswM_00120).
 * @par Return
 * None.
 * Preconditions: None.
 * Context: shutdown task; not inside an exclusive area.
 * Reentrancy: not reentrant. An action already running completes; postponed input is
 * dropped.
 * Ownership: None.
 */
void BswM_Deinit(void);
#endif
