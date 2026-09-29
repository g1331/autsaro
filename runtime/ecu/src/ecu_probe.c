#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Windows.h"
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
        ExitProcess(93u);
    }
}
#endif
static DWORD WINAPI control(void *argument) {
    uint64_t step;
    DWORD started = GetTickCount();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((GetTickCount() - started) < 5000u);
        Sleep(1u);
    }
    for (step = UINT64_C(1); step <= UINT64_C(20); ++step) {
        uint64_t ticket;
        Os_TickCompletion completion;
        require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = GetTickCount();
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
            require((status == E_OS_NOFUNC) && ((GetTickCount() - started) < 5000u));
            Sleep(1u);
        }
        require((completion.epoch == step) && (completion.kernel_tick == (uint32_t)step));
    }
    require(printf("ecu_probe completed=20\n") >= 0);
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv) {
    HANDLE thread;
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
    thread = CreateThread(NULL, 262144u, control, NULL, 0u, NULL);
    if (thread == NULL) {
        return 96;
    }
    if (CloseHandle(thread) == 0) {
        return 95;
    }
    StartOS(1u);
    return 94;
}
