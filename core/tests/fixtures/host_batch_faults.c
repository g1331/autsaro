#include "Ecu_Target.h"
#include "Can.h"
#include "Os_Windows.h"
#include <stdio.h>
#include <string.h>

static Ecu_HostBatch batch;
static int mode;
static void require(int accepted);
#ifdef ECU_TARGET_TESTS
static unsigned queued;
int Ecu_TargetTestFailStage(unsigned stage) {
    if ((mode == 4) && (stage == 8u)) {
        static const uint8_t bytes[8] = {0u};
        for (queued = 0u; queued < 256u; ++queued) {
            require(Can_TransmitPdu(1u, 0x321u, 4u, bytes) == ECU_OK);
        }
        require(printf("output_capacity accepted=256 next=257\n") >= 0);
        require(fflush(stdout) == 0);
        (void)Can_TransmitPdu(1u, 0x321u, 4u, bytes);
        require(0);
    }
    return 0;
}
void Ecu_TargetTestShutdown(StatusType reason) {
    require(printf("output_close queued=%u reason=%u\n", queued, reason) >= 0);
}
#endif

static void require(int accepted) {
    if (accepted == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}
static int sink(const Ecu_OutputRecord *output, const Ecu_HostBatch *state, StatusType status,
                void *context) {
    (void)status;
    (void)context;
    if (output != NULL) {
        require(printf("fault_output ticket=%llu mode=%d\n", (unsigned long long)output->ticket,
                       mode) >= 0);
        require(fflush(stdout) == 0);
        if (mode == 1) {
            return 0;
        }
        if (mode == 2) {
            Sleep(INFINITE);
        }
    } else {
        require(printf("fault_commit epoch=%llu sequence=%llu\n",
                       (unsigned long long)state->completed_epoch,
                       (unsigned long long)state->sequence) >= 0);
        require(fflush(stdout) == 0);
    }
    return 1;
}
static DWORD WINAPI control(void *argument) {
    Ecu_BatchCommand command;
    DWORD started = GetTickCount();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((GetTickCount() - started) < 5000u);
        Sleep(1u);
    }
    Ecu_HostBatchInitialize(&batch);
    if (mode == 3) {
        batch.sequence = UINT64_MAX;
        require(Ecu_HostBatchParse(&batch, "BEGIN 0", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OS_LIMIT);
        require(batch.completed_epoch == UINT64_C(0));
        Ecu_HostBatchInitialize(&batch);
        batch.sequence = UINT64_MAX - UINT64_C(1);
        require(Ecu_HostBatchParse(&batch, "BEGIN 1", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        require(Ecu_HostBatchExecute(&batch, sink, NULL) == E_OS_LIMIT);
        {
            Os_TickCompletion initial;
            require(Os_TargetTickCompletion(UINT64_C(0), &initial) == E_OK);
            require(initial.epoch == UINT64_C(0));
        }
        Ecu_HostBatchInitialize(&batch);
        batch.sequence = UINT64_MAX - UINT64_C(1);
        require(Ecu_HostBatchParse(&batch, "BEGIN 0", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        require(Ecu_HostBatchExecute(&batch, sink, NULL) == E_OK);
        require(batch.sequence == UINT64_MAX);
        require(printf("sequence_limit before_state_change=pass final_identity=pass\n") >= 0);
        ShutdownOS(E_OK);
    } else {
        require(Ecu_HostBatchParse(&batch, "BEGIN 10", 8u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        (void)Ecu_HostBatchExecute(&batch, sink, NULL);
        /* A failed/blocked output must close, not report a completed batch. */
        require(0);
    }
    return 0u;
}
int main(int argc, char **argv) {
    HANDLE thread;
    if (argc != 2) {
        return 98;
    }
    if (strcmp(argv[1], "fail") == 0) {
        mode = 1;
    } else if (strcmp(argv[1], "block") == 0) {
        mode = 2;
    } else if (strcmp(argv[1], "sequence") == 0) {
        mode = 3;
#ifdef ECU_TARGET_TESTS
    } else if (strcmp(argv[1], "overflow") == 0) {
        mode = 4;
#endif
    } else {
        return 98;
    }
    require(Ecu_TargetPrepare() == E_OK);
    if (mode != 4) {
        thread = CreateThread(NULL, 262144u, control, NULL, 0u, NULL);
        require(thread != NULL);
        require(CloseHandle(thread) != 0);
    }
    StartOS(1u);
    return 94;
}
