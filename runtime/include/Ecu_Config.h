/** @file
 * @brief Host ECU configuration types used by generated output.
 */
#ifndef ECU_CONFIG_H
#define ECU_CONFIG_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Status.h"
#include "Ecu_DcmCallbackTypes.h"

/** @brief Configured 11-bit Classical CAN frame. */
typedef struct {
    uint32_t id;           /**< Standard 11-bit CAN identifier. */
    uint8_t dlc;           /**< Payload length from 1 to 8 bytes. */
    uint8_t direction;     /**< 0 for Rx, 1 for Tx. */
    uint16_t first_signal; /**< First entry in EcuConfig::signals. */
    uint16_t signal_count; /**< Number of signals owned by this frame. */
    uint32_t period_ms;    /**< Tx period; zero for Rx frames. */
    uint32_t timeout_ms;   /**< Rx timeout; zero for Tx frames. */
} EcuFrameConfig;

/** @brief Unsigned LSB0 signal within a configured frame. */
typedef struct {
    uint16_t id;            /**< Signal identifier used by the host RTE. */
    uint8_t start_bit;      /**< LSB0 bit position in the frame. */
    uint8_t bit_length;     /**< Signal width in bits. */
    uint32_t initial_value; /**< Value loaded at ECU initialization. */
} EcuSignalConfig;

/** @brief Single host diagnostic trouble code and its monitor. */
typedef struct {
    uint32_t code;                /**< Configured 24-bit DTC number. */
    uint16_t monitor_frame_index; /**< Index of the monitored Rx frame. */
} EcuDtcConfig;

/** @brief Read callback for one configured DID data element.
 * @param[out] data Destination for the configured 32-bit signal value.
 * @return Host callback status.
 */
typedef Std_ReturnType (*EcuDidReadFunction)(uint8_t *data);
/** @brief Write callback for one configured DID data element.
 * @param[in] data Source bytes for the configured 32-bit signal value.
 * @param[out] error_code UDS negative response code on callback failure.
 * @return Host callback status.
 */
typedef Std_ReturnType (*EcuDidWriteFunction)(const uint8_t *data,
                                              Dcm_NegativeResponseCodeType *error_code);
/** @brief Start callback for the host DID reset routine.
 * @return Host status of the reset operation.
 */
typedef EcuStatus (*EcuRoutineStartFunction)(void);

/** @brief Optional host routine that restores a DID's configured values. */
typedef struct {
    uint16_t id;                   /**< Configured routine identifier. */
    EcuRoutineStartFunction start; /**< Routine implementation. */
} EcuResetRoutineConfig;

/** @brief One physical DoCAN connection and its supported host services. */
typedef struct {
    uint32_t request_can_id;                /**< Physical request CAN identifier. */
    uint32_t response_can_id;               /**< Physical response CAN identifier. */
    uint32_t s3_ms;                         /**< Inactivity timeout for nondefault sessions. */
    uint32_t n_as_ms;                       /**< CAN frame transmit confirmation timeout. */
    uint32_t n_bs_ms;                       /**< Flow-control wait timeout. */
    uint32_t n_cr_ms;                       /**< Consecutive-frame receive timeout. */
    uint16_t did;                           /**< Configured data identifier. */
    const uint16_t *did_signal_ids;         /**< Ordered signal identifiers in the DID. */
    uint8_t did_signal_count;               /**< Number of DID signals and callbacks. */
    const EcuDtcConfig *dtc;                /**< Optional single DTC; NULL disables DTC services. */
    const EcuDidReadFunction *did_readers;  /**< One live read callback per DID signal. */
    const EcuDidWriteFunction *did_writers; /**< NULL disables WriteDataByIdentifier. */
    const EcuResetRoutineConfig *reset_routine; /**< NULL disables the reset routine. */
    uint8_t security_enabled;                   /**< Enables the host-only SecurityAccess level. */
    uint8_t tx_pdu_id;                          /**< Generated diagnostic CAN transmit handle. */
} EcuDiagnosticConfig;

/** @brief Root of the generated host ECU configuration. */
typedef struct {
    const char *name;                      /**< Generated ECU name. */
    const EcuFrameConfig *frames;          /**< Configured frame array. */
    size_t frame_count;                    /**< Number of frames. */
    const EcuSignalConfig *signals;        /**< Configured signal array. */
    size_t signal_count;                   /**< Number of signals. */
    const EcuDiagnosticConfig *diagnostic; /**< Optional diagnostic connection. */
} EcuConfig;

/** @brief Exactly one configuration instance is defined by each generated project. */
extern const EcuConfig Ecu_Config;

/** @brief Maximum frame count accepted by the host runtime. */
#define ECU_MAX_FRAMES 32u
/** @brief Maximum signal count accepted by the host runtime. */
#define ECU_MAX_SIGNALS 64u
/** @brief Maximum diagnostic payload size in bytes. */
#define ECU_DIAG_MAX_PAYLOAD 256u
/** @brief Maximum number of 32-bit signals in one DID. */
#define ECU_DIAG_MAX_DID_SIGNALS 8u

#endif
