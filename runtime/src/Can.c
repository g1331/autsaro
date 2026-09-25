#include "Can.h"
#include "CanIf.h"

static CanMode controller_mode;
static CanTxSink tx_sink;

void Can_Init(CanTxSink sink)
{
    tx_sink = sink;
    controller_mode = CAN_STARTED;
}

void Can_SetMode(CanMode mode)
{
    controller_mode = mode;
}

CanMode Can_GetMode(void)
{
    return controller_mode;
}

EcuStatus Can_Transmit(uint32_t id, uint8_t dlc, const uint8_t data[8])
{
    if (controller_mode != CAN_STARTED) {
        return ECU_ERR_CONTROLLER;
    }
    if (id > 0x7ffu) {
        return ECU_ERR_FRAME_ID;
    }
    if (dlc < 1u || dlc > 8u) {
        return ECU_ERR_FRAME_DLC;
    }
    return tx_sink(id, dlc, data);
}

EcuStatus Can_Inject(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms)
{
    if (id > 0x7ffu) {
        return ECU_ERR_FRAME_ID;
    }
    if (dlc < 1u || dlc > 8u) {
        return ECU_ERR_FRAME_DLC;
    }
    if (controller_mode != CAN_STARTED) {
        return ECU_ERR_CONTROLLER;
    }
    return CanIf_RxIndication(id, dlc, data, now_ms);
}
