#ifndef ECU_CONFIG_H
#define ECU_CONFIG_H

#include <stddef.h>
#include <stdint.h>

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
    const char *name;
    const EcuFrameConfig *frames;
    size_t frame_count;
    const EcuSignalConfig *signals;
    size_t signal_count;
} EcuConfig;

/* 每个生成工程提供一个且仅一个此符号。 */
extern const EcuConfig Ecu_Config;

#define ECU_MAX_FRAMES 32u
#define ECU_MAX_SIGNALS 64u

#endif
