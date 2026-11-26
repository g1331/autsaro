/** @file Host probe executable that confirms every output during a selected tick window. */
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Ecu_ComRejection.h"
#include "Os_Host.h"
#include "Rte_Main.h"
#include "Os_HostIo.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

static uint32_t probe_ticks = 20u; /**< Main sets before the control thread starts. */

static void probe_require(boolean condition)
{
    if (condition == FALSE) {
        ShutdownOS(E_OS_STATE);
    }
}
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
static unsigned
    failure_stage; /**< Main initializes before OS startup; hooks only read; no Init reset. */
int Ecu_TargetTestFailStage(unsigned stage) { return (stage == failure_stage) ? 1 : 0; }
void Ecu_TargetTestRetire(unsigned stage)
{
    char storage[64];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "ecu_retire stage=", stage);
    Os_HostTextByte(&text, '\n');
    if (Os_HostTextWrite(&text) == 0) {
        Os_HostExit(93u);
    }
}
void Ecu_TargetTestShutdown(StatusType reason)
{
    char storage[128];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "ecu_shutdown reason=", reason);
    Os_HostTextField(&text, " state=", Ecu_TargetState());
    Os_HostTextField(&text, " fail_stage=", failure_stage);
    Os_HostTextField(&text, " schm_timing=", Ecu_SchMTimingActive());
    Os_HostTextByte(&text, '\n');
    if (Os_HostTextWrite(&text) == 0) {
        Os_HostExit(93u);
    }
}
#endif
static void write_probe_output(const Ecu_OutputRecord *output)
{
    char storage[192];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "ecu_output epoch=", output->epoch);
    Os_HostTextField(&text, " ticket=", output->ticket);
    Os_HostTextField(&text, " pdu=", output->pdu);
    Os_HostTextField(&text, " id=", output->can_id);
    Os_HostTextField(&text, " dlc=", output->dlc);
    Os_HostTextField(&text, " channel=", output->channel);
    if (output->data_phase_bitrate != 0u) {
        Os_HostTextField(&text, " brs=", output->brs ? 1u : 0u);
        Os_HostTextField(&text, " data_bitrate=", output->data_phase_bitrate);
    }
    Os_HostTextByte(&text, '\n');
    probe_require(Os_HostTextWrite(&text) != 0);
}
static Os_HostThreadResultType probe_control(void *argument)
{
    uint64_t step;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        probe_require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    for (step = UINT64_C(1); step <= probe_ticks; ++step) {
        uint64_t ticket;
        Os_TickCompletionType completion;
        probe_require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = Os_HostMonotonicMs();
        for (;;) {
            probe_require(Ecu_ComDrainRejections(FALSE) != 0);
            Ecu_OutputRecord output;
            StatusType status = Ecu_TargetTakeOutput(&output);
            if (status == E_OK) {
                write_probe_output(&output);
                probe_require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
            } else {
                probe_require(status == E_OS_NOFUNC);
            }
            status = Os_TargetTickCompletion(ticket, &completion);
            if (status == E_OK) {
                break;
            }
            probe_require(status == E_OS_NOFUNC);
            probe_require((Os_HostMonotonicMs() - started) < 5000u);
            Os_HostSleepMs(1u);
        }
        probe_require((completion.epoch == step) && (completion.kernel_tick == (uint32_t)step));
    }
    probe_require(Ecu_ComDrainRejections(TRUE) != 0);
    {
        char storage[64];
        Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
        Os_HostTextField(&text, "ecu_probe completed=", probe_ticks);
        Os_HostTextByte(&text, '\n');
        probe_require(Os_HostTextWrite(&text) != 0);
    }
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv)
{
    Os_HostHandleType thread;
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
    if (argc == 2) {
        char *end;
        unsigned long value;
        errno = 0;
        value = strtoul(argv[1], &end, 10);
        if ((argv[1][0] < '0') || (argv[1][0] > '9') || (*end != '\0') || (value > 8u)) {
            return 98;
        }
        failure_stage = (unsigned)value;
    } else if (argc != 1) {
        return 98;
    } else {
        /* No failure-stage argument keeps the default test selection. */
    }
#else
    if ((argc == 3) && (strcmp(argv[1], "--ticks") == 0)) {
        char *end;
        unsigned long long value;
        errno = 0;
        value = strtoull(argv[2], &end, 10);
        if ((errno != 0) || (argv[2][0] < '0') || (argv[2][0] > '9') || (*end != '\0') ||
            (value == 0u) || (value > UINT32_MAX)) {
            return 98;
        }
        probe_ticks = (uint32_t)value;
    } else if (argc != 1) {
        return 98;
    } else {
        /* Standalone use keeps the default observation window. */
    }
#endif
    if (Ecu_TargetPrepare() != E_OK) {
        return 97;
    }
    thread = Os_HostSpawnThread(&probe_control, NULL);
    if (thread == NULL) {
        return 96;
    }
    if (Os_HostClose(thread) == 0) {
        return 95;
    }
    StartOS(OSDEFAULTAPPMODE);
    return 94;
}
