/* Single-frame mailbox inputs share the HostBatch timer origin, without a batch receipt. */
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Host.h"
#include <stdio.h>
#include <string.h>

static void require_at(int accepted, unsigned line)
{
    if (accepted == 0) {
        (void)fprintf(stderr, "Frame timer assertion failed: line=%u\n", line);
        ShutdownOS(E_OS_STATE);
    }
}
#define require(accepted) require_at((accepted), __LINE__)

static Os_HostThreadResultType control(void *argument)
{
    static const uint8_t first[8] = {0x10u, 20u, 0x22u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u};
    static const uint8_t consecutive[8] = {0x21u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
    static const uint8_t last[8] = {0x22u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u, 0x12u, 0x34u};
    static const uint8_t response[8] = {3u, 0x7fu, 0x22u, 0x13u, 0u, 0u, 0u, 0u};
    unsigned responses = 0u;
    uint64_t step;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    for (step = 1u; step <= 250u; ++step) {
        uint64_t ticket;
        Os_TickCompletionType completion;
        Ecu_ProtocolRecord failure;
        if ((step == 10u) || (step == 50u) || (step == 250u)) {
            const uint8_t *data = (step == 10u) ? first : ((step == 50u) ? consecutive : last);
            require(Ecu_TargetPostFrame(step, 0x700u, 8u, data, &ticket) == E_OK);
        }
        require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = Os_HostMonotonicMs();
        for (;;) {
            Ecu_OutputRecord output;
            StatusType status = Ecu_TargetTakeOutput(&output);
            if (status == E_OK) {
                if (output.can_id == 0x708u) {
                    if (output.epoch == 250u) {
                        require((output.dlc == 8u) && (memcmp(output.data, response, 8u) == 0));
                        ++responses;
                    } else {
                        require((output.epoch == 10u) && (output.data[0] == 0x30u));
                    }
                }
                require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
            } else {
                require(status == E_OS_NOFUNC);
            }
            status = Os_TargetTickCompletion(ticket, &completion);
            if (status == E_OK) {
                require(completion.epoch == step);
                break;
            }
            require((status == E_OS_NOFUNC) && ((Os_HostMonotonicMs() - started) < 5000u));
            Os_HostSleepMs(1u);
        }
        require(Ecu_TargetTakeProtocolFailure(&failure) == E_OS_NOFUNC);
    }
    require(responses == 1u);
    require(printf("frame_timer deadline=250 accepted=1 no_timeout=1\n") >= 0);
    if (fflush(stdout) != 0) {
        Os_HostExit(93u);
    }
    ShutdownOS(E_OK);
    return 0u;
}

int main(void)
{
    Os_HostHandleType thread;
    require(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(control, NULL);
    require(thread != NULL);
    require(Os_HostClose(thread) != 0);
    StartOS(OSDEFAULTAPPMODE);
    return 94;
}
