/** 独立通知消费者：仅 owner/lifecycle 为测试适配，COM 与生成 RTE 均为真实 producer。
 * 同一配置同时含两个网络接收者（handle 43 与 61，均不是模板的 17），逐项核对状态互相独立。
 * Verifies: SWS_Com_91019、91020 */
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
static Com_SignalIdType last_signal = 0xffffu;

uint8 __real_Com_ReceiveSignal(Com_SignalIdType signal, void *data);
uint8 __wrap_Com_ReceiveSignal(Com_SignalIdType signal, void *data);

uint8 __wrap_Com_ReceiveSignal(Com_SignalIdType signal, void *data)
{
    ++receive_calls;
    last_signal = signal;
    return __real_Com_ReceiveSignal(signal, data);
}

int Ecu_TargetIsOwner(void) { return test_owner; }

Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
    return (lifecycle_allowed == TRUE) ? E_OK : E_NOT_OK;
}

#define FIRST_HANDLE   43u
#define SECOND_HANDLE  61u
#define UNKNOWN_HANDLE 59u

static Std_ReturnType read_first(uint32 *value)
{
    return Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(value);
}

static Std_ReturnType read_second(uint32 *value)
{
    return Rte_Application_Pipeline_IngressInstance_Read_RxSecond_Value(value);
}

/* 读取两个接收者并核对各自的状态与值。 */
static void expect(Std_ReturnType first_status,
                   uint32 first_value,
                   Std_ReturnType second_status,
                   uint32 second_value)
{
    uint32 value = 99u;
    Std_ReturnType status = read_first(&value);
    assert(status == first_status);
    assert(value == first_value);
    value = 99u;
    status = read_second(&value);
    assert(status == second_status);
    assert(value == second_value);
}

static void tick(uint8 count)
{
    uint8 index;
    for (index = 0u; index < count; ++index) {
        Ecu_ComMainFunctionRx();
    }
}

int main(void)
{
    /* R24-11 handle-bearing DRAFT 接缝；预期不读取 ABI 清单。 */
    void (*const receive)(CbkHandleIdType handle) = &Rte_COMCbk;
    void (*const timeout)(CbkHandleIdType handle) = &Rte_COMCbkRxTOut;
    /* 两条接收 PDU 的超时分别为 30 与 50 个 Rx 主函数周期；PDU/信号编号按配置容器名排序。 */
    static const Com_PduConfigType pdus[2] = {{0u, 1u, TRUE, 0u, 30u, FIRST_HANDLE},
                                              {4u, 0u, TRUE, 0u, 50u, SECOND_HANDLE}};
    const Com_ConfigType configuration = {pdus, 2u, 0u, receive, timeout, NULL_PTR};
    uint8 first_payload[4] = {21u, 0u, 0u, 0u};
    uint8 second_payload[4] = {33u, 0u, 0u, 0u};
    const PduInfoType first_info = {first_payload, NULL_PTR, 4u};
    const PduInfoType second_info = {second_payload, NULL_PTR, 4u};
    Std_ReturnType status;

    Com_Init(&configuration);
    Com_IpduGroupStart(0u, TRUE);
    /* 接收超时监控只在 Com_EnableReceptionDM 之后生效。 */
    Com_EnableReceptionDM(0u);
    receive(FIRST_HANDLE);
    receive(SECOND_HANDLE);
    timeout(FIRST_HANDLE);
    timeout(SECOND_HANDLE);
    assert(receive_calls == 0u);
    lifecycle_allowed = FALSE;
    status = Rte_Start();
    assert(status == RTE_E_LIMIT);
    expect(RTE_E_COM_STOPPED, 99u, RTE_E_COM_STOPPED, 99u);
    lifecycle_allowed = TRUE;
    status = Rte_Start();
    assert(status == E_OK);
    timeout(FIRST_HANDLE);
    timeout(SECOND_HANDLE);
    expect(RTE_E_NEVER_RECEIVED, 7u, RTE_E_NEVER_RECEIVED, 9u);

    test_owner = 0;
    receive(FIRST_HANDLE);
    receive(SECOND_HANDLE);
    test_owner = 1;
    expect(RTE_E_NEVER_RECEIVED, 7u, RTE_E_NEVER_RECEIVED, 9u);

    /* 未知 handle 不得把 COM 初值当成首个已收到值，也不得影响任一接收者。 */
    receive(UNKNOWN_HANDLE);
    expect(RTE_E_NEVER_RECEIVED, 7u, RTE_E_NEVER_RECEIVED, 9u);

    /* 一个 PDU 的接收不得让另一个显示为已接收。 */
    receive_calls = 0u;
    Com_RxIndication(0u, &first_info);
    /* handle 43 只读取自己的 COM 信号（编号 1）。 */
    assert((receive_calls == 1u) && (last_signal == 1u));
    expect(E_OK, 21u, RTE_E_NEVER_RECEIVED, 9u);
    timeout(UNKNOWN_HANDLE);
    expect(E_OK, 21u, RTE_E_NEVER_RECEIVED, 9u);
    receive_calls = 0u;
    Com_RxIndication(4u, &second_info);
    assert((receive_calls == 1u) && (last_signal == 0u));
    expect(E_OK, 21u, E_OK, 33u);

    /* 两个超时计时独立：第 30 个周期仅第一个过期，第 50 个周期第二个过期。 */
    tick(29u);
    expect(E_OK, 21u, E_OK, 33u);
    tick(1u);
    expect(RTE_E_MAX_AGE_EXCEEDED, 21u, E_OK, 33u);
    tick(19u);
    expect(RTE_E_MAX_AGE_EXCEEDED, 21u, E_OK, 33u);
    tick(1u);
    expect(RTE_E_MAX_AGE_EXCEEDED, 21u, RTE_E_MAX_AGE_EXCEEDED, 33u);

    /* 非 owner 回调不得清除过期状态；真实接收通知只恢复对应的接收者。 */
    test_owner = 0;
    receive(FIRST_HANDLE);
    receive(SECOND_HANDLE);
    test_owner = 1;
    expect(RTE_E_MAX_AGE_EXCEEDED, 21u, RTE_E_MAX_AGE_EXCEEDED, 33u);
    first_payload[0] = 42u;
    Com_RxIndication(0u, &first_info);
    expect(E_OK, 42u, RTE_E_MAX_AGE_EXCEEDED, 33u);
    test_owner = 0;
    timeout(FIRST_HANDLE);
    timeout(SECOND_HANDLE);
    test_owner = 1;
    expect(E_OK, 42u, RTE_E_MAX_AGE_EXCEEDED, 33u);
    second_payload[0] = 77u;
    Com_RxIndication(4u, &second_info);
    expect(E_OK, 42u, E_OK, 77u);

    /* 重新接收后各自重新计时：第一个 30 周期后过期，第二个仍在 50 周期内。 */
    tick(29u);
    expect(E_OK, 42u, E_OK, 77u);
    tick(1u);
    expect(RTE_E_MAX_AGE_EXCEEDED, 42u, E_OK, 77u);

    Com_IpduGroupStop(0u);
    receive(FIRST_HANDLE);
    receive(SECOND_HANDLE);
    expect(RTE_E_COM_STOPPED, 42u, RTE_E_COM_STOPPED, 77u);
    Com_IpduGroupStart(0u, FALSE);
    lifecycle_allowed = FALSE;
    status = Rte_Stop();
    assert(status == RTE_E_LIMIT);
    expect(RTE_E_MAX_AGE_EXCEEDED, 42u, E_OK, 77u);
    lifecycle_allowed = TRUE;
    status = Rte_Stop();
    assert(status == E_OK);
    receive_calls = 0u;
    receive(FIRST_HANDLE);
    receive(SECOND_HANDLE);
    timeout(FIRST_HANDLE);
    timeout(SECOND_HANDLE);
    assert(receive_calls == 0u);
    expect(RTE_E_COM_STOPPED, 99u, RTE_E_COM_STOPPED, 99u);
    Com_DeInit();
    return 0;
}
