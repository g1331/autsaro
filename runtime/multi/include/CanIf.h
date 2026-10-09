/** @file Standard CanIf for one Classical CAN controller and immutable routes. */
#ifndef CANIF_H
#define CANIF_H
#include "ComStack_Types.h"
#include "Can_GeneralTypes.h"
#include "Ecu_Status.h"
typedef enum {
    CANIF_OFFLINE = 0,
    CANIF_TX_OFFLINE = 1,
    CANIF_TX_OFFLINE_ACTIVE = 2,
    CANIF_ONLINE = 3
} CanIf_PduModeType;
typedef struct {
    Can_IdType can_id;
    PduIdType pdu;
    uint8 minimum_length;
} CanIf_RxPduConfigType;
typedef struct {
    Can_IdType can_id;
    PduIdType pdu;
    uint8 maximum_length;
} CanIf_TxPduConfigType;
typedef void (*CanIf_ModeNotificationType)(uint8 controller, Can_ControllerStateType mode);
typedef void (*CanIf_BusOffNotificationType)(uint8 controller);
typedef struct {
    const CanIf_RxPduConfigType *receive;
    uint16 receive_count;
    const CanIf_TxPduConfigType *transmit;
    uint16 transmit_count;
    CanIf_ModeNotificationType controller_mode;
    CanIf_BusOffNotificationType bus_off;
} CanIf_ConfigType;
void CanIf_Init(const CanIf_ConfigType *ConfigPtr);
void CanIf_DeInit(void);
Std_ReturnType CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr);
Std_ReturnType CanIf_SetControllerMode(uint8 ControllerId, Can_ControllerStateType ControllerMode);
Std_ReturnType CanIf_GetControllerMode(uint8 ControllerId,
                                       Can_ControllerStateType *ControllerModePtr);
Std_ReturnType CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest);
Std_ReturnType CanIf_GetPduMode(uint8 ControllerId, CanIf_PduModeType *PduModePtr);
void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr);
void CanIf_TxConfirmation(PduIdType CanTxPduId);
void CanIf_ControllerModeIndication(uint8 ControllerId, Can_ControllerStateType ControllerMode);
void CanIf_ControllerBusOff(uint8 ControllerId);
/** Host adapter: timestamp is not a COM deadline clock. */
EcuStatus CanIf_HostRxIndication(uint32 id, uint8 dlc, const uint8 data[8], uint64 now_ms);
#endif
