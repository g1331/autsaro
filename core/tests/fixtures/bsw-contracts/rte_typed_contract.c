/** @file Real synchronous application server with independent public signatures.
 * Verifies: SWS_Rte_01360, SWS_Rte_01166, SWS_Rte_08913, SWS_Rte_02573
 * Verifies: SWS_Rte_07008, SWS_Rte_01334, SWS_Rte_03770
 */
#define RTE_CORE
#include "Rte_Process.h"
#include "Rte_Observe.h"
#include "Rte_Ingress.h"
#include "Ecu_Target.h"
#include "Com.h"
#include <stdio.h>
#include <stdlib.h>

/* A normal nonzero exit lets the owned runner distinguish a failed assertion from a signal. */
#define CHECK(condition)                                                                           \
    do {                                                                                           \
        if (!(condition)) {                                                                        \
            fprintf(stderr, "typed C/S assertion at line %d\n", __LINE__);                         \
            exit(1);                                                                               \
        }                                                                                          \
    } while (0)

static uint32 slots[2];
static uint32 calls;
static uint8 *expected_input;
static uint32 *expected_output;
static boolean *expected_state;
static int owner = 1;

int Ecu_TargetIsOwner(void) { return owner; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
    return E_OK;
}
uint8 Com_SendSignal(Com_SignalIdType signal, const void *data)
{
    (void)signal;
    (void)data;
    return E_OK;
}
uint8 Com_ReceiveSignal(Com_SignalIdType signal, void *data)
{
    (void)signal;
    (void)data;
    return E_OK;
}
void Ingress_Periodic(void) {}
void Process_Periodic(void) {}
void Observe_Periodic(void) {}
Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    Data[0] = 1U;
    Data[1] = 2U;
    Data[2] = 3U;
    Data[3] = 4U;
    return E_OK;
}
Std_ReturnType Process_Transform(
    uint8 slot, uint8 bias, uint8 Input, uint32 *Output, boolean *State)
{
    CHECK(slot < 2U);
    CHECK(Output == expected_output && State == expected_state);
    CHECK(Input == *expected_input);
    ++calls;
    if (Input == 250U) {
        return RTE_E_ScalarService_Busy;
    }
    slots[slot] += (uint32)Input + bias;
    *Output = slots[slot];
    *State = (*State == FALSE) ? TRUE : FALSE;
    return (Input == 251U) ? RTE_E_ScalarService_Denied : RTE_E_ScalarService_E_OK;
}
Std_ReturnType Process_Ping(uint8 slot, uint8 bias)
{
    CHECK(slot < 2U);
    slots[slot] += bias;
    ++calls;
    return E_OK;
}

int main(void)
{
    uint8 input = 5U;
    uint32 output = 91U;
    boolean state = FALSE;
    Std_ReturnType (*const first)(uint8, uint32 *, boolean *) =
        &Rte_Call_Observe_ResultService_Transform;
    Std_ReturnType (*const second)(uint8, uint32 *, boolean *) =
        &Rte_Call_Observe_OtherService_Transform;
    Std_ReturnType (*const ping)(void) = &Rte_Call_Observe_ResultService_Ping;
    Std_ReturnType (*const other_ping)(void) = &Rte_Call_Observe_OtherService_Ping;
    Std_ReturnType (*const server)(uint8, uint8, uint8, uint32 *, boolean *) = &Process_Transform;
    Std_ReturnType (*const ping_server)(uint8, uint8) = &Process_Ping;
    expected_input = &input;
    expected_output = &output;
    expected_state = &state;
    CHECK(server != NULL_PTR && ping_server != NULL_PTR);
    CHECK(first(input, &output, &state) == RTE_E_COM_STOPPED);
    CHECK(output == 91U && state == FALSE && calls == 0U);
    CHECK(Rte_Start() == E_OK);
#ifdef UNCONNECTED_PORTS
    CHECK(second(input, &output, &state) == RTE_E_UNCONNECTED);
    CHECK(output == 91U && state == FALSE && calls == 0U);
#else
    CHECK(second(input, &output, &state) == E_OK);
    CHECK(output == 34U && state == TRUE && slots[0] == 0U && slots[1] == 34U);
#endif
    CHECK(first(input, &output, &state) == E_OK);
    CHECK(output == 18U && slots[0] == 18U);
    CHECK(ping() == E_OK && slots[0] == 31U);
#ifndef UNCONNECTED_PORTS
    CHECK(other_ping() == E_OK && slots[1] == 63U);
#else
    CHECK(other_ping() == RTE_E_UNCONNECTED && slots[1] == 0U);
#endif
    input = 250U;
    output = 67U;
    state = TRUE;
    CHECK(first(input, &output, &state) == 7U);
    CHECK(output == 67U && state == TRUE && slots[0] == 31U);
    input = 251U;
    CHECK(first(input, &output, &state) == 63U);
    CHECK(output == 295U && state == FALSE && slots[0] == 295U);
    {
        const uint32 before = calls;
        owner = 0;
        CHECK(first(input, &output, &state) == E_NOT_OK);
        CHECK(output == 295U && state == FALSE && calls == before);
        owner = 1;
        CHECK(Rte_Stop() == E_OK);
        CHECK(first(input, &output, &state) == RTE_E_COM_STOPPED);
        CHECK(output == 295U && state == FALSE && calls == before);
        CHECK(slots[0] == 295U);
    }
    return 0;
}
