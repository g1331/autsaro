/** @file DCMServices client linked to the real RTE and Dcm producers.
 * Verifies: SWS_Dcm_00698, SWS_Dcm_01030
 */
#define RTE_CORE
#include "Rte_Observe.h"
#include "Rte_Main.h"
#include "Ecu_Target.h"
#include "Ecu_Config.h"
#include "Dcm.h"
#include "Com.h"
#include <assert.h>
#include <signal.h>
#include <stdlib.h>

const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
static int owner = 1;
int Ecu_TargetIsOwner(void) { return owner; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
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

/* Reuse the independent public Dcm vectors through the connected SWC R-port.
 * Dcm.h has already declared the backend, so these aliases affect calls only. */
#define Dcm_GetSecurityLevel      Rte_Call_Observe_Diagnostics_GetSecurityLevel
#define Dcm_GetSesCtrlType        Rte_Call_Observe_Diagnostics_GetSesCtrlType
#define Dcm_ResetToDefaultSession Rte_Call_Observe_Diagnostics_ResetToDefaultSession
#define Dcm_SetActiveDiagnostic   Rte_Call_Observe_Diagnostics_SetActiveDiagnostic
#define main                      dcm_vectors
#include "dcm_contract.c"
#undef main

/* Expected mutation assertions keep a normal failure exit for the process owner. */
static void failed_assertion(int signum)
{
    (void)signum;
    _Exit(EXIT_FAILURE);
}

int main(void)
{
    Dcm_SecLevelType security = 99u;
    Dcm_SesCtrlType session_value = 99u;
    assert(signal(SIGABRT, &failed_assertion) != SIG_ERR);
    assert(Dcm_GetSecurityLevel(&security) == RTE_E_COM_STOPPED && security == 99u);
    assert(Dcm_GetSesCtrlType(&session_value) == RTE_E_COM_STOPPED && session_value == 99u);
    assert(Dcm_ResetToDefaultSession() == RTE_E_COM_STOPPED);
    assert(Dcm_SetActiveDiagnostic(FALSE) == RTE_E_COM_STOPPED);
    assert(Rte_Start() == E_OK);
    assert(dcm_vectors() == 0);
    owner = 0;
    security = 99u;
    session_value = 99u;
    assert(Dcm_GetSecurityLevel(&security) == E_NOT_OK && security == 99u);
    assert(Dcm_GetSesCtrlType(&session_value) == E_NOT_OK && session_value == 99u);
    assert(Dcm_ResetToDefaultSession() == E_NOT_OK);
    assert(Dcm_SetActiveDiagnostic(FALSE) == E_NOT_OK);
    owner = 1;
    assert(Rte_Stop() == E_OK);
    assert(Dcm_GetSecurityLevel(&security) == RTE_E_COM_STOPPED && security == 99u);
    return 0;
}
