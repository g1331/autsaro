/** @file
 * @brief Host PDU routing and transport buffer interface.
 */
#ifndef PDUR_H
#define PDUR_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8]);
EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms);
void PduR_Init(const EcuConfig *config);

/* The PduR owns the N-SDU buffers; CanTp only copies individual segments. */
EcuStatus PduR_CanTpStartOfReception(size_t length);
EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length);
EcuStatus PduR_CanTpRxIndication(uint64_t now_ms);
void PduR_CanTpRxAbort(void);
EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length);
void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms);
EcuStatus PduR_DcmTransmit(const uint8_t *data, size_t length, uint64_t now_ms);

#endif
