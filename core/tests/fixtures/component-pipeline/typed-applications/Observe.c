#include "Rte_Observe.h"
#define Observe_START_SEC_CODE
#include "Observe_MemMap.h"
Observe_CODE void Observe_Periodic(void)
{
    uint32 input = 0U;
    uint32 output = 0U;
    boolean state = FALSE;
    if ((Rte_Read_Value_Value(&input) == E_OK) && (input <= 255U)) {
        if (Rte_Call_ResultService_Transform((uint8)input, &output, &state) == E_OK) {
            (void)Rte_Call_ResultService_Ping();
        }
        if (Rte_Call_OtherService_Transform((uint8)input, &output, &state) == E_OK) {
            (void)Rte_Call_OtherService_Ping();
        }
    }
}
#define Observe_STOP_SEC_CODE
#include "Observe_MemMap.h"
