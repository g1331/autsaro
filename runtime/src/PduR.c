#include "PduR.h"
#include "CanIf.h"
#include "Com.h"

EcuStatus PduR_Transmit(size_t frame_index, const uint8_t data[8])
{
    return CanIf_Transmit(frame_index, data);
}

EcuStatus PduR_RxIndication(size_t frame_index, const uint8_t data[8], uint64_t now_ms)
{
    return Com_RxIndication(frame_index, data, now_ms);
}
