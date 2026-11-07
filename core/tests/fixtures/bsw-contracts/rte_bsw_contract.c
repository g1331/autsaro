/** @file Explicitly mapped ordinary C/S consumer linked to the real Can producer.
 * Verifies: SWS_Rte_01132, SWS_Rte_06704
 */
#define RTE_CORE
#include "Rte_Observe.h"
#include "Rte_Ingress.h"
#include "Rte_Process.h"
#include "Rte_Main.h"
#include "Ecu_Target.h"
#include "SchM_Can.h"
#include "CanIf.h"
#include "Com.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <stdlib.h>

static unsigned entries;
static int owner = 1;
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};

int Ecu_TargetIsOwner(void) { return owner; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
    return E_OK;
}
void SchM_Enter_Can_CAN_STATE(void) { ++entries; }
void SchM_Exit_Can_CAN_STATE(void) {}
void CanIf_ControllerModeIndication(uint8 ControllerId, Can_ControllerStateType ControllerMode)
{
    (void)ControllerId;
    (void)ControllerMode;
}
void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr)
{
    (void)Mailbox;
    (void)PduInfoPtr;
}
void CanIf_TxConfirmation(PduIdType CanTxPduId) { (void)CanTxPduId; }
void CanIf_ControllerBusOff(uint8 ControllerId) { (void)ControllerId; }
Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId, uint8 InstanceId, uint8 ApiId, uint8 ErrorId)
{
    (void)ModuleId;
    (void)InstanceId;
    (void)ApiId;
    (void)ErrorId;
    return E_OK;
}
uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_OK;
}
uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_OK;
}
void Process_Transform(uint32 Input, uint32 *Output, uint32 *State)
{
    *Output = Input;
    *State = Input;
}
Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    (void)Data;
    return E_OK;
}

int main(void)
{
    Std_ReturnType (*const call)(void) = &Rte_Call_Observe_ResultService_Ping;
    if (call() != RTE_E_COM_STOPPED || entries != 0U || Rte_Start() != E_OK) {
        return EXIT_FAILURE;
    }
    if (call() != E_OK || entries != 1U) {
        return EXIT_FAILURE;
    }
    owner = 0;
    if (call() != E_NOT_OK || entries != 1U) {
        return EXIT_FAILURE;
    }
    owner = 1;
    Rte_Stop();
    return (call() == RTE_E_COM_STOPPED && entries == 1U) ? EXIT_SUCCESS : EXIT_FAILURE;
}
