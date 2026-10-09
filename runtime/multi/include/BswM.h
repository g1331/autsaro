/** @file One IMMEDIATE ComM input with mode-dependent lower admission rule.
 * Each selected action runs in caller context; no deferred rules are selected.
 */
#ifndef BSWM_H
#define BSWM_H
#include "ComM.h"
typedef uint16 BswM_ModeType;
typedef uint16 BswM_UserType;
typedef Std_ReturnType (*BswM_ModeActionType)(NetworkHandleType Network, ComM_ModeType Mode);
typedef struct {
    NetworkHandleType channel;
    ComM_ModeType initial_mode;
    BswM_ModeActionType action;
} BswM_ConfigType;
void BswM_Init(const BswM_ConfigType *ConfigPtr);
void BswM_Deinit(void);
#endif
