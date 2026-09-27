#include "Ecu_Status.h"

const char *Ecu_StatusName(EcuStatus status) {
    switch (status) {
    case ECU_OK:
        return "OK";
    case ECU_ERR_CONFIG:
        return "CONFIG";
    case ECU_ERR_SIGNAL_ID:
        return "SIGNAL_ID";
    case ECU_ERR_SIGNAL_VALUE:
        return "SIGNAL_VALUE";
    case ECU_ERR_DIRECTION:
        return "DIRECTION";
    case ECU_ERR_FRAME_ID:
        return "FRAME_ID";
    case ECU_ERR_FRAME_DLC:
        return "FRAME_DLC";
    case ECU_ERR_CONTROLLER:
        return "CONTROLLER";
    case ECU_ERR_TIME:
        return "TIME";
    case ECU_ERR_TP_SEQUENCE:
        return "TP_SEQUENCE";
    case ECU_ERR_TP_TIMEOUT:
        return "TP_TIMEOUT";
    case ECU_ERR_TP_LENGTH:
        return "TP_LENGTH";
    case ECU_ERR_TP_FLOW:
        return "TP_FLOW";
    case ECU_ERR_TP_BUSY:
        return "TP_BUSY";
    case ECU_ERR_IO:
        return "IO";
    case ECU_ERR_NVM:
        return "NVM";
    default:
        return "INTERNAL";
    }
}
