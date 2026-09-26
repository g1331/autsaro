#ifndef ECU_RUNTIME_H
#define ECU_RUNTIME_H

#include "Can.h"
#include "Ecu_Config.h"
#include "Ecu_Status.h"

/* 校验整个生成配置后初始化各模块；失败时不得驱动 ECU。 */
EcuStatus Ecu_Init(const EcuConfig *config, CanTxSink sink, const char *nvm_path,
                   const char *security_key_path, const char *security_state_path);

#endif
