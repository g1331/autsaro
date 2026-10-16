/** @file Independent explicit RTE value, status, instance and synchronous-call consumer.
 * Verifies: SWS_Rte_01330, SWS_Rte_01332, SWS_Rte_01334, SWS_Rte_03770
 * Verifies: SWS_Rte_07381, SWS_Rte_07382, SWS_Rte_07383, SWS_Rte_07655
 * Verifies: SWS_Rte_03837, SWS_Rte_03711, SWS_Rte_03712, SWS_Rte_03715
 * Verifies: SWS_Rte_03793, SWS_Rte_06830, SWS_Rte_07822
 */
#define RTE_CORE
#include "Rte_Ingress.h"
#include "Rte_Process.h"
#include "Rte_Observe.h"
#include "Rte_DcmService.h"
#include "Ecu_Target.h"
#include "Com.h"
#include <assert.h>

/** Selected receiver ComSpec status before the first publication. */
#ifdef HANDLE_NEVER_DISABLED
#define INITIAL_STATUS RTE_E_OK
#else
#define INITIAL_STATUS RTE_E_NEVER_RECEIVED
#endif

static uint8 com_result;
static uint32 server_calls;
static uint32 sends;

int Ecu_TargetIsOwner(void) { return 1; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
    return E_OK;
}
uint8 Com_SendSignal(Com_SignalIdType signal, const void *data)
{
    (void)signal;
    (void)data;
    ++sends;
    return com_result;
}
uint8 Com_ReceiveSignal(Com_SignalIdType signal, void *data)
{
    (void)signal;
    if (com_result == E_OK) {
        *(uint32 *)data = 72u;
    }
    return com_result;
}
void Ingress_Periodic(void) {}
void Process_Periodic(void) {}
void Observe_Periodic(void) {}
Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    Data[0] = 1u;
    Data[1] = 2u;
    Data[2] = 3u;
    Data[3] = 4u;
    return E_OK;
}
void Process_Transform(uint32 Input, uint32 *Output, uint32 *State)
{
    ++server_calls;
    *Output = Input + 3u;
    *State += Input;
}
static void expect(Std_ReturnType status, uint32 first, uint32 second)
{
    uint32 value = 99u;
    assert(Rte_Read_Process_Value_Value(&value) == status);
    assert(value == first);
    value = 88u;
    assert(Rte_Read_Observe_Value_Value(&value) == status);
    assert(value == second);
}
int main(void)
{
    uint32 output = 91u;
    uint32 state = 92u;
    uint32 network = 99u;
    assert(Rte_Start() == RTE_E_OK);
    assert(Rte_Inst_Ingress == &Rte_Instance_Ingress);
    assert(Rte_Inst_Process == &Rte_Instance_Process);
    assert(Rte_Inst_Observe == &Rte_Instance_Observe);
    assert(Rte_Inst_DcmService == &Rte_Instance_DcmService);
    assert(Rte_Inst_Process->Rte_Dummy == 0u);
#ifdef UNCONNECTED_PORTS
    expect(RTE_E_UNCONNECTED, 7u, 9u);
    assert(Rte_Write_Ingress_Value_Value(0u) == RTE_E_OK);
    assert(Rte_Write_Ingress_Value_Value(42u) == RTE_E_OK);
    expect(RTE_E_UNCONNECTED, 7u, 9u);
    assert(Rte_Call_Observe_ResultService_Transform(11u, &output, &state) == RTE_E_UNCONNECTED);
    assert(Rte_Call_Observe_ResultService_Transform(11u, NULL_PTR, NULL_PTR) == RTE_E_UNCONNECTED);
    assert(output == 91u && state == 92u && server_calls == 0u);
#else
    expect(INITIAL_STATUS, 7u, 9u);
    expect(INITIAL_STATUS, 7u, 9u);
    assert(Rte_Write_Ingress_Value_Value(0u) == RTE_E_OK);
    expect(RTE_E_OK, 0u, 0u);
    assert(Rte_Write_Ingress_Value_Value(42u) == RTE_E_OK);
    expect(RTE_E_OK, 42u, 42u);
    assert(Rte_Call_Observe_ResultService_Transform(11u, &output, &state) == RTE_E_OK);
    assert(output == 14u && state == 103u && server_calls == 1u);
#endif
    assert(sends == 0u);
    assert(Rte_Write_Ingress_TxValue_Value(42u) == RTE_E_OK);
    assert(sends == 1u);
    assert(Rte_Read_Ingress_RxValue_Value(&network) == INITIAL_STATUS);
    assert(network == 0u);
    com_result = COM_SERVICE_NOT_AVAILABLE;
    assert(Rte_Write_Ingress_TxValue_Value(43u) == RTE_E_COM_STOPPED);
    network = 99u;
    assert(Rte_Read_Ingress_RxValue_Value(&network) == RTE_E_COM_STOPPED);
    assert(network == 0u);
    com_result = E_OK;
    assert(Rte_Write_Ingress_TxValue_Value(44u) == RTE_E_OK);
    assert(sends == 3u);
    assert(SCHM_E_LOST_DATA == 64u);
    assert(Rte_IsInfrastructureError(RTE_E_UNCONNECTED));
    assert(!Rte_IsInfrastructureError(RTE_E_INVALID));
    assert(Rte_HasOverlayedError(RTE_E_MAX_AGE_EXCEEDED));
    assert(Rte_ApplicationError(RTE_E_MAX_AGE_EXCEEDED | 3u) == 3u);
    return 0;
}
