#ifndef ECU_STATUS_H
#define ECU_STATUS_H

typedef enum {
    ECU_OK = 0,
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
    ECU_ERR_IO
} EcuStatus;

const char *Ecu_StatusName(EcuStatus status);

#endif
