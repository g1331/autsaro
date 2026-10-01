#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Host.h"
#include <stdio.h>
#include <stdlib.h>

static void require(int condition) {
    if (condition == 0) {
        ShutdownOS(E_OS_STATE);
    }
}
#ifdef ECU_TARGET_TESTS
static unsigned failure_stage;
int Ecu_TargetTestFailStage(unsigned stage) { return stage == failure_stage; }
void Ecu_TargetTestShutdown(StatusType reason) {
    if (printf("ecu_shutdown reason=%u state=%u fail_stage=%u\n", reason, Ecu_TargetState(),
               failure_stage) < 0) {
        Os_HostExit(93u);
    }
}
#endif
static Os_HostThreadResult control(void *argument) {
    uint64_t step;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    for (step = UINT64_C(1); step <= UINT64_C(20); ++step) {
        uint64_t ticket;
        Os_TickCompletion completion;
        require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = Os_HostMonotonicMs();
        for (;;) {
            Ecu_OutputRecord output;
            StatusType status = Ecu_TargetTakeOutput(&output);
            if (status == E_OK) {
                require(printf("ecu_output epoch=%llu ticket=%llu pdu=%u id=%u dlc=%u\n",
                               (unsigned long long)output.epoch, (unsigned long long)output.ticket,
                               output.pdu, output.can_id, output.dlc) >= 0);
                require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
            } else {
                require(status == E_OS_NOFUNC);
            }
            status = Os_TargetTickCompletion(ticket, &completion);
            if (status == E_OK) {
                break;
            }
            require((status == E_OS_NOFUNC) && ((Os_HostMonotonicMs() - started) < 5000u));
            Os_HostSleepMs(1u);
        }
        require((completion.epoch == step) && (completion.kernel_tick == (uint32_t)step));
    }
    require(printf("ecu_probe completed=20\n") >= 0);
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv) {
    Os_HostHandle thread;
#ifdef ECU_TARGET_TESTS
    if (argc == 2) {
        char *end;
        unsigned long value = strtoul(argv[1], &end, 10);
        if ((argv[1][0] < '0') || (argv[1][0] > '9') || (*end != '\0') || (value > 8u)) {
            return 98;
        }
        failure_stage = (unsigned)value;
    } else if (argc != 1) {
        return 98;
    }
#else
    (void)argv;
    if (argc != 1) {
        return 98;
    }
#endif
    if (Ecu_TargetPrepare() != E_OK) {
        return 97;
    }
    thread = Os_HostSpawnThread(control, NULL);
    if (thread == NULL) {
        return 96;
    }
    if (Os_HostClose(thread) == 0) {
        return 95;
    }
    StartOS(1u);
    return 94;
}
