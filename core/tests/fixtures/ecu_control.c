/* Independent consumer: fixed reference vectors are not generated expectations. */
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Rte_Os_Type.h"
#include "Os_Host.h"
#include "Rte_EchoApplication.h"
#include <stdio.h>
#include <string.h>

static void require_at(int accepted, unsigned line) {
    if (accepted == 0) {
        (void)fprintf(stderr, "ECU consumer assertion failed: line=%u\n", line);
        ShutdownOS(E_OS_STATE);
    }
}
#define require(accepted) require_at((accepted), __LINE__)

static Os_HostThreadResult control(void *argument) {
    static const uint8_t signal[8] = {0x78u, 0x56u, 0x34u, 0x12u, 0u, 0u, 0u, 0u};
    static const uint8_t request[8] = {3u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    static const uint8_t initial[8] = {7u, 0x62u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    static const uint8_t committed[8] = {7u, 0x62u, 0x12u, 0x34u, 0x12u, 0x34u, 0x56u, 0x78u};
    uint64_t step;
    unsigned signal_outputs = 0u;
    unsigned diagnostic_outputs = 0u;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    {
        uint64_t untouched = UINT64_C(42);
        require(Ecu_TargetPostFrame(UINT64_C(2), 0x320u, 4u, signal, &untouched) == E_OS_VALUE);
        require(Ecu_TargetPostFrame(UINT64_MAX, 0x320u, 4u, signal, &untouched) == E_OS_VALUE);
        require((untouched == UINT64_C(42)) && (Ecu_TargetState() == ECU_TARGET_READY));
    }
    for (step = UINT64_C(1); step <= UINT64_C(20); ++step) {
        uint64_t ticket;
        Os_TickCompletion completion;
        if (step == UINT64_C(1)) {
            require(Ecu_TargetPostFrame(step, 0x320u, 4u, signal, &ticket) == E_OK);
            {
                uint64_t untouched = UINT64_C(42);
                require(Ecu_TargetPostFrame(UINT64_C(0), 0x320u, 4u, signal, &untouched) ==
                        E_OS_VALUE);
                require(untouched == UINT64_C(42));
            }
        } else if ((step == UINT64_C(2)) || (step == UINT64_C(11))) {
            require(Ecu_TargetPostFrame(step, 0x700u, 8u, request, &ticket) == E_OK);
        }
        require(Os_TargetAdvanceOneTick(step, &ticket) == E_OK);
        started = Os_HostMonotonicMs();
        for (;;) {
            Ecu_OutputRecord output;
            StatusType status = Ecu_TargetTakeOutput(&output);
            if (status == E_OK) {
                uint64_t untouched = UINT64_C(42);
                require(Ecu_TargetPostFrame(step, 0x320u, 4u, signal, &untouched) == E_OS_STATE);
                require(untouched == UINT64_C(42));
                printf("independent_output epoch=%llu id=%u pdu=%u dlc=%u\n",
                       (unsigned long long)output.epoch, output.can_id, output.pdu, output.dlc);
                if (output.can_id == 0x321u) {
                    require((output.dlc == 4u) && (output.pdu == 1u) &&
                            ((output.epoch == UINT64_C(10)) || (output.epoch == UINT64_C(20))) &&
                            (memcmp(output.data, signal, 4u) == 0));
                    ++signal_outputs;
                } else {
                    const uint8_t *expected = (diagnostic_outputs == 0u) ? initial : committed;
                    require((output.can_id == 0x708u) && (output.dlc == 8u) && (output.pdu == 3u) &&
                            (memcmp(output.data, expected, 8u) == 0));
                    ++diagnostic_outputs;
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
    }
    require((signal_outputs == 2u) && (diagnostic_outputs == 2u));
    printf("independent_control signal=2 default_session_did=2 completed=20\n");
    ShutdownOS(E_OK);
    return 0u;
}

int main(void) {
    CounterType counter = UINT32_MAX;
    TimeInMicrosecondsType microseconds = UINT64_MAX;
    if (sizeof(counter) != 4u || sizeof(microseconds) != 8u || counter != UINT32_MAX ||
        microseconds != UINT64_MAX || TotalNumberOfCores != 1u) {
        return 48;
    }
    const Os_TimeConfig *time = Ecu_OsConfig.time;
    if (time == NULL || time->counter_count != 1u || time->counters == NULL ||
        time->system_counter != 0u || time->counters[0u].id != 0u ||
        time->counters[0u].maximum != UINT64_C(65535) || time->counters[0u].ticks_per_base != 1u ||
        time->counters[0u].minimum_cycle != 1u || OSMAXALLOWEDVALUE != time->counters[0u].maximum ||
        OSTICKSPERBASE != time->counters[0u].ticks_per_base ||
        OSMINCYCLE != time->counters[0u].minimum_cycle || OSTICKDURATION != UINT64_C(1000000)) {
        return 49;
    }
    Os_HostHandle thread;
    require(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(control, NULL);
    require(thread != NULL);
    require(Os_HostClose(thread) != 0);
    StartOS(1u);
    return 94;
}
