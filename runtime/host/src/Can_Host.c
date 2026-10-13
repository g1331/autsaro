#include "Can_Internal.h"
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
CAN_CODE EcuStatus Can_HostFlush(void) { return Can_InternalHostFlush(); }

CAN_CODE uint64_t Can_HostTransmitToken(void) { return Can_InternalHostTransmitToken(); }

CAN_CODE Std_ReturnType Can_HostCompleteTransmit(PduIdType handle, uint64_t token) {
    return Can_InternalHostCompleteTransmit(handle, token);
}

CAN_CODE void Can_SetMode(CanMode mode) { Can_InternalSetMode(mode); }

CAN_CODE CanMode Can_GetMode(void) { return Can_InternalGetMode(); }

CAN_CODE EcuStatus Can_TransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc,
                                   const uint8_t data[8]) {
    return Can_InternalTransmitPdu(pdu_id, id, dlc, data);
}

CAN_CODE EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    return Can_InternalInject(id, dlc, data, now_ms);
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
