/** @file Exercise the generated BswM binding when a FULL indication arrives after CAN stops. */
#include "BswM.h"
#include "BswM_ComM.h"
#include "Can.h"
#include "Can_Cfg.h"
#include "CanIf_Cfg.h"
#include "ComM_Cfg.h"
#include "Dcm.h"
#include "Ecu_HostBusSM.h"
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Host.h"
#include "SchM_Can.h"
#include <SchM_ComM.h>
#include <assert.h>

void __real_BswM_Init(const BswM_ConfigType *ConfigPtr);
void __wrap_BswM_Init(const BswM_ConfigType *ConfigPtr);
Std_ReturnType __wrap_Ecu_StackRunCycle(EventMaskType events);

static BswM_ModeActionType generated_action;

void __wrap_BswM_Init(const BswM_ConfigType *ConfigPtr)
{
    generated_action = ConfigPtr->action;
    __real_BswM_Init(ConfigPtr);
}

/* After bootstrap publishes READY, the probe owns the controller polls and
 * observations. Hold the cyclic work so the queued STOP indication cannot race it. */
Std_ReturnType __wrap_Ecu_StackRunCycle(EventMaskType events)
{
    (void)events;
    return E_OK;
}

static void expect_mode(ComM_ModeType expected, Rte_ModeType_ComMMode published)
{
    ComM_ModeType actual = 0xffu;
    assert(ComM_GetCurrentComMode(ComMConf_ComMUser_HostUser, &actual) == E_OK);
    assert(actual == expected);
    assert(SchM_Mode_ComM_UM_HostUser() == published);
}

static Os_HostThreadResultType observe(void *argument)
{
    const uint64_t started = Os_HostMonotonicMs();
    const NetworkHandleType channel = ComMConf_ComMChannel_Host;
    PduLengthType available = 0u;
    uint8 bytes[4] = {0u};
    const PduInfoType tx = {bytes, NULL_PTR, sizeof(bytes)};
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        assert((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    expect_mode(COMM_FULL_COMMUNICATION, RTE_MODE_ComMMode_COMM_FULL_COMMUNICATION);
    assert(generated_action != NULL_PTR);

    /* The driver has stopped, but its queued indication has not been polled. */
    assert(Can_SetControllerMode(CanConf_CanController_Controller, CAN_CS_STOPPED) == E_OK);
    expect_mode(COMM_NO_COMMUNICATION, RTE_MODE_ComMMode_COMM_FULL_COMMUNICATION);
    assert(generated_action(channel, COMM_FULL_COMMUNICATION) == E_NOT_OK);
    BswM_ComM_CurrentMode(channel, COMM_FULL_COMMUNICATION);
    expect_mode(COMM_NO_COMMUNICATION, RTE_MODE_ComMMode_COMM_FULL_COMMUNICATION);
    Ecu_HostBusSM_PublishPendingMode();
    expect_mode(COMM_NO_COMMUNICATION, RTE_MODE_ComMMode_COMM_NO_COMMUNICATION);
    assert(Dcm_StartOfReception(0u, NULL_PTR, 1u, &available) == BUFREQ_E_NOT_OK);
    assert(CanIf_Transmit(CanIfConf_CanIfTxPduCfg_TxValue, &tx) == E_NOT_OK);

    Can_MainFunction_Mode();
    assert(Ecu_HostBusSM_RequestComMode(channel, COMM_SILENT_COMMUNICATION) == E_OK);
    Can_MainFunction_Mode();
    expect_mode(COMM_SILENT_COMMUNICATION, RTE_MODE_ComMMode_COMM_SILENT_COMMUNICATION);
    assert(generated_action(channel, COMM_SILENT_COMMUNICATION) == E_OK);
    assert(CanIf_Transmit(CanIfConf_CanIfTxPduCfg_TxValue, &tx) == E_NOT_OK);

    assert(Ecu_HostBusSM_RequestComMode(channel, COMM_FULL_COMMUNICATION) == E_OK);
    expect_mode(COMM_FULL_COMMUNICATION, RTE_MODE_ComMMode_COMM_FULL_COMMUNICATION);
    assert(generated_action(channel, COMM_FULL_COMMUNICATION) == E_OK);
    assert(CanIf_Transmit(CanIfConf_CanIfTxPduCfg_TxValue, &tx) == E_OK);
    assert(Dcm_StartOfReception(0u, NULL_PTR, 1u, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(0u, E_NOT_OK);
    ShutdownOS(E_OK);
    return 0u;
}

int main(void)
{
    Os_HostHandleType thread;
    assert(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(&observe, NULL);
    assert(thread != NULL);
    assert(Os_HostClose(thread) != 0);
    StartOS(OSDEFAULTAPPMODE);
    return 1;
}
