/** @file EcuM wake-up indication for the configured CDD channel. */
#ifndef COMM_ECUM_H
#define COMM_ECUM_H
#include "ComM.h"
/** Turn a wake-up in COMM_NO_COM_NO_PENDING_REQUEST into a pending request that lasts
 * until the channel enters FULL (SWS_ComM_00275, SWS_ComM_00893).
 * @param[in] Channel Configured channel; other channels are ignored.
 * @par Return
 * None.
 * Preconditions: initialized; otherwise no action. Other states ignore it.
 * Context: EcuM callback; not inside an exclusive area.
 * Reentrancy: reentrant; a bus request it causes is issued by one caller at a time.
 * Ownership: None.
 */
void ComM_EcuM_WakeUpIndication(NetworkHandleType Channel);
#endif
