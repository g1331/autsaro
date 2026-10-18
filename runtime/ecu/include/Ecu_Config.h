/** @file Generated from the selected source contract. */
#ifndef ECU_CONFIG_H
#define ECU_CONFIG_H
#include "Std_Types.h"
/** Host transmission completion policy selected by the target. */
typedef enum {
    ECU_TX_SYNCHRONOUS = 0, /**< Complete in the lower transmit call. */
    ECU_TX_QUEUED = 1       /**< Complete through the queued host polling path. */
} EcuTxConfirmation;
/** Immutable generated target policy. */
typedef struct {
    EcuTxConfirmation tx_confirmation; /**< Completion path; fixed for this ECU lifetime. */
} EcuPolicyConfig;
extern const EcuPolicyConfig Ecu_Policy;
#endif
