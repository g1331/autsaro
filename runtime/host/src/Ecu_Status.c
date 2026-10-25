/** @file Printable names for host EcuStatus values. */
#include "Ecu_Status.h"

const char *Ecu_StatusName(EcuStatus status)
{
    const char *name = "INTERNAL";
    switch (status) {
        case ECU_OK:
            name = "OK";
            break;
        case ECU_ERR_CONFIG:
            name = "CONFIG";
            break;
        case ECU_ERR_SIGNAL_ID:
            name = "SIGNAL_ID";
            break;
        case ECU_ERR_SIGNAL_VALUE:
            name = "SIGNAL_VALUE";
            break;
        case ECU_ERR_DIRECTION:
            name = "DIRECTION";
            break;
        case ECU_ERR_FRAME_ID:
            name = "FRAME_ID";
            break;
        case ECU_ERR_FRAME_DLC:
            name = "FRAME_DLC";
            break;
        case ECU_ERR_CONTROLLER:
            name = "CONTROLLER";
            break;
        case ECU_ERR_TIME:
            name = "TIME";
            break;
        case ECU_ERR_TP_SEQUENCE:
            name = "TP_SEQUENCE";
            break;
        case ECU_ERR_TP_TIMEOUT:
            name = "TP_TIMEOUT";
            break;
        case ECU_ERR_TP_LENGTH:
            name = "TP_LENGTH";
            break;
        case ECU_ERR_TP_FLOW:
            name = "TP_FLOW";
            break;
        case ECU_ERR_TP_BUSY:
            name = "TP_BUSY";
            break;
        case ECU_ERR_IO:
            name = "IO";
            break;
        case ECU_ERR_NVM:
            name = "NVM";
            break;
        case ECU_ERR_CAN_BUSY:
            name = "CAN_BUSY";
            break;
        default:
            break;
    }
    return name;
}
