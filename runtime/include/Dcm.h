/** @file
 * @brief Host diagnostic request processing interface.
 */
#ifndef DCM_H
#define DCM_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/** @brief Load the generated host diagnostic connection.
 * @param[in] config Generated diagnostic configuration.
 */
void Dcm_Init(const EcuDiagnosticConfig *config);
/** @brief Process one complete physical diagnostic request.
 * @param[in] request Request payload owned by the caller.
 * @param[in] length Request length in bytes.
 * @param[in] now_ms Current host clock in milliseconds.
 * @return Host request-processing status.
 */
EcuStatus Dcm_RxIndication(const uint8_t *request, size_t length, uint64_t now_ms);
/** @brief Notify Dcm that a transport response completed or failed.
 * @param[in] status Transport completion status.
 * @param[in] now_ms Current host clock in milliseconds.
 */
void Dcm_TpTxConfirmation(EcuStatus status, uint64_t now_ms);
/** @brief Advance diagnostic session timing.
 * @param[in] now_ms Current host clock in milliseconds.
 */
void Dcm_AdvanceTime(uint64_t now_ms);

#ifdef ECU_TARGET_EPIC4
/** @brief Process copied requests at the owner diagnostic/confirmation phase.
 * @param[in] now_ms Explicit owner epoch; does not advance timers.
 * @return Actual dispatch status; queued requests wait for prior transport.
 */
EcuStatus Dcm_TargetProcess(uint64_t now_ms);
/** @brief Inspect the owner's deferred request count.
 * @return Number of complete requests awaiting dispatch.
 */
unsigned Dcm_TargetPending(void);
#endif

#endif
