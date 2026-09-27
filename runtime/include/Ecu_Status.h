/** @file
 * @brief Status codes shared by the host runtime.
 */
#ifndef ECU_STATUS_H
#define ECU_STATUS_H

/** @brief Result codes used by the host virtual ECU runtime. */
typedef enum {
    ECU_OK = 0,           /**< Operation completed. */
    ECU_ERR_CONFIG,       /**< Generated configuration is invalid or unsupported. */
    ECU_ERR_SIGNAL_ID,    /**< Signal identifier is unknown. */
    ECU_ERR_SIGNAL_VALUE, /**< Signal value does not fit its configured width. */
    ECU_ERR_DIRECTION,    /**< Frame direction does not permit the operation. */
    ECU_ERR_FRAME_ID,     /**< CAN identifier is invalid or unknown. */
    ECU_ERR_FRAME_DLC,    /**< CAN payload length is invalid. */
    ECU_ERR_CONTROLLER,   /**< Virtual CAN controller cannot transmit or receive. */
    ECU_ERR_TIME,         /**< Host clock moved backward or exceeded a limit. */
    ECU_ERR_TP_SEQUENCE,  /**< Transport sequence number is invalid. */
    ECU_ERR_TP_TIMEOUT,   /**< Transport timing limit expired. */
    ECU_ERR_TP_LENGTH,    /**< Transport payload length is invalid. */
    ECU_ERR_TP_FLOW,      /**< Transport flow-control exchange failed. */
    ECU_ERR_TP_BUSY,      /**< Transport channel is already occupied. */
    ECU_ERR_IO,           /**< Host input or output operation failed. */
    ECU_ERR_NVM,          /**< Host DTC persistence operation failed. */
    ECU_ERR_CAN_BUSY      /**< Host CAN transmit object has no free slot. */
} EcuStatus;

/** @brief Return a stable text name for a host status code.
 * @param[in] status Status value to describe.
 * @return Static text owned by the runtime.
 */
const char *Ecu_StatusName(EcuStatus status);

#endif
