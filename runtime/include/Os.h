#ifndef OS_H
#define OS_H

#include <stdint.h>
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/* 单核主机时钟：按配置周期调度 Com 发送，推进接收超时。 */
void Os_Init(const EcuConfig *config);
EcuStatus Os_Advance(uint64_t now_ms);
uint64_t Os_Now(void);

#endif
