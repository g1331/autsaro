/** @file Forward ECU transmission and clock access to the controlled target. */
#include "Ecu_Execution.h"
#include "Ecu_Target.h"
EcuStatus Ecu_ExecutionTransmit(
    Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8])
{
    (void)sink;
    return Ecu_TargetEnqueueTransmit(handle, id, dlc, data);
}
uint64_t Ecu_ExecutionNow(void) { return Ecu_TargetNow(); }
