/* Test execution adapter: frames go straight to the driver's configured sink. */
#include "Ecu_Execution.h"

EcuStatus Ecu_ExecutionTransmit(
    Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8])
{
    (void)handle;
    return sink(id, dlc, data);
}
