/** @file Public status-name values, including the documented unknown-code fallback. */
#include "Ecu_Status.h"
#include <stddef.h>
#include <stdio.h>

int main(void)
{
    const EcuStatus statuses[] = {ECU_OK,
                                  ECU_ERR_CONFIG,
                                  ECU_ERR_SIGNAL_ID,
                                  ECU_ERR_SIGNAL_VALUE,
                                  ECU_ERR_DIRECTION,
                                  ECU_ERR_FRAME_ID,
                                  ECU_ERR_FRAME_DLC,
                                  ECU_ERR_CONTROLLER,
                                  ECU_ERR_TIME,
                                  ECU_ERR_TP_SEQUENCE,
                                  ECU_ERR_TP_TIMEOUT,
                                  ECU_ERR_TP_LENGTH,
                                  ECU_ERR_TP_FLOW,
                                  ECU_ERR_TP_BUSY,
                                  ECU_ERR_IO,
                                  ECU_ERR_NVM,
                                  ECU_ERR_CAN_BUSY,
                                  (EcuStatus)99};
    for (size_t i = 0u; i < sizeof(statuses) / sizeof(statuses[0]); ++i) {
        if (puts(Ecu_StatusName(statuses[i])) < 0) {
            return 1;
        }
    }
    return 0;
}
