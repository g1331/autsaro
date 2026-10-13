/** 独立通知消费者：仅 owner/lifecycle 为测试适配，COM 与生成 RTE 均为真实 producer。 */
#define RTE_CORE
#include "Rte_Com.h"
#include "Rte_Main.h"
#include "Rte_Ingress.h"
#include "Ecu_Target.h"
#include "Com.h"
#include "Com_Internal.h"
#include <assert.h>

static int test_owner = 1;
static boolean lifecycle_allowed = TRUE;
static uint32 receive_calls;

uint8 __real_Com_ReceiveSignal(Com_SignalIdType signal, void *data);
uint8 __wrap_Com_ReceiveSignal(Com_SignalIdType signal, void *data);

uint8 __wrap_Com_ReceiveSignal(Com_SignalIdType signal, void *data) {
    ++receive_calls;
    return __real_Com_ReceiveSignal(signal, data);
}

int Ecu_TargetIsOwner(void) { return test_owner; }

Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting) {
    (void)starting;
    return (lifecycle_allowed == TRUE) ? E_OK : E_NOT_OK;
}

int main(void) {
    /* R24-11 handle-bearing DRAFT 接缝；预期不读取 ABI 清单。 */
    void (*const receive)(CbkHandleIdType handle) = &Rte_COMCbk;
    void (*const timeout)(CbkHandleIdType handle) = &Rte_COMCbkRxTOut;
    static const Com_PduConfigType pdus[1] = {{0u, 0u, TRUE, 0u, 30u, TEST_RX_HANDLE}};
    const Com_ConfigType configuration = {pdus, 1u, 0u, receive, timeout, NULL_PTR};
    uint32 value = 99u;
    uint8 payload[4] = {21u, 0u, 0u, 0u};
    const PduInfoType info = {payload, NULL_PTR, 4u};
    Std_ReturnType status;
    uint8 index;

    Com_Init(&configuration);
    Com_IpduGroupStart(0u, TRUE);
    receive(TEST_RX_HANDLE);
    timeout(TEST_RX_HANDLE);
    assert(receive_calls == 0u);
    lifecycle_allowed = FALSE;
    status = Rte_Start();
    assert(status == RTE_E_LIMIT);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_COM_STOPPED);
    assert(value == 99u);
    lifecycle_allowed = TRUE;
    status = Rte_Start();
    assert(status == E_OK);
    timeout(TEST_RX_HANDLE);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_NEVER_RECEIVED);
    assert(value == 7u);

    test_owner = 0;
    receive(TEST_RX_HANDLE);
    test_owner = 1;
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_NEVER_RECEIVED);
    assert(value == 7u);

    /* 未知 handle 不得把 COM 初值当成首个已收到值。 */
    receive(59u);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_NEVER_RECEIVED);
    Com_RxIndication(0u, &info);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == E_OK);
    assert(value == 21u);
    timeout(59u);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == E_OK);

    for (index = 0u; index < 29u; ++index) {
        Ecu_ComMainFunctionRx();
        status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
        assert(status == E_OK);
        assert(value == 21u);
    }
    Ecu_ComMainFunctionRx();
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_MAX_AGE_EXCEEDED);
    assert(value == 21u);

    /* 非 owner 回调不得清除过期状态；真实接收通知才恢复。 */
    test_owner = 0;
    receive(TEST_RX_HANDLE);
    test_owner = 1;
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_MAX_AGE_EXCEEDED);
    payload[0] = 42u;
    Com_RxIndication(0u, &info);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == E_OK);
    assert(value == 42u);
    test_owner = 0;
    timeout(TEST_RX_HANDLE);
    test_owner = 1;
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == E_OK);

    for (index = 0u; index < 29u; ++index) {
        Ecu_ComMainFunctionRx();
        status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
        assert(status == E_OK);
        assert(value == 42u);
    }
    Ecu_ComMainFunctionRx();
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_MAX_AGE_EXCEEDED);
    assert(value == 42u);

    Com_IpduGroupStop(0u);
    receive(TEST_RX_HANDLE);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_COM_STOPPED);
    assert(value == 42u);
    Com_IpduGroupStart(0u, FALSE);
    lifecycle_allowed = FALSE;
    status = Rte_Stop();
    assert(status == RTE_E_LIMIT);
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_MAX_AGE_EXCEEDED);
    assert(value == 42u);
    lifecycle_allowed = TRUE;
    status = Rte_Stop();
    assert(status == E_OK);
    receive_calls = 0u;
    receive(TEST_RX_HANDLE);
    timeout(TEST_RX_HANDLE);
    assert(receive_calls == 0u);
    value = 99u;
    status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
    assert(status == RTE_E_COM_STOPPED);
    assert(value == 99u);
    Com_DeInit();
    return 0;
}
