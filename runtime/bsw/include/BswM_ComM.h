/** @file ComM mode indication entry of BswM for the single configured channel. */
#ifndef BSWM_COMM_H
#define BSWM_COMM_H
#include "BswM_Version.h"
#include "BswM.h"
/** Arbitrate the indicated ComM mode and run the matching rule's action (SWS_BswM_00047).
 * @param[in] Network Configured channel; other channels are ignored.
 * @param[in] RequestedMode NO, SILENT or FULL; other values are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action.
 * Context: ComM callback; not inside an exclusive area.
 * Reentrancy: reentrant. An indication received while an action runs is postponed and
 * arbitrated after it in the same caller's context (SWS_BswM_00069); each postponed
 * indication arbitrates the latest indicated mode (SWS_BswM_00281).
 * Ownership: None.
 */
void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType RequestedMode);
#endif
