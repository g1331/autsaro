/** @file Host-facing Can_Host entry points forwarding to the private driver operations. */
#include "Can_Internal.h"
#define CAN_START_SEC_CODE
#include "Can_MemMap.h"
CAN_CODE EcuStatus Can_HostFlush(void) { return Can_InternalHostFlush(); }

CAN_CODE uint64_t Can_HostTransmitToken(void) { return Can_InternalHostTransmitToken(); }

CAN_CODE Std_ReturnType Can_HostCompleteTransmit(PduIdType handle, uint64_t token)
{
    return Can_InternalHostCompleteTransmit(handle, token);
}

CAN_CODE EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t *data, uint64_t now_ms)
{
    uint8 short_frame[CAN_CLASSICAL_MAX_LENGTH] = {0};
    const uint8 *driver_data = data;
    /* The host caller supplies only the stated bytes. The driver takes at least eight. */
    if ((data != NULL_PTR) && (dlc < CAN_CLASSICAL_MAX_LENGTH)) {
        for (uint8 i = 0u; i < dlc; ++i) {
            short_frame[i] = data[i];
        }
        driver_data = short_frame;
    }
    return Can_InternalInject(id, dlc, driver_data, now_ms);
}
#define CAN_STOP_SEC_CODE
#include "Can_MemMap.h"
