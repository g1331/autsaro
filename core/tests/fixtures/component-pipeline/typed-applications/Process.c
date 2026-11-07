#include "Rte_Process.h"
#define Process_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Process_MemMap.h"
static uint32 slots[2] Process_VAR_CLEARED_UNSPECIFIED;
#define Process_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Process_MemMap.h"
#define Process_START_SEC_CODE
#include "Process_MemMap.h"
Process_CODE void Process_Periodic(void)
{
    uint32 value = 0U;
    if (Rte_Read_Value_Value(&value) == E_OK) {
        (void)Rte_Write_Result_Value(value);
    }
}
Process_CODE Std_ReturnType
Process_Transform(uint8 slot, uint8 bias, uint8 Input, uint32 *Output, boolean *State)
{
    if (Input == 250U) {
        return RTE_E_ScalarService_Busy;
    }
    slots[slot] += (uint32)Input + bias;
    *Output = slots[slot];
    *State = (*State == FALSE) ? TRUE : FALSE;
    return (Input == 251U) ? RTE_E_ScalarService_Denied : RTE_E_ScalarService_E_OK;
}
Process_CODE Std_ReturnType Process_Ping(uint8 slot, uint8 bias)
{
    slots[slot] += bias;
    return E_OK;
}
#define Process_STOP_SEC_CODE
#include "Process_MemMap.h"
