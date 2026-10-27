/** Real owner/driver/output/confirmation consumer with a positive-size memcpy oracle. */
#include "Ecu_Target.h"
#include "Can_Host.h"
#include "Os_Host.h"
#include <stdio.h>
#include <string.h>

static void require_at(int accepted, unsigned line)
{
    if (accepted == 0) {
        (void)fprintf(stderr, "zero-DLC expectation failed at line %u\n", line);
        Os_HostExit(93u);
    }
}
#define require(condition) require_at((condition), __LINE__)

void *__real_memcpy(void *destination, const void *source, size_t length);
static void require_positive_copy(size_t length)
{
    if (length == 0u) {
        (void)fputs("zero_length_memcpy\n", stderr);
        Os_HostExit(92u);
    }
}
void *__wrap_memcpy(void *destination, const void *source, size_t length)
{
    require_positive_copy(length);
    return __real_memcpy(destination, source, length);
}
#ifdef _WIN32
void *(*__wrap___imp_memcpy)(void *destination, const void *source, size_t length) = __wrap_memcpy;
#else
void *__real___memcpy_chk(void *destination, const void *source, size_t length, size_t capacity);
void *__wrap___memcpy_chk(void *destination, const void *source, size_t length, size_t capacity)
{
    require_positive_copy(length);
    return __real___memcpy_chk(destination, source, length, capacity);
}
#endif

static uint8_t payload_length(uint64_t epoch)
{
    return (uint8_t)((epoch == 1u) ? 0u : ((epoch == 2u) ? 1u : 8u));
}
int Ecu_TargetTestFailStage(unsigned stage)
{
    const uint64_t epoch = Ecu_TargetNow();
    if ((stage == 9u) && (epoch >= 1u) && (epoch <= 3u)) {
        static const uint8_t bytes[8] = {1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u};
        require(Can_TransmitPdu(1u, 0x321u, payload_length(epoch), bytes) == ECU_OK);
    }
    return 0;
}
void Ecu_TargetTestRetire(unsigned stage) { (void)stage; }
void Ecu_TargetTestShutdown(StatusType reason)
{
    if (reason != E_OK) {
        (void)fprintf(stderr, "zero-DLC unexpected shutdown reason=%u\n", (unsigned)reason);
    }
    require(reason == E_OK);
    (void)puts("zero_dlc closed reason=0");
    require(fflush(stdout) == 0);
}
static Os_HostThreadResultType control(void *argument)
{
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    for (uint64_t epoch = 1u; epoch <= 3u; ++epoch) {
        uint64_t ticket;
        Ecu_OutputRecord output;
        Os_TickCompletionType completion;
        StatusType status;
        started = Os_HostMonotonicMs();
        require(Os_TargetAdvanceOneTick(epoch, &ticket) == E_OK);
        do {
            status = Ecu_TargetTakeOutput(&output);
            require((status == E_OK) || (status == E_OS_NOFUNC));
            require((Os_HostMonotonicMs() - started) < 5000u);
            if (status == E_OS_NOFUNC) {
                Os_HostSleepMs(1u);
            }
        } while (status == E_OS_NOFUNC);
        require(output.epoch == epoch && output.can_id == 0x321u &&
                output.dlc == payload_length(epoch));
        for (unsigned i = 0u; i < 8u; ++i) {
            require(output.data[i] == ((i < output.dlc) ? (uint8_t)(i + 1u) : 0u));
        }
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
        while (Os_TargetTickCompletion(ticket, &completion) != E_OK) {
            require((Os_HostMonotonicMs() - started) < 5000u);
            Os_HostSleepMs(1u);
        }
        require(completion.epoch == epoch);
        (void)printf("zero_dlc epoch=%llu length=%u confirmed=1\n",
                     (unsigned long long)epoch,
                     (unsigned)output.dlc);
    }
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv)
{
    Os_HostHandleType thread;
#ifndef _WIN32
    if (argc == 2 && strcmp(argv[1], "fortify") == 0) {
        unsigned char destination[8] = {0u};
        const unsigned char source[9] = {0u};
        volatile size_t length = sizeof(source);
        (void)__wrap___memcpy_chk(destination, source, length, sizeof(destination));
        return 91;
    }
#else
    (void)argc;
    (void)argv;
#endif
    require(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(control, NULL);
    require(thread != NULL && Os_HostClose(thread) != 0);
    StartOS(OSDEFAULTAPPMODE);
    return 94;
}
