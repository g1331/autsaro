/** @file Generated from the selected source contract. */
#ifndef ECU_CONFIG_H
#define ECU_CONFIG_H
#include "Can_CfgTypes.h"
/** Host transmission completion policy selected by the target. */
typedef enum {
    ECU_TX_SYNCHRONOUS = 0, /**< Complete in the lower transmit call. */
    ECU_TX_QUEUED = 1       /**< Complete through the queued host polling path. */
} EcuTxConfirmation;
/** Driver admission for a host frame, on the configured controller and outside exclusive areas. */
typedef EcuStatus (*EcuHostIngressType)(uint8 controller, uint32 id, uint8 dlc, const uint8 *data);
/** Immutable generated target policy. */
typedef struct {
    EcuTxConfirmation tx_confirmation; /**< Completion path; fixed for this ECU lifetime. */
    const Can_ConfigType *can_config;  /**< Driver configuration shared with host admission. */
    EcuHostIngressType ingress;        /**< Selected host frame admission operation. */
} EcuPolicyConfig;
extern const EcuPolicyConfig Ecu_Policy;
#endif
