/** @file Explicit CDD provider for one controlled-host CAN channel.
 * Full completion requires actual STARTED indication and ONLINE PDU mode.
 * Bus-off cannot be recovered by an ordinary ComM request. The normal host
 * CAN mode command remains the explicit recovery boundary.
 */
#ifndef ECU_HOSTBUSSM_H
#define ECU_HOSTBUSSM_H
#include "ComM.h"
#include "CanIf.h"
typedef struct {
    NetworkHandleType channel;
    uint8 controller;
} Ecu_HostBusSM_ConfigType;
void Ecu_HostBusSM_Init(const Ecu_HostBusSM_ConfigType *ConfigPtr);
void Ecu_HostBusSM_DeInit(void);
Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel, ComM_ModeType ComMode);
Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel, ComM_ModeType *ComMode);
/** Actual BswM user-callout action; controls directions, never COM groups. */
Std_ReturnType Ecu_HostBusSM_ApplyMode(NetworkHandleType Channel, ComM_ModeType ComMode);
void Ecu_HostBusSM_ControllerModeIndication(uint8 ControllerId,
                                            Can_ControllerStateType ControllerMode);
void Ecu_HostBusSM_ControllerBusOff(uint8 ControllerId);
#endif
