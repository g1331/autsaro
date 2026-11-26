/** @file Independent driver tests select the runtime's standalone defaults. */
#ifndef ECU_PROFILE_CONFIG_H
#define ECU_PROFILE_CONFIG_H
#define COMM_USER_CAPACITY    6u
#define COMM_CHANNEL_CAPACITY 6u
#ifndef CAN_HOH_CAPACITY
#define CAN_HOH_CAPACITY 12u
#endif
#ifndef CAN_CONTROLLER_CAPACITY
#define CAN_CONTROLLER_CAPACITY 6u
#endif
#ifndef CANIF_CONTROLLER_CAPACITY
#define CANIF_CONTROLLER_CAPACITY 6u
#endif
#ifndef CANTP_CONNECTION_CAPACITY
#define CANTP_CONNECTION_CAPACITY 1u
#endif
#define CANIF_RX_CAPACITY 32u
#define CANIF_TX_CAPACITY 32u
#endif
