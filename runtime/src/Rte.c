#include "Rte.h"
#include "Com.h"

EcuStatus Rte_WriteSignal(uint16_t id, uint32_t value)
{
    return Com_SetSignal(id, value);
}

EcuStatus Rte_ReadSignal(uint16_t id, uint32_t *value, uint8_t *valid)
{
    return Com_GetSignal(id, value, valid);
}
