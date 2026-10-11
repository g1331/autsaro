#include "Ecu_Target.h"
#include "Can_Host.h"
#include "Os_Host.h"
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#else
#include <unistd.h>
#endif
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
        require(fprintf(stderr, "fault_output ticket=%llu mode=%d\n",
                        (unsigned long long)output->ticket, mode) >= 0);
        require(fflush(stderr) == 0);
        if ((mode == 5) || (mode == 6)) {
            /* Use the exact production writer, with a real failed host stream. */
            int written = write_output(output, state, status, context);
            require(written == 0);
            require(ferror(stdout) != 0);
            require(fprintf(stderr, "physical_%s failed no_receipt ticket=%llu\n",
                            (mode == 5) ? "write" : "flush",
                            (unsigned long long)output->ticket) >= 0);
            return 0;
        }
        if (mode == 1) {
            return 0;
        }
        if (mode == 2) {
            for (;;) {
                Os_HostSleepMs(1000u);
            }
        }
    } else {
        require(printf("fault_commit epoch=%llu sequence=%llu\n",
                       (unsigned long long)state->completed_epoch,
                       (unsigned long long)state->sequence) >= 0);
        require(fflush(stdout) == 0);
    }
    return 1;
}
static Os_HostThreadResult control(void *argument) {
    Ecu_BatchCommand command;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
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
        if ((mode == 5) || (mode == 6)) {
#ifdef _WIN32
            HANDLE reader;
            HANDLE writer;
            int descriptor;
            require(CreatePipe(&reader, &writer, NULL, 0u) != 0);
            require(CloseHandle(reader) != 0);
            descriptor = _open_osfhandle((intptr_t)writer, _O_WRONLY | _O_BINARY);
            require(descriptor >= 0);
            require(_dup2(descriptor, _fileno(stdout)) == 0);
            require(_close(descriptor) == 0);
#else
            require(freopen("/dev/full", "w", stdout) != NULL);
#endif
            require(setvbuf(stdout, NULL, (mode == 5) ? _IONBF : _IOFBF, BUFSIZ) == 0);
        }
        (void)Ecu_HostBatchExecute(&batch, sink, NULL);
        /* A failed/blocked output must close, not report a completed batch. */
        require(0);
    }
    return 0u;
}
int main(int argc, char **argv) {
    Os_HostHandle thread;
    if (argc != 2) {
        return 98;
    }
    if (strcmp(argv[1], "fail") == 0) {
        mode = 1;
    } else if (strcmp(argv[1], "block") == 0) {
        mode = 2;
    } else if (strcmp(argv[1], "write") == 0) {
        mode = 5;
    } else if (strcmp(argv[1], "flush") == 0) {
        mode = 6;
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
        thread = Os_HostSpawnThread(control, NULL);
        require(thread != NULL);
        require(Os_HostClose(thread) != 0);
    }
    StartOS(1u);
    return 94;
}
