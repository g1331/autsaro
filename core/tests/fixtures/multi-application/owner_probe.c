/** Normal ECU test control: automotive observations occur only on the owner. */
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Host.h"
#include "Com.h"
#include "Dcm.h"
#include "SchM_Dcm.h"
#include "CanIf.h"
#include "SchM_Can.h"
#define RTE_CORE
#include "Rte_Ingress.h"
#include "Rte_Process.h"
#include "Rte_Observe.h"
#include "Rte_Main.h"
#include <stdio.h>
#include <string.h>

void MultiTest_Copy(uint32 result[10]);
static Os_Atomic32 snapshot_lock;
static unsigned after_main;
static unsigned canonical;
static unsigned controls;
static unsigned epoch_zero;
static unsigned late_transport;
static uint64_t observed_epoch;
static uint64_t observed_receptions;
static uint32 observed_value;
static Std_ReturnType observed_status;
static void require_at(int condition, unsigned line) {
    if (condition == 0) {
        (void)fprintf(stderr, "owner expectation failed at line %u\n", line);
        Os_HostExit(93u);
    }
}
#define require(condition) require_at((condition), __LINE__)
static void lock_snapshot(void) {
    while (Os_HostAtomicCompareExchange(&snapshot_lock, 1, 0) != 0) {
        Os_HostSleepMs(1u);
    }
}
static void unlock_snapshot(void) { (void)Os_HostAtomicExchange(&snapshot_lock, 0); }
int Ecu_TargetTestFailStage(unsigned stage) {
    uint32 value = 0u;
    if ((late_transport != 0u) && (stage == 10u)) {
        PduLengthType available = 99u;
        const PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
        require(Dcm_CopyTxData(0u, &query, NULL_PTR, &available) == BUFREQ_OK);
        require(available == 0u);
        if (Ecu_TargetNow() == 1u) {
            require(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
            Can_MainFunction_Mode();
            available = 99u;
            require(Dcm_CopyTxData(0u, &query, NULL_PTR, &available) == BUFREQ_E_NOT_OK);
            require(available == 99u);
            require(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
            Can_MainFunction_Mode();
        }
    }
    if ((late_transport != 0u) && (stage == 11u) && (Ecu_TargetNow() == 2u)) {
        PduLengthType available = 99u;
        const PduInfoType query = {NULL_PTR, NULL_PTR, 0u};
        require(Dcm_CopyTxData(0u, &query, NULL_PTR, &available) == BUFREQ_E_NOT_OK);
        require(available == 99u);
    }
    if ((stage == 9u) && (Ecu_TargetNow() == 1u)) {
        require(SchM_Mode_Dcm_DcmAuthenticationState_Physical() ==
                RTE_MODE_DcmAuthenticationState_Physical_DCM_DEAUTHENTICATED);
    }
    if ((stage == 9u) && (Ecu_TargetNow() == 1u) && (epoch_zero == 0u)) {
        uint32 com_value = 99u;
        require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) ==
                RTE_E_NEVER_RECEIVED);
        require(value == 7u);
        require(Com_ReceiveSignal(0u, &com_value) == E_OK);
        require(com_value == 0u);
        Com_IpduGroupStop(0u);
        require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) ==
                RTE_E_COM_STOPPED);
        require(value == 7u);
        Com_IpduGroupStart(0u, FALSE);
        require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) ==
                RTE_E_NEVER_RECEIVED);
        require(value == 7u);
        Com_IpduGroupStop(0u);
        Com_IpduGroupStart(0u, TRUE);
        require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) ==
                RTE_E_NEVER_RECEIVED);
        require(value == 7u);
        require(Rte_Application_Pipeline_ProcessInstance_Read_Value_Value(&value) == E_OK);
        require(value == 7u);
        require(Rte_Application_Pipeline_ObserveInstance_Read_Value_Value(&value) == E_OK);
        require(value == 9u);
        require(Rte_Application_Pipeline_IngressInstance_Write_Value_Value(42u) == E_OK);
        require(Rte_Application_Pipeline_ProcessInstance_Read_Value_Value(&value) == E_OK);
        require(value == 42u);
        require(Rte_Application_Pipeline_ObserveInstance_Read_Value_Value(&value) == E_OK);
        require(value == 42u);
        {
            uint32 result = 99u;
            uint32 state = 98u;
            require(Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform(
                        42u, NULL_PTR, &state) == E_NOT_OK);
            require(state == 98u);
            require(Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform(
                        42u, &result, NULL_PTR) == E_NOT_OK);
            require(result == 99u);
        }
    }
    if ((controls != 0u) && (stage == 9u)) {
        const uint64_t epoch = Ecu_TargetNow();
        if (epoch == 2u) {
            Com_DisableReceptionDM(0u);
        }
        if ((epoch == 41u) || (epoch == 43u)) {
            Com_EnableReceptionDM(0u);
        }
        if (epoch == 73u) {
            Com_IpduGroupStop(0u);
        }
        if (epoch == 74u) {
            Com_IpduGroupStart(0u, FALSE);
        }
        if (epoch == 75u) {
            uint32 com_value = 99u;
            Com_IpduGroupStop(0u);
            Com_IpduGroupStart(0u, TRUE);
            require(Com_ReceiveSignal(0u, &com_value) == E_OK);
            require(com_value == 0u);
            require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) == E_OK);
            require(value == 0u);
        }
        if (epoch == 76u) {
            require(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
            Can_MainFunction_Mode();
            require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) == E_OK);
            require(value == 45u);
            require(Rte_Application_Pipeline_IngressInstance_Write_TxValue_Value(77u) == E_OK);
            require(Com_TriggerIPDUSend(1u) == E_NOT_OK);
            require(Rte_Application_Pipeline_IngressInstance_Write_Value_Value(99u) == E_OK);
            require(Rte_Application_Pipeline_ProcessInstance_Read_Value_Value(&value) == E_OK);
            require(value == 99u);
            require(Rte_Application_Pipeline_ObserveInstance_Read_Value_Value(&value) == E_OK);
            require(value == 99u);
            require(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
            Can_MainFunction_Mode();
        }
        if (epoch == 110u) {
            Com_DeInit();
            value = 99u;
            require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value) ==
                    RTE_E_COM_STOPPED);
            require(value == 45u);
            require(Rte_Application_Pipeline_IngressInstance_Write_TxValue_Value(77u) ==
                    RTE_E_COM_STOPPED);
        }
    }
    if ((stage == 10u) || (stage == 11u)) {
        uint32 counts[10];
        const uint32 periods = (uint32)(Ecu_TargetNow() / 10u);
        MultiTest_Copy(counts);
        require((counts[0] == periods) && (counts[1] == periods) && (counts[2] == periods) &&
                (counts[3] == periods));
        require((counts[4] == 0u) && (counts[5] == 0u));
        if (periods != 0u) {
            const uint32 result =
                (controls != 0u)
                    ? ((periods < 3u) ? 22u : ((periods < 5u) ? 43u : ((periods < 8u) ? 44u : 46u)))
                    : ((canonical != 0u) ? 0x12345679u : ((Ecu_TargetNow() < 40u) ? 22u : 43u));
            require((counts[6] == result) && (counts[7] == result) && (counts[8] == result));
            require(counts[9] == E_OK);
        }
    }
    if ((stage == 10u) || (stage == 11u)) {
        Std_ReturnType status = Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&value);
        lock_snapshot();
        if (stage == 11u) {
            ++observed_receptions;
        }
        observed_epoch = Ecu_TargetNow();
        observed_value = value;
        observed_status = status;
        unlock_snapshot();
    }
    return 0;
}
void Ecu_TargetTestShutdown(StatusType reason) { require(reason == E_OK); }
static void drain(void) {
    Ecu_OutputRecord output;
    StatusType status = Ecu_TargetTakeOutput(&output);
    if (status == E_OK) {
        require(printf("TX %llu %u\n", (unsigned long long)output.epoch, output.can_id) >= 0);
        require(fflush(stdout) == 0);
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
    } else {
        require(status == E_OS_NOFUNC);
    }
}
static void report(const char *phase) {
    uint64_t stamp;
    uint32 value;
    Std_ReturnType status;
    lock_snapshot();
    stamp = observed_epoch;
    value = observed_value;
    status = observed_status;
    unlock_snapshot();
    require(printf("OBS %s %llu %u %u\n", phase, (unsigned long long)stamp, status, value) >= 0);
    require(fflush(stdout) == 0);
}
static void receive_result(uint64_t stamp, uint32 value, uint32 expected_value,
                           Std_ReturnType expected_status) {
    const uint8 data[8] = {(uint8)value,
                           (uint8)(value >> 8u),
                           (uint8)(value >> 16u),
                           (uint8)(value >> 24u),
                           0u,
                           0u,
                           0u,
                           0u};
    uint64_t ticket;
    const uint64_t started = Os_HostMonotonicMs();
    unsigned received = 0u;
    uint64_t previous;
    lock_snapshot();
    previous = observed_receptions;
    unlock_snapshot();
    require(Ecu_TargetPostFrame(stamp, 800u, 4u, data, &ticket) == E_OK);
    while (received == 0u) {
        lock_snapshot();
        received = ((observed_receptions > previous) && (observed_epoch == stamp) &&
                    (observed_value == expected_value) && (observed_status == expected_status))
                       ? 1u
                       : 0u;
        unlock_snapshot();
        drain();
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    report("rx");
}
static void receive(uint64_t stamp, uint32 value) { receive_result(stamp, value, value, E_OK); }
static void diagnostic_request(uint64_t stamp) {
    const uint8 request[8] = {3u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    uint64_t ticket;
    uint64_t previous;
    uint64_t started = Os_HostMonotonicMs();
    unsigned received = 0u;
    lock_snapshot();
    previous = observed_receptions;
    unlock_snapshot();
    require(Ecu_TargetPostFrame(stamp, 1792u, 8u, request, &ticket) == E_OK);
    while (received == 0u) {
        lock_snapshot();
        received = (observed_receptions > previous) ? 1u : 0u;
        unlock_snapshot();
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
}
static void take_response(Ecu_OutputRecord *output, uint64_t stamp) {
    const uint8 expected[8] = {7u, 0x62u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    const uint64_t started = Os_HostMonotonicMs();
    StatusType status;
    do {
        status = Ecu_TargetTakeOutput(output);
        require((status == E_OK) || (status == E_OS_NOFUNC));
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    } while (status != E_OK);
    require(output->epoch == stamp && output->can_id == 1800u && output->dlc == 8u);
    require(memcmp(output->data, expected, sizeof(expected)) == 0);
}
static void complete_response(const Ecu_OutputRecord *output) {
    unsigned i;
    require(printf("LATE %llu 1800 ", (unsigned long long)output->epoch) >= 0);
    for (i = 0u; i < 8u; ++i) {
        require(printf("%02x", (unsigned)output->data[i]) >= 0);
    }
    require(printf("\n") >= 0);
    require(fflush(stdout) == 0);
    require(Ecu_TargetConfirmOutput(output->ticket, output->pdu) == E_OK);
}
static void wait_tick(uint64_t ticket) {
    Os_TickCompletion completion;
    const uint64_t started = Os_HostMonotonicMs();
    while (Os_TargetTickCompletion(ticket, &completion) != E_OK) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
}
static Os_HostThreadResult control(void *argument) {
    uint64_t step;
    uint64_t started = Os_HostMonotonicMs();
    uint32 sentinel = 99u;
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    require(Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(&sentinel) == E_NOT_OK);
    require(sentinel == 99u);
    require(Rte_Start() == RTE_E_LIMIT);
    require(Rte_Stop() == RTE_E_LIMIT);
    if (late_transport != 0u) {
        uint64_t ticket = 0u;
        uint64_t refused = 99u;
        Ecu_OutputRecord old;
        Ecu_OutputRecord current;
        diagnostic_request(0u);
        require(Os_TargetAdvanceOneTick(1u, &ticket) == E_OK);
        take_response(&old, 1u);
        require(Os_TargetAdvanceOneTick(2u, &refused) == E_OS_STATE);
        require(refused == 99u);
        {
            const uint8 ignored[8] = {21u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
            require(Ecu_TargetPostFrame(1u, 800u, 4u, ignored, &refused) == E_OS_STATE);
            require(refused == 99u);
        }
        require(Os_TargetAdvanceOneTick(2u, &refused) == E_OS_STATE);
        require(refused == 99u);
        complete_response(&old);
        wait_tick(ticket);
        receive(1u, 21u);
        diagnostic_request(1u);
        require(Os_TargetAdvanceOneTick(2u, &ticket) == E_OK);
        take_response(&current, 2u);
        require(current.pdu == old.pdu && current.ticket != old.ticket);
        complete_response(&current);
        wait_tick(ticket);
        receive(2u, 21u);
        ShutdownOS(E_OK);
        return 0u;
    }
    if (epoch_zero != 0u) {
        receive(0u, 21u);
    }
    for (step = 1u;
         step <=
         ((epoch_zero != 0u) ? 30u : ((controls != 0u) ? 120u : ((canonical != 0u) ? 20u : 61u)));
         ++step) {
        uint64_t ticket;
        Os_TickCompletion completion;
        if ((step == 31u) && (after_main == 0u) && (controls == 0u)) {
            receive(step, 42u);
            receive(step, 42u);
        }
        require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = Os_HostMonotonicMs();
        while (Os_TargetTickCompletion(ticket, &completion) != E_OK) {
            drain();
            require((Os_HostMonotonicMs() - started) < 5000u);
            Os_HostSleepMs(1u);
        }
        require(completion.epoch == step);
        report("main");
        if ((step == 1u) && (epoch_zero == 0u)) {
            receive(step, (canonical != 0u) ? 0x12345678u : 21u);
        }
        if (controls != 0u) {
            if (step == 20u) {
                receive(step, 42u);
            }
            if (step == 42u) {
                receive(step, 43u);
            }
            if (step == 73u) {
                receive_result(step, 99u, 43u, RTE_E_COM_STOPPED);
            }
            if (step == 74u) {
                receive(step, 44u);
            }
            if (step == 75u) {
                receive(step, 45u);
            }
        }
        if ((step == 31u) && (after_main != 0u) && (controls == 0u)) {
            receive(step, 42u);
            receive(step, 42u);
        }
    }
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv) {
    Os_HostHandle thread;
    if ((argc != 2) || ((strcmp(argv[1], "before") != 0) && (strcmp(argv[1], "after") != 0) &&
                        (strcmp(argv[1], "canonical") != 0) && (strcmp(argv[1], "controls") != 0) &&
                        (strcmp(argv[1], "epoch0") != 0) && (strcmp(argv[1], "late") != 0))) {
        return 98;
    }
    after_main = (strcmp(argv[1], "after") == 0) ? 1u : 0u;
    canonical = (strcmp(argv[1], "canonical") == 0) ? 1u : 0u;
    late_transport = (strcmp(argv[1], "late") == 0) ? 1u : 0u;
    epoch_zero = (strcmp(argv[1], "epoch0") == 0) ? 1u : 0u;
    controls = (strcmp(argv[1], "controls") == 0) ? 1u : 0u;
    require(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(control, NULL);
    require(thread != NULL);
    require(Os_HostClose(thread) != 0);
    StartOS(1u);
    return 94;
}
