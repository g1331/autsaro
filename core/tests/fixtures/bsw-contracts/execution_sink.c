/* Test execution adapter: frames go straight to the driver's configured sink. */
#include "Ecu_Execution.h"

EcuStatus Ecu_ExecutionTransmit(Can_TxSinkType sink,
                                uint8 controller,
                                PduIdType handle,
                                uint32_t id,
                                uint8_t dlc,
                                const uint8_t *data,
                                boolean brs,
                                uint32 data_phase_bitrate)
{
    (void)controller;
    (void)handle;
    (void)brs;
    (void)data_phase_bitrate;
    return sink(id, dlc, data);
}
