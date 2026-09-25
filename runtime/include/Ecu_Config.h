#ifndef ECU_CONFIG_H
#define ECU_CONFIG_H

#include <stddef.h>
#include <stdint.h>
#include "Ecu_Status.h"

/* 仅支持标准 11 位 CAN、DLC 1..8、LSB0 小端无符号信号。 */
typedef struct {
    uint32_t id;
    uint8_t dlc;
    uint8_t direction; /* 0 Rx, 1 Tx */
    uint16_t first_signal;
    uint16_t signal_count;
    uint32_t period_ms;  /* Tx 周期；Rx 必须为 0 */
    uint32_t timeout_ms; /* Rx 接收超时；Tx 必须为 0 */
} EcuFrameConfig;

typedef struct {
    uint16_t id;
    uint8_t start_bit;
    uint8_t bit_length;
    uint32_t initial_value;
} EcuSignalConfig;

typedef struct {
    uint32_t code;
    uint16_t monitor_frame_index;
} EcuDtcConfig;

typedef EcuStatus (*EcuDidWriteFunction)(const uint8_t data[4]);

/* Single physical DoCAN connection; DIDs read live, configured Com signals. */
typedef struct {
    uint32_t request_can_id;
    uint32_t response_can_id;
    uint32_t s3_ms;
    uint32_t n_bs_ms;
    uint32_t n_cr_ms;
    uint16_t did;
    const uint16_t *did_signal_ids;
    uint8_t did_signal_count;
    const EcuDtcConfig *dtc;
    const EcuDidWriteFunction *did_writers; /* NULL disables WriteDataByIdentifier. */
} EcuDiagnosticConfig;

typedef struct {
    const char *name;
    const EcuFrameConfig *frames;
    size_t frame_count;
    const EcuSignalConfig *signals;
    size_t signal_count;
    const EcuDiagnosticConfig *diagnostic;
} EcuConfig;

/* 每个生成工程提供一个且仅一个此符号。 */
extern const EcuConfig Ecu_Config;

#define ECU_MAX_FRAMES 32u
#define ECU_MAX_SIGNALS 64u
#define ECU_DIAG_MAX_PAYLOAD 256u
#define ECU_DIAG_MAX_DID_SIGNALS 8u

#endif
