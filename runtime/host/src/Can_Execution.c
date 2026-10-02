#include "Ecu_Execution.h"

EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc,
                                const uint8_t data[8]) {
    (void)handle;
    return sink(id, dlc, data);
}
