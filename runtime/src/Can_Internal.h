/** @file Private driver operations; state remains in Can.c. */
#ifndef CAN_INTERNAL_H
#define CAN_INTERNAL_H
#include "Can_Host.h"
EcuStatus Can_InternalHostFlush(void);
uint64_t Can_InternalHostTransmitToken(void);
Std_ReturnType Can_InternalHostCompleteTransmit(PduIdType handle, uint64_t token);
void Can_InternalSetMode(CanMode mode);
CanMode Can_InternalGetMode(void);
EcuStatus Can_InternalTransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc,
                                  const uint8_t data[8]);
EcuStatus Can_InternalInject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms);
/* Private ingress for a frame already accepted by the platform, not host admission. */
EcuStatus Can_InternalRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8],
                                   uint64_t now_ms);
#endif
