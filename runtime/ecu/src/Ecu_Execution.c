/** @file Forward ECU transmission to the controlled target. */
#include "Ecu_Execution.h"
#include "Ecu_Target.h"
EcuStatus Ecu_ExecutionTransmit(Can_TxSinkType sink,
                                uint8 controller,
                                PduIdType handle,
                                uint32_t id,
                                uint8_t dlc,
                                const uint8_t *data,
                                boolean brs,
                                uint32 data_phase_bitrate)
{
    (void)sink;
    return Ecu_TargetEnqueueTransmit(controller, handle, id, dlc, data, brs, data_phase_bitrate);
}
